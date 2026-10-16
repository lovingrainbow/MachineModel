#![allow(clippy::assert_is_empty)]

use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;
use machine_model::load_checked;

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn codes(text: &str) -> Vec<&'static str> {
    match load_checked(text, &UnitCatalog::standard()) {
        Ok(c) => c.warnings.iter().map(|d| d.code).collect(),
        Err(d) => d.iter().map(|x| x.code).collect(),
    }
}

#[test]
fn the_valid_fixture_loads_end_to_end() {
    let checked = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    assert!(checked.warnings.is_empty());
    assert_eq!(checked.resolved.machine_id, "demo");
    assert!(
        checked
            .resolved
            .tag("plc1.M_Cyl1_ManExt")
            .unwrap()
            .write_candidate
    );
}

#[test]
fn crlf_and_bom_load_the_same() {
    let text = format!("\u{feff}{}", FIXTURE.replace('\n', "\r\n"));
    let a = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    let b = load_checked(&text, &UnitCatalog::standard()).unwrap();
    assert_eq!(a.resolved.fingerprint, b.resolved.fingerprint);
    assert_eq!(a.resolved.tags, b.resolved.tags);
}

#[test]
fn load_rule_errors_stop_before_the_unit_rules() {
    // 位址錯誤（R18）時不再跑 U 規則，避免連鎖雜訊
    let bad = FIXTURE.replace("address: X10, type: bool", "address: Q99, type: bool");
    assert_eq!(codes(&bad), ["R18"]);
}

#[test]
fn stale_type_version_is_u2() {
    let stale = FIXTURE.replace(
        "id: cyl1, type: cylinder_double, type_version: 1",
        "id: cyl1, type: cylinder_double, type_version: 2",
    );
    assert_eq!(codes(&stale), ["U2"]);
}

#[test]
fn operation_pin_on_a_read_only_input_is_rejected() {
    let bad = FIXTURE.replace("man_ext: plc1.M_Cyl1_ManExt", "man_ext: plc1.X_Cyl1_Ext_LS");
    let c = codes(&bad);
    assert!(c.contains(&"U13"), "{c:?}");
    assert!(c.contains(&"U11"), "{c:?}"); // 同一個 X 點也被 lim_ext 綁著
}

#[test]
fn safety_monitor_tag_cannot_be_an_operation() {
    let bad = FIXTURE.replace("man_ext: plc1.M_Cyl1_ManExt", "man_ext: plc1.M_Estop");
    assert!(codes(&bad).contains(&"U13"));
}

#[test]
fn missing_error_binding_is_u6() {
    let bad = FIXTURE.replace("error_ext: plc1.M_Cyl1_ErrExt, ", "");
    assert_eq!(codes(&bad), ["U6"]);
}

#[test]
fn warnings_do_not_fail_the_load() {
    let text = FIXTURE.replace(
        "{ id: plc1.M_Cyl1_ManExt, address: M100, type: bool }",
        "{ id: plc1.M_Cyl1_ManExt, address: M100, type: bool, role: status }",
    );
    let checked = load_checked(&text, &UnitCatalog::standard()).unwrap();
    assert_eq!(checked.warnings.len(), 1);
    assert_eq!(checked.warnings[0].code, "U14");
    assert_eq!(checked.warnings[0].severity, Severity::Warning);
}

#[test]
fn a_catalog_without_the_needed_type_is_u2() {
    let empty = UnitCatalog::from_sources(&[]).unwrap().0;
    let err = load_checked(FIXTURE, &empty).unwrap_err();
    assert!(err.iter().any(|d| d.code == "U2"), "{err:#?}");
}

#[test]
fn unreferenced_tags_are_never_write_candidates() {
    let checked = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    let spare = checked.resolved.tag("plc1.M_Spare").unwrap();
    assert!(!spare.write_candidate);
    assert!(spare.bindings.is_empty());
    assert!(spare.binding().is_none());
}
