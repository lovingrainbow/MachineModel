#![allow(clippy::assert_is_empty)]

use machine_model::catalog::UnitCatalog;
use machine_model::naming::Prefix;
use machine_model::resolve::{ResolvedModel, RoleSource, resolve};
use machine_model::schema::Role;

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn resolved() -> ResolvedModel {
    let model = machine_model::load(FIXTURE).expect("fixture 必須通過載入安全規則");
    let catalog = UnitCatalog::standard();
    let diags = machine_model::analyze::check(&model, &catalog);
    assert!(diags.is_empty(), "{diags:#?}");
    resolve(&model, &catalog, FIXTURE)
}

#[test]
fn write_candidate_is_true_only_for_man_and_par_bindings() {
    let r = resolved();
    let wc = |id: &str| r.tag(id).unwrap().write_candidate;
    for id in [
        "plc1.M_Cyl1_ManExt",
        "plc1.M_Cyl1_ManRet",
        "plc1.D_Cyl1_ExtTime", // par_ext_time
        "plc1.D_ModeCmd",      // man_mode
    ] {
        assert!(wc(id), "{id}");
    }
    for id in [
        "plc1.X_Cyl1_Ext_LS", // field 腳
        "plc1.Y_Cyl1_Ext",    // field 腳
        "plc1.M_Cyl1_ErrExt", // error_
        "plc1.M_Estop",
        "plc1.D_ModeAct", // at_mode
        "plc1.M_Spare",   // 沒被任何 Unit 綁定：fail-closed
    ] {
        assert!(!wc(id), "{id}");
    }
}

#[test]
fn role_comes_from_the_pin_prefix_unless_explicit() {
    let r = resolved();
    let t = r.tag("plc1.M_Cyl1_ManExt").unwrap();
    assert_eq!(t.role, Some(Role::Command));
    assert_eq!(t.role_source, RoleSource::Pin(Prefix::Man));
    assert_eq!(r.tag("plc1.M_Cyl1_ErrExt").unwrap().role, Some(Role::Alarm));
    assert_eq!(
        r.tag("plc1.D_Cyl1_ExtTime").unwrap().role,
        Some(Role::Setpoint)
    );
    // field 腳與未綁定的 Tag 沒有 role
    for id in ["plc1.X_Cyl1_Ext_LS", "plc1.M_Spare"] {
        let t = r.tag(id).unwrap();
        assert_eq!((t.role, t.role_source), (None, RoleSource::Unknown), "{id}");
    }
}

#[test]
fn explicit_role_wins_and_is_reported_as_explicit() {
    let text = FIXTURE.replace(
        "{ id: plc1.M_Spare, address: M130, type: bool }",
        "{ id: plc1.M_Spare, address: M130, type: bool, role: status }",
    );
    let model = machine_model::load(&text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), &text);
    let t = r.tag("plc1.M_Spare").unwrap();
    assert_eq!(
        (t.role, t.role_source),
        (Some(Role::Status), RoleSource::Explicit)
    );
    assert!(!t.write_candidate);
}

#[test]
fn par_pin_supplies_range_default_and_unit_when_the_tag_has_none() {
    let r = resolved();
    let t = r.tag("plc1.D_Cyl1_ExtTime").unwrap();
    assert_eq!((t.min, t.max), (Some(0.0), Some(30000.0)));
    assert_eq!(t.default, Some(machine_model::schema::Scalar::Int(3000)));
    assert_eq!(t.unit.as_deref(), Some("ms"));
}

#[test]
fn tag_values_beat_pin_values() {
    let text = FIXTURE.replace(
        "{ id: plc1.D_Cyl1_ExtTime, address: D100, type: int16 }",
        "{ id: plc1.D_Cyl1_ExtTime, address: D100, type: int16, min: 100, max: 9000, default: 500, unit: s }",
    );
    let model = machine_model::load(&text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), &text);
    let t = r.tag("plc1.D_Cyl1_ExtTime").unwrap();
    assert_eq!((t.min, t.max), (Some(100.0), Some(9000.0)));
    assert_eq!(t.unit.as_deref(), Some("s"));
}

#[test]
fn class_and_binding_details() {
    let r = resolved();
    assert!(r.tag("plc1.X_Cyl1_Ext_LS").unwrap().class_read_only);
    assert!(!r.tag("plc1.M_Cyl1_ManExt").unwrap().class_read_only);
    assert!(r.tag("plc1.M_Estop").unwrap().safety_monitor);
    let b = r.tag("plc1.M_Cyl1_ManExt").unwrap().binding().unwrap();
    assert_eq!(
        (
            b.unit_id.as_str(),
            b.unit_path.as_str(),
            b.pin.as_str(),
            b.prefix
        ),
        ("cyl1", "machine/st1/cyl1", "man_ext", Some(Prefix::Man))
    );
    assert_eq!(r.tag("plc1.M_Cyl1_ManExt").unwrap().bindings.len(), 1);
    let f = r.tag("plc1.Y_Cyl1_Ext").unwrap().binding().unwrap();
    assert_eq!(f.prefix, None);
    assert!(r.tag("plc1.M_Spare").unwrap().bindings.is_empty());
    assert!(r.tag("plc1.M_Spare").unwrap().binding().is_none());
    assert!(!r.tag("plc1.M_Spare").unwrap().spare);
    assert_eq!(r.tag("plc1.M_Spare").unwrap().polarity, None);
    assert!(r.tag("plc1.M_Spare").unwrap().ext.is_empty());
}

#[test]
fn unit_tree_and_paths() {
    let r = resolved();
    let u = |id: &str| r.units.iter().find(|u| u.id == id).unwrap();
    assert_eq!(u("machine").parent, None);
    assert_eq!(u("machine").path, "machine");
    assert_eq!(u("cyl1").parent.as_deref(), Some("st1"));
    assert_eq!(u("cyl1").path, "machine/st1/cyl1");
    assert_eq!(u("cyl1").type_version, 1);
}

#[test]
fn pin_lists_carry_defaults_and_labels() {
    let r = resolved();
    let op = r
        .operations()
        .find(|p| p.unit_id == "cyl1" && p.pin == "man_ext")
        .unwrap();
    assert_eq!(op.label, "伸出");
    assert_eq!(op.tag.as_deref(), Some("plc1.M_Cyl1_ManExt"));

    // 順序：依 units[] 的宣告順序（machine、st1、cyl1），單元內依接腳名字母序
    let alarms: Vec<_> = r
        .alarms()
        .map(|p| (p.unit_id.as_str(), p.pin.as_str()))
        .collect();
    assert_eq!(
        alarms,
        [
            ("machine", "error_estop"),
            ("cyl1", "error_ext"),
            ("cyl1", "error_ret")
        ]
    );
    let ext = r.alarms().find(|p| p.pin == "error_ext").unwrap();
    assert_eq!(ext.message.as_deref(), Some("伸出逾時"));

    assert_eq!(
        r.parameters().map(|p| p.pin.as_str()).collect::<Vec<_>>(),
        ["par_cycle_time", "par_err_time", "par_ext_time"]
    );
    assert!(r.values().any(|p| p.pin == "count_done"));
    assert_eq!(r.messages().count(), 0);
}

#[test]
fn label_falls_back_to_the_function_name() {
    // station_load 的 at_running 沒有 label；詞典有 running → 運轉
    let r = resolved();
    let p = r
        .pins
        .iter()
        .find(|p| p.unit_id == "st1" && p.pin == "at_running")
        .unwrap();
    assert_eq!(p.label, "運轉");
    let p = r
        .pins
        .iter()
        .find(|p| p.unit_id == "st1" && p.pin == "count_done")
        .unwrap();
    assert_eq!(p.label, "done");
}

#[test]
fn coverage_gaps_list_unbound_optional_pins_but_not_wiring_pins() {
    let r = resolved();
    let gaps: Vec<_> = r
        .coverage_gaps()
        .iter()
        .map(|p| (p.unit_id.clone(), p.pin.clone()))
        .collect();
    assert!(gaps.contains(&("cyl1".to_owned(), "par_err_time".to_owned())));
    assert!(gaps.contains(&("st1".to_owned(), "man_start".to_owned())));
    // 已綁定的不在清單內；cmd_／ena_ 純接線不算
    assert!(!gaps.contains(&("cyl1".to_owned(), "man_ext".to_owned())));
    assert!(
        !gaps
            .iter()
            .any(|(_, pin)| pin.starts_with("cmd_") || pin.starts_with("ena_"))
    );
}

#[test]
fn fingerprint_pins_source_and_type_versions() {
    let r = resolved();
    assert_eq!(r.fingerprint.source_hash.len(), 16);
    let crlf = FIXTURE.replace('\n', "\r\n");
    let model = machine_model::load(&crlf).unwrap();
    let r2 = resolve(&model, &UnitCatalog::standard(), &crlf);
    assert_eq!(r.fingerprint.source_hash, r2.fingerprint.source_hash);
    let v: Vec<_> = r
        .fingerprint
        .type_versions
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    assert_eq!(
        v,
        [
            ("cylinder_double", 1),
            ("machine_ctrl", 2),
            ("station_load", 1)
        ]
    );
}

#[test]
fn resolve_does_not_panic_on_an_unvalidated_model() {
    // 先決條件被違反（未知類型、不存在的 Tag）時也不得 panic
    let text = "schema_version: 2\nmachine: {id: m, plcs: [{id: plc1, family: inovance_easy}]}\n\
units:\n  - {id: machine, type: ghost, type_version: 9, bind: {x: plc1.NOPE, y: other}}\n";
    let model = machine_model::parse(text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), text);
    assert!(r.tags.is_empty());
}

#[test]
fn write_reason_names_the_binding_pin() {
    let r = resolved();
    let reason = r
        .tag("plc1.M_Cyl1_ManExt")
        .unwrap()
        .write_candidate_reason();
    assert!(reason.contains("cyl1.man_ext"), "{reason}");
    assert!(reason.contains("man_*"), "{reason}");

    let reason = r
        .tag("plc1.X_Cyl1_Ext_LS")
        .unwrap()
        .write_candidate_reason();
    assert!(reason.contains("cyl1.lim_ext"), "{reason}");
    assert!(reason.contains("不是可寫候選"), "{reason}");
}

#[test]
fn write_candidate_is_fail_closed_without_bindings() {
    let r = resolved();
    let t = r.tag("plc1.M_Spare").unwrap();
    assert!(!t.write_candidate);
    assert!(t.write_candidate_reason().contains("沒有綁定"));
}
