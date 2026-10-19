//! 位址規則：R8、R18、R19、R24。
//!
//! v2 沒有 R20：Tag 沒有 `write_permission`。「操作腳不得綁唯讀位址類別」是 U13（Plan 2）。
//!
//! 佔用範圍（[`occupancy`]）與重疊判定（[`collisions`]）同時供服務點位使用（`writable` 模組，DS9）。

use std::collections::{BTreeMap, BTreeSet};

use crate::address::{self, Address};
use crate::diag::Diagnostic;
use crate::family::{ClassProfile, Width};
use crate::schema::{MachineModel, Plc, Tag, split_id};
use crate::types::DataType;

/// 一個點位在某個位址類別上佔用的空間（R8 與服務點位共用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Occupancy {
    /// 半開區間 `[start, end)`，單位是該類別的位址（位元類為位元，字組類為字組）。
    Span { start: u32, end: u32 },
    /// 字組類別中的單一位元（`R10.3`）。
    Bit { word: u32, bit: u8 },
}

pub(super) fn check(m: &MachineModel, out: &mut Vec<Diagnostic>) {
    check_tags(&m.machine.plcs, &m.tags, out);
}

fn check_tags(plcs: &[Plc], tags: &[Tag], out: &mut Vec<Diagnostic>) {
    let plcs: BTreeMap<&str, &Plc> = plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    // (plc id, 類別名稱) → 佔用
    let mut placed: BTreeMap<(&str, &'static str), Vec<(Occupancy, &Tag)>> = BTreeMap::new();

    for t in tags {
        let Some((plc_id, _)) = split_id(&t.id) else {
            continue;
        };
        let Some(plc) = plcs.get(plc_id) else {
            continue;
        };
        if t.address.is_empty() {
            continue;
        }
        let loc = format!("tags[{}]", t.id);
        let addr = match address::parse(plc.family, &t.address) {
            Ok(a) => a,
            Err(e) => {
                out.push(Diagnostic::error(
                    "R18",
                    &loc,
                    format!("address `{}`：{e}", t.address),
                ));
                continue;
            }
        };
        let class = addr.class;

        let Ok(ty) = DataType::parse(&t.ty) else {
            continue; // R2 已回報
        };
        check_polarity(t, &ty, class, &loc, out);
        // 非空語法沒有位址類別與範圍，不做型別、重疊檢查（見 family.rs 的可信度說明）
        if class.opaque {
            continue;
        }
        if let Err(msg) = type_fits(class, &addr, &ty) {
            out.push(Diagnostic::error("R19", &loc, msg));
            continue;
        }
        let occ = match occupancy(class, &addr, &ty) {
            Ok(o) => o,
            Err(msg) => {
                out.push(Diagnostic::error("R18", &loc, msg));
                continue;
            }
        };
        placed
            .entry((plc_id, class.name))
            .or_default()
            .push((occ, t));
    }

    for ((plc_id, class_name), slots) in &placed {
        let items: Vec<(&str, Occupancy)> =
            slots.iter().map(|(o, t)| (t.id.as_str(), *o)).collect();
        for (later, earlier) in collisions(&items) {
            out.push(overlap(
                plc_id,
                class_name,
                slots[later].1,
                slots[earlier].1,
            ));
        }
    }
}

fn check_polarity(
    t: &Tag,
    ty: &DataType,
    class: &ClassProfile,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) {
    if t.polarity.is_some() && !(ty.is_bool() && !ty.array && class.read_only) {
        out.push(Diagnostic::error(
            "R24",
            loc,
            "polarity 只能用在 bool 型別的輸入點（唯讀輸入位址類別）",
        ));
    }
}

/// 位址的型別是否與類別相容（R19）。
pub(crate) fn type_fits(class: &ClassProfile, addr: &Address, ty: &DataType) -> Result<(), String> {
    match class.width {
        Width::Bit => {
            if ty.is_bool() {
                Ok(())
            } else {
                Err(format!("{} 類別是位元位址，只能使用 bool", class.name))
            }
        }
        Width::Word => match (ty.is_bool(), addr.bit) {
            (true, None) => Err(format!(
                "字組類別 {} 上的 bool 必須使用位元後綴（例如 {}10.3）",
                class.name, class.name
            )),
            (true, Some(_)) if ty.array => Err("位元位址不支援 bool 陣列".to_owned()),
            (false, Some(_)) => Err("位元後綴只能搭配 bool".to_owned()),
            _ => Ok(()),
        },
    }
}

/// 依位址與型別算出佔用空間；超過類別上限時回傳錯誤訊息（R18）。
pub(crate) fn occupancy(
    class: &ClassProfile,
    addr: &Address,
    ty: &DataType,
) -> Result<Occupancy, String> {
    if let Some(bit) = addr.bit {
        return Ok(Occupancy::Bit {
            word: addr.number,
            bit,
        });
    }
    let span = match class.width {
        Width::Bit => ty.len,
        Width::Word => ty.words(),
    };
    let span = u32::try_from(span).unwrap_or(u32::MAX);
    let end = addr.number.saturating_add(span);
    if end > class.max.saturating_add(1) {
        return Err(format!(
            "{} 類別從 {} 起佔用 {span} 個位址，超過上限 {}",
            class.name, addr.number, class.max
        ));
    }
    Ok(Occupancy::Span {
        start: addr.number,
        end,
    })
}

/// 同一位址類別內的佔用重疊配對（R8 與服務點位共用）。
///
/// `items` 為 `(點位 id, 佔用)`。回傳 `(後者的索引, 先者的索引)`，順序與 R8 的診斷順序一致。
pub(crate) fn collisions(items: &[(&str, Occupancy)]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();

    let mut spans: Vec<(u32, u32, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, (_, occ))| match *occ {
            Occupancy::Span { start, end } => Some((start, end, i)),
            Occupancy::Bit { .. } => None,
        })
        .collect();
    spans.sort_by_key(|(start, _, i)| (*start, items[*i].0));

    let mut holder: Option<(u32, usize)> = None; // (目前最遠的 end, 持有者)
    for &(start, end, i) in &spans {
        if let Some((max_end, owner)) = holder {
            if start < max_end {
                pairs.push((i, owner));
            }
        }
        if holder.is_none_or(|(max_end, _)| end > max_end) {
            holder = Some((end, i));
        }
    }

    let mut bit_slots: Vec<(u32, u8, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, (_, occ))| match *occ {
            Occupancy::Bit { word, bit } => Some((word, bit, i)),
            Occupancy::Span { .. } => None,
        })
        .collect();
    bit_slots.sort_by_key(|(w, b, i)| (*w, *b, items[*i].0));

    let mut seen_bits: BTreeMap<(u32, u8), usize> = BTreeMap::new();
    let mut reported: BTreeSet<usize> = BTreeSet::new();
    for (word, bit, i) in bit_slots {
        if let Some(&other) = seen_bits.get(&(word, bit)) {
            pairs.push((i, other));
            continue;
        }
        seen_bits.insert((word, bit), i);
        if let Some(&(_, _, owner)) = spans.iter().find(|(s, e, _)| (*s..*e).contains(&word)) {
            if reported.insert(i) {
                pairs.push((i, owner));
            }
        }
    }
    pairs
}

fn overlap(plc_id: &str, class_name: &str, tag: &Tag, other: &Tag) -> Diagnostic {
    Diagnostic::error(
        "R8",
        format!("tags[{}]", tag.id),
        format!(
            "{plc_id} 的 {class_name} 類別位址 `{}` 與 `{}`（{}）重疊",
            tag.address, other.id, other.address
        ),
    )
}
