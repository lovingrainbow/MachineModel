//! 結構規則：R1、R21、R27。

use std::collections::BTreeSet;

use crate::diag::Diagnostic;
use crate::schema::{Machine, is_valid_plc_id, split_id};

pub(crate) fn version_error(found: u32, supported: u32) -> Diagnostic {
    Diagnostic::error(
        "R27",
        "schema_version",
        format!("不支援 schema_version {found}，此工具只支援 {supported}"),
    )
}

fn is_machine_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// R21：機台 id、PLC id 格式與唯一，離線以外的 PLC 必須有可用協定。回傳已宣告的 PLC id。
pub(super) fn check_machine_and_plcs<'a>(
    machine: &'a Machine,
    out: &mut Vec<Diagnostic>,
) -> BTreeSet<&'a str> {
    if !is_machine_id(&machine.id) {
        out.push(Diagnostic::error(
            "R21",
            "machine.id",
            format!(
                "機台 id `{}` 格式不符，只能使用英數、底線與連字號（ASCII）",
                machine.id
            ),
        ));
    }
    let mut seen = BTreeSet::new();
    for plc in &machine.plcs {
        let loc = format!("machine.plcs[{}]", plc.id);
        if !is_valid_plc_id(&plc.id) {
            out.push(Diagnostic::error(
                "R21",
                &loc,
                format!(
                    "PLC id `{}` 格式不符，必須是 plc 加正整數（如 plc1）",
                    plc.id
                ),
            ));
        }
        if !seen.insert(plc.id.as_str()) {
            out.push(Diagnostic::error("R21", &loc, "PLC id 重複"));
        }
        if plc.online && plc.family.profile().protocols.is_empty() {
            out.push(Diagnostic::error(
                "R21",
                &loc,
                format!(
                    "家族 {} 沒有可用的連線協定，必須設為 online: false",
                    plc.family.as_str()
                ),
            ));
        }
    }
    seen
}

/// R1：id 格式 `plc<N>.<Name>`、前綴已宣告、全機唯一（tags 與 variables 共用命名空間）。
pub(super) fn check_ids<'a>(
    ids: impl Iterator<Item = (&'static str, &'a str)>,
    plc_ids: &BTreeSet<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (section, id) in ids {
        let loc = format!("{section}[{id}]");
        match split_id(id) {
            None => out.push(Diagnostic::error(
                "R1",
                &loc,
                "id 格式不符，必須是 plc<N>.<Name>，Name 只能使用英數與底線",
            )),
            Some((plc, _)) if !plc_ids.contains(plc) => out.push(Diagnostic::error(
                "R1",
                &loc,
                format!("id 的前綴 `{plc}` 沒有在 machine.plcs 宣告"),
            )),
            Some(_) => {}
        }
        if !seen.insert(id) {
            out.push(Diagnostic::error(
                "R1",
                &loc,
                "id 重複（tags 與 variables 共用命名空間，全機唯一）",
            ));
        }
    }
}
