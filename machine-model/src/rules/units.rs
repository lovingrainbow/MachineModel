//! 單元實例規則：R25（variables 不得有 address）、R26（單元實例 id）、
//! R28（`type`、`type_version` 與 `bind` 的語法）。
//!
//! 引用是否存在、單親、無環、類型符合性需要類型目錄或整張圖，屬 U 規則（Plan 2）。

use std::collections::BTreeSet;

use crate::diag::Diagnostic;
use crate::schema::{
    BindRef, MachineModel, UNASSIGNED_UNIT, classify_bind, is_valid_name, split_id,
};

pub(super) fn check(m: &MachineModel, plc_ids: &BTreeSet<&str>, out: &mut Vec<Diagnostic>) {
    for v in &m.variables {
        if v.address.is_some() {
            out.push(Diagnostic::error(
                "R25",
                format!("variables[{}]", v.id),
                "variables 不得有 address；有位址的點位請放在 tags",
            ));
        }
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for u in &m.units {
        let loc = format!("units[{}]", u.id);
        if u.id == UNASSIGNED_UNIT {
            out.push(Diagnostic::error(
                "R26",
                &loc,
                format!("`{}` 是保留的單元 id，不得宣告", u.id),
            ));
        } else if !is_valid_name(&u.id) {
            out.push(Diagnostic::error("R26", &loc, "單元 id 只能使用英數與底線"));
        }
        if !seen.insert(u.id.as_str()) {
            out.push(Diagnostic::error("R26", &loc, "單元 id 重複"));
        }
        if !is_valid_name(&u.ty) {
            out.push(Diagnostic::error(
                "R28",
                &loc,
                format!("type `{}` 只能使用英數與底線（Unit 類型檔 id）", u.ty),
            ));
        }
        if u.type_version == 0 {
            out.push(Diagnostic::error(
                "R28",
                &loc,
                "type_version 必須是正整數（與類型檔的 version 相同）",
            ));
        }
        for (pin, value) in &u.bind.0 {
            if !is_valid_name(pin) {
                out.push(Diagnostic::error(
                    "R28",
                    &loc,
                    format!("bind 的鍵 `{pin}` 只能使用英數與底線"),
                ));
            }
            let Some(value) = value else {
                continue; // null：刻意不接
            };
            match classify_bind(value) {
                None => out.push(Diagnostic::error(
                    "R28",
                    &loc,
                    format!(
                        "bind.{pin} 的值 `{value}` 格式不符，必須是 null、單元 id，或 plc<N>.<Name> 的 tag id"
                    ),
                )),
                Some(BindRef::Tag(id)) => {
                    if let Some((plc, _)) = split_id(id) {
                        if !plc_ids.contains(plc) {
                            out.push(Diagnostic::error(
                                "R28",
                                &loc,
                                format!(
                                    "bind.{pin} 的 tag `{id}` 所屬的 PLC `{plc}` 沒有在 machine.plcs 宣告"
                                ),
                            ));
                        }
                    }
                }
                Some(BindRef::Unit(_)) => {}
            }
        }
    }
}
