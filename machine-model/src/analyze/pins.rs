//! 接腳綁定規則：U4、U6、U10、U11、U13、U14。

use std::collections::BTreeMap;

use crate::address;
use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::family::ClassProfile;
use crate::naming::{BindPolicy, Prefix};
use crate::schema::{BindRef, MachineModel, Plc, Tag, classify_bind, split_id};
use crate::types::DataType;
use crate::unit_type::PinRef;

struct BoundPin<'a> {
    loc: String,
    is_field: bool,
    is_field_out: bool,
    #[allow(dead_code)]
    prefix: Option<Prefix>,
    write_candidate: bool,
    ty: &'a str,
}

pub(super) fn check(m: &MachineModel, catalog: &UnitCatalog, out: &mut Vec<Diagnostic>) {
    let tags: BTreeMap<&str, &Tag> = m.tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let plcs: BTreeMap<&str, &Plc> = m.machine.plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    // 存在的 Tag id → 綁定它的接腳資訊
    let mut users: BTreeMap<&str, Vec<BoundPin<'_>>> = BTreeMap::new();

    for u in &m.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue; // U2 已回報
        };
        let uloc = format!("units[{}]", u.id);

        // U6：必接的 field 腳、必綁的 error 腳
        for (name, pin) in ty.field.input.0.iter().chain(ty.field.output.0.iter()) {
            if pin.required != Some(false) && !u.bind.0.contains_key(name) {
                out.push(Diagnostic::error(
                    "U6",
                    &uloc,
                    format!("field 腳 `{name}` 必須綁定（刻意不接請明確寫 null）"),
                ));
            }
        }
        for name in ty.ctrl.0.keys() {
            let Some((prefix, _)) = crate::naming::split(name) else {
                continue;
            };
            if prefix.info().bind == BindPolicy::Required
                && !matches!(u.bind.get(name), Some(Some(_)))
            {
                out.push(Diagnostic::error(
                    "U6",
                    &uloc,
                    format!("`{name}` 是 error_ 接腳，必須綁定 Tag（不得省略或寫 null）"),
                ));
            }
        }

        for (key, value) in &u.bind.0 {
            let Some(pin) = ty.lookup(key) else {
                continue; // U3 已回報
            };
            let Some(value) = value else {
                continue; // null：刻意不接
            };
            let (code, kind, pin_type): (&'static str, &str, Option<&str>) = match pin {
                PinRef::FieldIn(p) | PinRef::FieldOut(p) => ("U4", "field", Some(p.ty.as_str())),
                PinRef::Ctrl(prefix, p) => ("U10", "ctrl", p.type_text(prefix)),
                PinRef::Part(_) => continue, // 由圖規則處理
            };
            let Some(BindRef::Tag(tag_id)) = classify_bind(value) else {
                out.push(Diagnostic::error(
                    code,
                    &uloc,
                    format!("{kind} 腳 `{key}` 必須綁 Tag（plc<N>.<Name>），而不是 `{value}`"),
                ));
                continue;
            };
            let Some(tag) = tags.get(tag_id) else {
                out.push(Diagnostic::error(
                    code,
                    &uloc,
                    format!("{kind} 腳 `{key}` 綁定的 tag `{tag_id}` 不存在"),
                ));
                continue;
            };

            let (is_field, is_field_out, prefix, write_candidate) = match pin {
                PinRef::FieldIn(_) => (true, false, None, false),
                PinRef::FieldOut(_) => (true, true, None, false),
                PinRef::Ctrl(p, _) => (false, false, Some(p), p.info().write_candidate),
                PinRef::Part(_) => unreachable!(),
            };
            users.entry(tag_id).or_default().push(BoundPin {
                loc: format!("{}.{key}", u.id),
                is_field,
                is_field_out,
                prefix,
                write_candidate,
                ty: pin_type.unwrap_or_default(),
            });

            if let (Some(want), Ok(have)) = (pin_type, DataType::parse(&tag.ty)) {
                if DataType::parse(want).is_ok_and(|w| w != have) {
                    out.push(Diagnostic::error(
                        code,
                        &uloc,
                        format!(
                            "{kind} 腳 `{key}` 的型別是 `{want}`，但 tag `{tag_id}` 的型別是 `{}`",
                            tag.ty
                        ),
                    ));
                }
            }

            let class = class_of(tag, &plcs);
            match pin {
                PinRef::FieldIn(_) => {
                    if class.is_some_and(|c| !c.read_only) {
                        out.push(Diagnostic::error(
                            code,
                            &uloc,
                            format!("field.in 腳 `{key}` 必須綁輸入位址類別（唯讀輸入，如 X），tag `{tag_id}` 不是"),
                        ));
                    }
                }
                PinRef::FieldOut(_) => {
                    if class.is_some_and(|c| c.name != "Y") {
                        out.push(Diagnostic::error(
                            code,
                            &uloc,
                            format!(
                                "field.out 腳 `{key}` 必須綁實體輸出位址類別（如 Y），tag `{tag_id}` 位於 `{}` 類別",
                                class.map_or("未知", |c| c.name)
                            ),
                        ));
                    }
                }
                PinRef::Ctrl(prefix, _) => {
                    if prefix.info().write_candidate {
                        if class.is_some_and(|c| c.read_only) {
                            out.push(Diagnostic::error(
                                "U13",
                                &uloc,
                                format!(
                                    "`{key}` 是操作／參數腳，不得綁唯讀位址類別的 tag `{tag_id}`"
                                ),
                            ));
                        }
                        if tag.safety_monitor == Some(true) {
                            out.push(Diagnostic::error(
                                "U13",
                                &uloc,
                                format!("`{key}` 是操作／參數腳，不得綁 safety_monitor 的 tag `{tag_id}`"),
                            ));
                        }
                    }
                    let effective = prefix.info().role;
                    if tag.role.is_some_and(|r| r != effective) {
                        out.push(Diagnostic::warning(
                            "U14",
                            format!("tags[{tag_id}]"),
                            format!(
                                "明確的 role 與接腳 `{}.{key}` 推測的 {effective:?} 不同；以明確值為準",
                                u.id
                            ),
                        ));
                    }
                }
                PinRef::Part(_) => {}
            }
        }
    }

    for (tag_id, us) in &users {
        if us.len() <= 1 {
            continue;
        }
        let who = us
            .iter()
            .map(|u| u.loc.as_str())
            .collect::<Vec<_>>()
            .join("、");

        // 1. field.out 必須全機唯一
        if us.iter().filter(|u| u.is_field_out).count() > 1 {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("實體輸出 field.out 腳不得由多個單元共用同一 Tag（{who}）"),
            ));
        }

        // 2. 禁止 field 與 ctrl 混用
        let has_field = us.iter().any(|u| u.is_field);
        let has_ctrl = us.iter().any(|u| !u.is_field);
        if has_field && has_ctrl {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 不得同時被 field 腳與 ctrl 腳綁定（{who}）"),
            ));
        }

        // 3. 禁止寫入候選（man_、par_）與唯讀狀態混用
        let has_wc = us.iter().any(|u| u.write_candidate);
        let has_ro = us.iter().any(|u| !u.write_candidate && !u.is_field);
        if has_wc && has_ro {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 不得同時被寫入候選腳與唯讀狀態腳綁定（{who}）"),
            ));
        }

        // 4. 接腳型別宣告必須完全一致
        let first_ty = us[0].ty;
        if us.iter().any(|u| u.ty != first_ty) {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 被型別不同的接腳綁定（{who}）"),
            ));
        }
    }
}

fn class_of(tag: &Tag, plcs: &BTreeMap<&str, &Plc>) -> Option<&'static ClassProfile> {
    let (plc_id, _) = split_id(&tag.id)?;
    let plc = plcs.get(plc_id)?;
    address::parse(plc.family, &tag.address)
        .ok()
        .map(|a| a.class)
}
