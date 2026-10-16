//! 單元圖規則：U2、U3、U5、U7、U8、U9。全部用迭代演算法（Kahn、BFS），不遞迴。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::schema::{BindRef, MACHINE_UNIT, MachineModel, UnitInstance, classify_bind};
use crate::unit_type::PinRef;

pub(super) fn check(m: &MachineModel, catalog: &UnitCatalog, out: &mut Vec<Diagnostic>) {
    let by_id: BTreeMap<&str, &UnitInstance> = m.units.iter().map(|u| (u.id.as_str(), u)).collect();
    // 只含 id 存在的邊：父 → 子（可重複，代表同一父層以多個 part 綁同一子層）
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut parents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for u in &m.units {
        let loc = format!("units[{}]", u.id);
        let Some(ty) = catalog.get(&u.ty) else {
            out.push(Diagnostic::error(
                "U2",
                &loc,
                format!("類型 `{}` 不在類型目錄內", u.ty),
            ));
            continue;
        };
        if ty.version != u.type_version {
            out.push(Diagnostic::error(
                "U2",
                &loc,
                format!(
                    "type_version {} 與類型目錄中 `{}` 的 version {} 不符；不會靜默採用，請確認要用哪一版",
                    u.type_version, u.ty, ty.version
                ),
            ));
        }
        for (key, value) in &u.bind.0 {
            match ty.lookup(key) {
                None => out.push(Diagnostic::error(
                    "U3",
                    &loc,
                    format!("bind 的鍵 `{key}` 不是類型 `{}` 的接腳或 part", u.ty),
                )),
                Some(PinRef::Part(part_ty)) => {
                    let Some(value) = value else {
                        continue; // null：刻意不接
                    };
                    let Some(BindRef::Unit(child_id)) = classify_bind(value) else {
                        out.push(Diagnostic::error(
                            "U5",
                            &loc,
                            format!("part `{key}` 必須綁單元 id（不含 `.`），而不是 `{value}`"),
                        ));
                        continue;
                    };
                    match by_id.get(child_id) {
                        None => out.push(Diagnostic::error(
                            "U5",
                            &loc,
                            format!("part `{key}` 綁定的單元 `{child_id}` 不存在"),
                        )),
                        Some(child) => {
                            // 若子層類型本身已 U2（不在目錄內），跳過 U5 的類型比對以避免連鎖雜訊（邊照加，供 U8 可達性與環分析使用）
                            if catalog.get(&child.ty).is_some() && child.ty != part_ty {
                                out.push(Diagnostic::error(
                                    "U5",
                                    &loc,
                                    format!(
                                        "part `{key}` 宣告的類型是 `{part_ty}`，但單元 `{child_id}` 的類型是 `{}`",
                                        child.ty
                                    ),
                                ));
                            }
                            children
                                .entry(u.id.as_str())
                                .or_default()
                                .push(child.id.as_str());
                            parents
                                .entry(child.id.as_str())
                                .or_default()
                                .push(u.id.as_str());
                        }
                    }
                }
                Some(_) => {} // field／ctrl 腳由接腳規則處理
            }
        }
    }

    for (child, ps) in &parents {
        if ps.len() > 1 {
            out.push(Diagnostic::error(
                "U7",
                format!("units[{child}]"),
                format!(
                    "被 {} 個 part 綁定（{}）；每個單元只能有一個父層",
                    ps.len(),
                    ps.join("、")
                ),
            ));
        }
    }

    if !by_id.contains_key(MACHINE_UNIT) {
        out.push(Diagnostic::error(
            "U9",
            "units",
            "缺少根單元 `machine`（machine.yaml 必須宣告 id 為 machine 的單元）",
        ));
    } else if let Some(ps) = parents.get(MACHINE_UNIT) {
        out.push(Diagnostic::error(
            "U9",
            format!("units[{MACHINE_UNIT}]"),
            format!("`machine` 必須是根，不得被 {} 綁定", ps.join("、")),
        ));
    }

    let cyclic = cycle_members(&by_id, &children);
    for id in &cyclic {
        out.push(Diagnostic::error(
            "U8",
            format!("units[{id}]"),
            "參與包含關係的環，或位於環之下",
        ));
    }

    if by_id.contains_key(MACHINE_UNIT) {
        let reached = reachable_from(MACHINE_UNIT, &children);
        for id in by_id.keys() {
            if !reached.contains(id) && !cyclic.contains(id) {
                out.push(Diagnostic::error(
                    "U8",
                    format!("units[{id}]"),
                    "無法由 `machine` 到達（沒有被任何父層綁定）",
                ));
            }
        }
    }
}

/// Kahn 拓撲排序：移除所有能被剝掉的節點，剩下的就是環上或環之下的節點。
fn cycle_members<'a>(
    by_id: &BTreeMap<&'a str, &UnitInstance>,
    children: &BTreeMap<&'a str, Vec<&'a str>>,
) -> BTreeSet<&'a str> {
    let mut indegree: BTreeMap<&str, usize> = by_id.keys().map(|id| (*id, 0)).collect();
    for cs in children.values() {
        for c in cs {
            if let Some(n) = indegree.get_mut(c) {
                *n += 1;
            }
        }
    }
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter(|&(_, &n)| n == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut removed: BTreeSet<&str> = BTreeSet::new();
    while let Some(id) = queue.pop_front() {
        removed.insert(id);
        for c in children.get(id).into_iter().flatten() {
            if let Some(n) = indegree.get_mut(c) {
                *n -= 1;
                if *n == 0 {
                    queue.push_back(c);
                }
            }
        }
    }
    by_id
        .keys()
        .filter(|id| !removed.contains(*id))
        .copied()
        .collect()
}

fn reachable_from<'a>(
    root: &'a str,
    children: &BTreeMap<&'a str, Vec<&'a str>>,
) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue = VecDeque::from([root]);
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        queue.extend(children.get(id).into_iter().flatten().copied());
    }
    seen
}
