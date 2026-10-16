use machine_model::{load, parse, validate};

const FULL: &str = include_str!("fixtures/structural_full.yaml");

#[test]
fn full_fixture_loads_with_no_diagnostics() {
    let model = parse(FULL).expect("parse");
    let diags = validate(&model);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(model.tags.len(), 8);
    assert_eq!(model.variables.len(), 1);
    assert_eq!(model.units.len(), 2);
    assert_eq!(model.units[0].type_version, 1);
}

#[test]
fn full_fixture_survives_crlf_and_bom() {
    let text = format!("\u{feff}{}", FULL.replace('\n', "\r\n"));
    assert!(load(&text).is_ok());
}

#[test]
fn removing_a_required_field_from_the_fixture_is_reported_not_ignored() {
    // 每個 tag 都需要 type；拿掉一個要得到 PARSE，而不是被當成預設值
    let broken = FULL.replace(
        "{ id: plc1.Y_Cyl1_Ext, address: Y10, type: bool }",
        "{ id: plc1.Y_Cyl1_Ext, address: Y10 }",
    );
    assert_ne!(broken, FULL);
    let diag = parse(&broken).unwrap_err();
    assert_eq!(diag.code, "PARSE");
}
