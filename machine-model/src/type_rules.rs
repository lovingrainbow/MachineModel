//! 類型層規則：U1、U12、U15、U18、U19（U17 在 `catalog`）。輸入是整個目錄，
//! 因為 U18 需要跨類型。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::diag::Diagnostic;
use crate::naming::{self, Prefix};
use crate::schema::Scalar;
use crate::types::DataType;
use crate::unit_type::UnitType;

pub(crate) fn check(types: &BTreeMap<String, UnitType>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for t in types.values() {
        check_unique_names(t, &mut out);
        check_par_ranges(t, &mut out);
        check_error_pins(t, &mut out);
        check_function_spelling(t, &mut out);
    }
    check_parts(types, &mut out);
    out
}

fn check_unique_names(t: &UnitType, out: &mut Vec<Diagnostic>) {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    let names = t
        .field
        .input
        .0
        .keys()
        .chain(t.field.output.0.keys())
        .chain(t.ctrl.0.keys())
        .chain(t.parts.0.keys());
    for name in names {
        *count.entry(name.as_str()).or_default() += 1;
    }
    for (name, n) in count {
        if n > 1 {
            out.push(Diagnostic::error(
                "U1",
                format!("types[{}].{name}", t.id),
                format!("名稱 `{name}` 在 field、ctrl、parts 內出現 {n} 次，必須全體唯一"),
            ));
        }
    }
}

#[allow(clippy::cast_precision_loss)] // 型別界限轉 f64 只用於比較
fn check_par_ranges(t: &UnitType, out: &mut Vec<Diagnostic>) {
    for (name, pin) in &t.ctrl.0 {
        let Some((Prefix::Par, _)) = naming::split(name) else {
            continue;
        };
        let (Some(min), Some(max), Some(default)) = (pin.min, pin.max, pin.default.as_ref()) else {
            continue; // 缺屬性是 SC3
        };
        let loc = format!("types[{}].ctrl[{name}]", t.id);
        let value = match default {
            Scalar::Int(i) => *i as f64,
            Scalar::Float(f) => *f,
            _ => {
                out.push(Diagnostic::error("U12", &loc, "par_ 的 default 必須是數字"));
                continue;
            }
        };
        // 用否定寫法，NaN 也會被擋下
        if !(min <= value && value <= max) {
            out.push(Diagnostic::error(
                "U12",
                &loc,
                format!("必須滿足 min ≤ default ≤ max（min={min}、default={value}、max={max}）"),
            ));
        }
        let bounds = pin
            .type_text(Prefix::Par)
            .and_then(|text| DataType::parse(text).ok())
            .and_then(|dt| dt.int_bounds());
        if let Some((lo, hi)) = bounds {
            for (what, v) in [("min", min), ("max", max), ("default", value)] {
                if v < lo as f64 || v > hi as f64 {
                    out.push(Diagnostic::error(
                        "U12",
                        &loc,
                        format!(
                            "{what}={v} 超出型別 `{}` 的範圍",
                            pin.ty.as_deref().unwrap_or("")
                        ),
                    ));
                }
            }
        }
    }
}

fn check_error_pins(t: &UnitType, out: &mut Vec<Diagnostic>) {
    for (name, pin) in &t.ctrl.0 {
        let Some((Prefix::Error, _)) = naming::split(name) else {
            continue;
        };
        let is_bool = pin
            .type_text(Prefix::Error)
            .and_then(|text| DataType::parse(text).ok())
            .is_some_and(|dt| dt.is_bool() && !dt.array);
        if !is_bool {
            out.push(Diagnostic::error(
                "U19",
                format!("types[{}].ctrl[{name}]", t.id),
                "error_ 接腳的型別必須是 bool",
            ));
        }
    }
}

/// 只差一個字元（取代、插入或刪除），且兩者不同。
fn within_one_edit(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a == b {
        return false;
    }
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    if long.len() - short.len() > 1 {
        return false;
    }
    let i = short
        .iter()
        .zip(long.iter())
        .take_while(|(x, y)| x == y)
        .count();
    if short.len() == long.len() {
        short[i + 1..] == long[i + 1..]
    } else {
        short[i..] == long[i + 1..]
    }
}

fn check_function_spelling(t: &UnitType, out: &mut Vec<Diagnostic>) {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for name in t.ctrl.0.keys() {
        if let Some((_, rest)) = naming::split(name) {
            *count.entry(naming::function_of(rest)).or_default() += 1;
        }
    }
    for (&f, &n) in &count {
        if n != 1 {
            continue;
        }
        let neighbour = count
            .iter()
            .find(|&(&g, &m)| m >= 2 && within_one_edit(f, g));
        if let Some((&g, &m)) = neighbour {
            out.push(Diagnostic::warning(
                "U15",
                format!("types[{}]", t.id),
                format!("功能名 `{f}` 只出現一次，與出現 {m} 次的 `{g}` 只差一個字元，是否拼錯？"),
            ));
        }
    }
}

fn check_parts(types: &BTreeMap<String, UnitType>, out: &mut Vec<Diagnostic>) {
    for t in types.values() {
        for (part, ty) in &t.parts.0 {
            if !types.contains_key(ty) {
                out.push(Diagnostic::error(
                    "U18",
                    format!("types[{}].parts[{part}]", t.id),
                    format!("part 的類型 `{ty}` 不在目錄內"),
                ));
            }
        }
    }
    // 環：從每個類型出發，看能不能經由 parts 回到自己。
    for t in types.values() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut queue: VecDeque<&str> = t.parts.0.values().map(String::as_str).collect();
        let mut cyclic = false;
        while let Some(id) = queue.pop_front() {
            if id == t.id {
                cyclic = true;
                break;
            }
            if !seen.insert(id) {
                continue;
            }
            if let Some(next) = types.get(id) {
                queue.extend(next.parts.0.values().map(String::as_str));
            }
        }
        if cyclic {
            out.push(Diagnostic::error(
                "U18",
                format!("types[{}]", t.id),
                "parts 關係成環（含自我引用）",
            ));
        }
    }
}
