#![allow(clippy::assert_is_empty)]

mod common;

use common::{HEAD, codes, doc, tags};
use machine_model::schema::Scan;
use machine_model::{load, parse, validate};

#[test]
fn defaults_apply_when_fields_are_omitted() {
    let model = parse(&doc(&tags(&["{id: plc1.A, address: M1, type: bool}"]))).unwrap();
    assert_eq!(model.tags[0].scan(), Scan::Normal);
    assert!(model.units.is_empty());
    assert!(model.machine.plcs[0].online);
    assert!(!model.machine.plcs[1].online);
}

#[test]
fn utf8_bom_and_crlf_are_accepted() {
    let text = format!(
        "\u{feff}{}",
        doc(&tags(&["{id: plc1.A, address: M1, type: bool}"])).replace('\n', "\r\n")
    );
    assert_eq!(codes(&text), Vec::<&str>::new());
}

#[test]
fn unknown_field_is_r22() {
    let diag = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, bogus: 1}",
    ])))
    .unwrap_err();
    assert_eq!(diag.code, "R22");
}

#[test]
fn unknown_top_level_field_is_r22() {
    let diag = parse(&format!("{HEAD}extra: 1\n")).unwrap_err();
    assert_eq!(diag.code, "R22");
}

#[test]
fn v1_only_fields_are_r22_not_ignored() {
    for field in ["writable: true", "write_permission: P01", "unit_id: st1"] {
        let entry = format!("{{id: plc1.A, address: M1, type: bool, {field}}}");
        assert_eq!(
            parse(&doc(&tags(&[&entry]))).unwrap_err().code,
            "R22",
            "{field}"
        );
    }
    for block in ["devices: []", "alarms: []", "parameters: []"] {
        let diag = parse(&format!("{HEAD}{block}\n")).unwrap_err();
        assert_eq!(diag.code, "R22", "{block}");
    }
}

#[test]
fn other_syntax_problems_are_parse() {
    for text in [
        String::new(),
        "schema_version: 2\nmachine: [".to_owned(),
        // 缺少必填欄位 type
        doc(&tags(&["{id: plc1.A, address: M1}"])),
        // 未知家族
        HEAD.replace("inovance_easy", "siemens"),
        // 缺少 schema_version
        HEAD.replace("schema_version: 2\n", ""),
        // 單元缺 type_version
        doc("units:\n  - {id: u1, type: t}\n"),
    ] {
        let diag = parse(&text).unwrap_err();
        assert_eq!(diag.code, "PARSE", "{text}");
    }
}

#[test]
fn yaml_no_is_not_silently_a_boolean() {
    // `no` 填給布林欄位：報錯，不會被當成 false
    let diag = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, retain: no}",
    ])))
    .unwrap_err();
    assert_eq!(diag.code, "PARSE");
    // `no` 填給字串欄位：保持字串
    let model = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, description: no}",
    ])))
    .unwrap();
    assert_eq!(model.tags[0].description.as_deref(), Some("no"));
}

#[test]
fn duplicate_keys_are_rejected() {
    let dup_field = doc("tags:\n  - id: plc1.A\n    id: plc1.B\n    address: M1\n    type: bool\n");
    assert_eq!(parse(&dup_field).unwrap_err().code, "PARSE");
    let dup_ext = doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, ext: {inovance: {seq: 1, seq: 2}}}",
    ]));
    assert_eq!(parse(&dup_ext).unwrap_err().code, "PARSE");
    let dup_bind = doc("units:\n  - {id: u1, type: t, type_version: 1, bind: {a: st1, a: st2}}\n");
    assert_eq!(parse(&dup_bind).unwrap_err().code, "PARSE");
}

#[test]
fn schema_version_1_is_r27_even_with_unknown_fields() {
    let text = "schema_version: 1\nmachine: {id: m1}\nfuture_block: []\n";
    assert_eq!(parse(text).unwrap_err().code, "R27");
    let diags = load(text).unwrap_err();
    assert_eq!(diags.len(), 1);
}

#[test]
fn validate_flags_a_wrong_version_on_a_constructed_model() {
    let mut model = parse(&doc("")).unwrap();
    model.schema_version = 3;
    let codes: Vec<_> = validate(&model).iter().map(|d| d.code).collect();
    assert_eq!(codes, ["R27"]);
}

#[test]
fn parse_diagnostics_are_traditional_chinese() {
    let unknown = parse(&format!("{HEAD}extra: 1\n")).unwrap_err();
    assert!(
        unknown.message.starts_with("未知欄位"),
        "{}",
        unknown.message
    );
    let syntax = parse("schema_version: 2\nmachine: [").unwrap_err();
    assert!(
        syntax.message.starts_with("YAML 格式或型別錯誤"),
        "{}",
        syntax.message
    );
}
