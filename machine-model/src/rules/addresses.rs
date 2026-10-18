//! 位址規則：R8、R18、R19、R24。
//!
//! v2 沒有 R20：Tag 沒有 `write_permission`。「操作腳不得綁唯讀位址類別」是 U13（Plan 2）。

use std::collections::{BTreeMap, BTreeSet};

use crate::address::{self, Address};
use crate::diag::Diagnostic;
use crate::family::{ClassProfile, Width};
use crate::schema::{MachineModel, Plc, Tag, split_id};
use crate::types::DataType;

/// 一個已通過 R18、R19 的 tag 在某個位址類別上佔用的空間。
enum Slot<'a> {
    /// 半開區間 `[start, end)`，單位是該類別的位址（位元類為位元，字組類為字組）。
    Span { start: u32, end: u32, tag: &'a Tag },
    /// 字組類別中的單一位元（`R10.3`）。
    Bit { word: u32, bit: u8, tag: &'a Tag },
}

pub(super) fn check(m: &MachineModel, out: &mut Vec<Diagnostic>) {
    check_tags(&m.machine.plcs, &m.tags, out);
}

fn check_tags(plcs: &[Plc], tags: &[Tag], out: &mut Vec<Diagnostic>) {
    let plcs: BTreeMap<&str, &Plc> = plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    // (plc id, 類別名稱) → 佔用
    let mut placed: BTreeMap<(&str, &'static str), Vec<Slot<'_>>> = BTreeMap::new();

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
        let Some(slot) = make_slot(t, class, &addr, &ty, &loc, out) else {
            continue;
        };
        placed
            .entry((plc_id, addr.class.name))
            .or_default()
            .push(slot);
    }

    for ((plc_id, class_name), slots) in &placed {
        check_overlap(plc_id, class_name, slots, out);
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

fn type_fits(class: &ClassProfile, addr: &Address, ty: &DataType) -> Result<(), String> {
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

fn make_slot<'a>(
    t: &'a Tag,
    class: &ClassProfile,
    addr: &Address,
    ty: &DataType,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<Slot<'a>> {
    if let Some(bit) = addr.bit {
        return Some(Slot::Bit {
            word: addr.number,
            bit,
            tag: t,
        });
    }
    let span = match class.width {
        Width::Bit => ty.len,
        Width::Word => ty.words(),
    };
    let span = u32::try_from(span).unwrap_or(u32::MAX);
    let end = addr.number.saturating_add(span);
    if end > class.max.saturating_add(1) {
        out.push(Diagnostic::error(
            "R18",
            loc,
            format!(
                "{} 類別從 {} 起佔用 {span} 個位址，超過上限 {}",
                class.name, addr.number, class.max
            ),
        ));
        return None;
    }
    Some(Slot::Span {
        start: addr.number,
        end,
        tag: t,
    })
}

fn check_overlap(plc_id: &str, class_name: &str, slots: &[Slot<'_>], out: &mut Vec<Diagnostic>) {
    let mut spans: Vec<(u32, u32, &Tag)> = slots
        .iter()
        .filter_map(|s| match s {
            Slot::Span { start, end, tag } => Some((*start, *end, *tag)),
            Slot::Bit { .. } => None,
        })
        .collect();
    spans.sort_by_key(|(start, _, tag)| (*start, tag.id.as_str()));

    let mut holder: Option<(u32, &Tag)> = None; // (目前最遠的 end, 持有者)
    for (start, end, tag) in &spans {
        if let Some((max_end, owner)) = holder {
            if *start < max_end {
                out.push(overlap(plc_id, class_name, tag, owner));
            }
        }
        if holder.is_none_or(|(max_end, _)| *end > max_end) {
            holder = Some((*end, tag));
        }
    }

    let mut seen_bits: BTreeMap<(u32, u8), &Tag> = BTreeMap::new();
    let mut bit_slots: Vec<(u32, u8, &Tag)> = slots
        .iter()
        .filter_map(|s| match s {
            Slot::Bit { word, bit, tag } => Some((*word, *bit, *tag)),
            Slot::Span { .. } => None,
        })
        .collect();
    bit_slots.sort_by_key(|(w, b, tag)| (*w, *b, tag.id.as_str()));
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    for (word, bit, tag) in bit_slots {
        if let Some(other) = seen_bits.get(&(word, bit)) {
            out.push(overlap(plc_id, class_name, tag, other));
            continue;
        }
        seen_bits.insert((word, bit), tag);
        if let Some((_, _, owner)) = spans.iter().find(|(s, e, _)| (*s..*e).contains(&word)) {
            if reported.insert(tag.id.as_str()) {
                out.push(overlap(plc_id, class_name, tag, owner));
            }
        }
    }
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
