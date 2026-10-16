#![cfg(feature = "schema")]
#![allow(clippy::assert_is_empty)]

use machine_model::unit_type::{parse_unit_type, validate_unit_type};

fn is_sc(code: &str) -> bool {
    code == "SC2" || code == "SC3"
}

fn both_fail(text: &str) {
    let code_err = parse_unit_type("t.yaml", text).unwrap_err();
    assert!(
        !code_err.is_empty() && code_err.iter().all(|d| is_sc(d.code)),
        "程式碼版應報 SC2/SC3：{text} -> {code_err:?}"
    );
    let schema_err = validate_unit_type(text);
    assert!(
        !schema_err.is_empty() && schema_err.iter().all(|d| is_sc(d.code)),
        "Schema 版應報 SC2/SC3：{text} -> {schema_err:?}"
    );
}

fn both_pass(text: &str) {
    assert!(
        parse_unit_type("t.yaml", text).is_ok(),
        "程式碼版應通過：{text}"
    );
    let errs = validate_unit_type(text);
    assert!(errs.is_empty(), "Schema 版應通過：{text} -> {errs:?}");
}

fn with(ctrl: &str) -> String {
    format!("id: t\nversion: 1\ntitle: T\nctrl:\n{ctrl}")
}

#[test]
fn standard_types_pass_schema_validation() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../standards/unit-types");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("找不到 standards/unit-types") {
        let path = entry.expect("目錄項").path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("yaml"))
        {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("讀取類型檔");
        let errs = validate_unit_type(&text);
        assert!(errs.is_empty(), "{}: {errs:?}", path.display());
        count += 1;
    }
    assert!(count >= 3, "標準類型太少：{count}");
}

#[test]
fn consistency_sc2_structure_errors() {
    for text in [
        "id: t\nversion: 1\n".to_owned(),
        "id: t\nversion: 1\ntitle: T\nbogus: 1\n".to_owned(),
        with("  man_a: { writable: true }\n"),
        with("  man_a: { write_permission: P01 }\n"),
        with("  man_a: {}\n  man_a: {}\n"),
        "id: t\nversion: 1\ntitle: [\n".to_owned(),
        "id: 'bad id'\nversion: 1\ntitle: T\n".to_owned(),
        "id: t\nversion: 0\ntitle: T\n".to_owned(),
        "id: t\nversion: 1\ntitle: ''\n".to_owned(),
    ] {
        both_fail(&text);
    }
}

#[test]
fn consistency_sc3_prefix_rules() {
    for text in [
        with("  lim_ext: { type: bool }\n"),
        with("  par: {}\n"),
        with("  pv_t: {}\n"),
        with("  msg_m: {}\n"),
        with("  par_x: { type: int16 }\n"),
        with("  par_x: { type: int16, min: 0, max: 5 }\n"),
        with("  error_x: {}\n"),
        with("  error_x: { message: '' }\n"),
        with("  msg_m: { type: int16 }\n"),
        with("  man_a: { type: integer }\n"),
        with("  pv_t: { type: 'string:0' }\n"),
        with("  par_x: { type: 'int16[0]' }\n"),
        with("  par_x: { type: 'int16[501]' }\n"),
        with("  par_x: { type: 'string:4[2]' }\n"),
    ] {
        both_fail(&text);
    }
    both_pass(&with("  msg_m: { type: 'string:16' }\n"));
}

#[test]
fn consistency_field_and_part_rules() {
    both_fail("id: t\nversion: 1\ntitle: T\nfield:\n  in:\n    man_x: { type: bool }\n");
    both_fail("id: t\nversion: 1\ntitle: T\nfield:\n  out:\n    v: { type: other_unit }\n");
    both_fail("id: t\nversion: 1\ntitle: T\nparts:\n  p: bool\n");
    both_fail("id: t\nversion: 1\ntitle: T\nparts:\n  'p-1': other\n");
}

#[test]
fn schema_diagnostics_are_traditional_chinese_with_location() {
    let errs = validate_unit_type(&with("  lim_ext: {}\n"));
    assert!(!errs.is_empty());
    let text = errs[0].to_string();
    assert!(text.starts_with("[SC"), "{text}");
    assert!(text.contains("types[t].ctrl[lim_ext]"), "{text}");
}
