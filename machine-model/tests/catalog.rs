use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;

fn ty(id: &str, version: u32, body: &str) -> String {
    format!("id: {id}\nversion: {version}\ntitle: T\n{body}")
}

fn build(types: &[(&str, String)]) -> Result<Vec<(&'static str, Severity)>, Vec<&'static str>> {
    let sources: Vec<(&str, &str)> = types.iter().map(|(n, t)| (*n, t.as_str())).collect();
    match UnitCatalog::from_sources(&sources) {
        Ok((_, d)) => Ok(d.iter().map(|x| (x.code, x.severity)).collect()),
        Err(d) => Err(d.iter().map(|x| x.code).collect()),
    }
}

fn err_codes(types: &[(&str, String)]) -> Vec<&'static str> {
    build(types).unwrap_err()
}

#[test]
fn valid_catalog_builds_and_can_be_queried() {
    let a = ty("a", 1, "ctrl:\n  man_x: {}\nparts:\n  child: b\n");
    let b = ty("b", 3, "");
    let sources = [("a.yaml", a.as_str()), ("b.yaml", b.as_str())];
    let (cat, diags) = UnitCatalog::from_sources(&sources).unwrap();
    assert_eq!(diags, []);
    assert_eq!(cat.len(), 2);
    assert_eq!(cat.get("b").unwrap().version, 3);
    assert!(cat.get("zzz").is_none());
    assert_eq!(
        cat.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn syntax_errors_are_reported_per_file() {
    let bad = "id: x\n".to_owned();
    assert_eq!(err_codes(&[("x.yaml", bad)]), ["SC2"]);
}

#[test]
fn u17_duplicate_ids_in_one_layer() {
    let a = ty("same", 1, "");
    assert_eq!(err_codes(&[("a.yaml", a.clone()), ("b.yaml", a)]), ["U17"]);
}

#[test]
fn u1_names_are_unique_across_field_ctrl_and_parts() {
    let t = ty(
        "t",
        1,
        "field:\n  in:\n    dup: { type: bool }\n  out:\n    dup: { type: bool }\nparts:\n  dup: other\n",
    );
    let other = ty("other", 1, "");
    let codes = err_codes(&[("t.yaml", t), ("other.yaml", other)]);
    assert_eq!(codes, ["U1"]);
}

#[test]
fn u12_par_range_and_default() {
    let par = |min: &str, max: &str, def: &str, ty_: &str| {
        ty(
            "t",
            1,
            &format!("ctrl:\n  par_x: {{ type: {ty_}, min: {min}, max: {max}, default: {def} }}\n"),
        )
    };
    // 合法
    assert!(build(&[("t.yaml", par("0", "10", "5", "int16"))]).is_ok());
    assert!(build(&[("t.yaml", par("0", "10", "10", "int16"))]).is_ok());
    assert!(build(&[("t.yaml", par("0.5", "1.0", "0.75", "float32"))]).is_ok());
    // default 超出範圍、min > max、default 不是數字、超出型別範圍
    for bad in [
        par("0", "10", "11", "int16"),
        par("0", "10", "-1", "int16"),
        par("10", "0", "5", "int16"),
        par("0", "10", "true", "int16"),
        par("0", "70000", "5", "int16"),
        par("-1", "10", "5", "uint16"),
    ] {
        assert_eq!(err_codes(&[("t.yaml", bad.clone())]), ["U12"], "{bad}");
    }
}

#[test]
fn u15_flags_a_likely_misspelled_function_name_as_warning() {
    let t = ty(
        "t",
        1,
        "ctrl:\n  man_ext: {}\n  at_ext: {}\n  man_exr: {}\n  man_stop: {}\n",
    );
    let diags = build(&[("t.yaml", t)]).unwrap();
    assert_eq!(diags, [("U15", Severity::Warning)]);
    // 兩個都只出現一次：沒有「多數」可比，不提示
    let t = ty("t", 1, "ctrl:\n  man_ext: {}\n  man_exr: {}\n");
    assert_eq!(
        build(&[("t.yaml", t)]).unwrap(),
        Vec::<(&str, Severity)>::new()
    );
}

#[test]
fn u18_parts_must_exist_and_must_not_form_a_cycle() {
    let missing = ty("t", 1, "parts:\n  p: ghost\n");
    assert_eq!(err_codes(&[("t.yaml", missing)]), ["U18"]);
    let selfref = ty("t", 1, "parts:\n  p: t\n");
    assert_eq!(err_codes(&[("t.yaml", selfref)]), ["U18"]);
    let a = ty("a", 1, "parts:\n  p: b\n");
    let b = ty("b", 1, "parts:\n  p: a\n");
    // 環上每個類型各報一次
    assert_eq!(err_codes(&[("a.yaml", a), ("b.yaml", b)]), ["U18", "U18"]);
}

#[test]
fn u19_error_pins_must_be_bool() {
    let t = ty("t", 1, "ctrl:\n  error_x: { type: int16, message: m }\n");
    assert_eq!(err_codes(&[("t.yaml", t)]), ["U19"]);
    let ok = ty("t", 1, "ctrl:\n  error_x: { message: m }\n");
    assert!(build(&[("t.yaml", ok)]).is_ok());
}

#[test]
fn local_types_override_standard_ones_with_an_info_notice() {
    let std_a = ty("a", 1, "ctrl:\n  man_x: {}\n");
    let sources = [("a.yaml", std_a.as_str())];
    let (std, _) = UnitCatalog::from_sources(&sources).unwrap();
    let local_a = ty("a", 2, "ctrl:\n  man_x: {}\n  man_y: {}\n");
    let locals = [("local/a.yaml", local_a.as_str())];
    let (merged, diags) = std.with_local(&locals).unwrap();
    assert_eq!(merged.get("a").unwrap().version, 2);
    assert_eq!(diags.len(), 1);
    assert_eq!((diags[0].code, diags[0].severity), ("U17", Severity::Info));
}

#[test]
fn local_types_are_checked_against_the_combined_catalog() {
    // 本地類型的 part 引用標準類型：存在就通過，不存在就是 U18
    let std_b = ty("b", 1, "");
    let (std, _) = UnitCatalog::from_sources(&[("b.yaml", std_b.as_str())]).unwrap();
    let ok = ty("c", 1, "parts:\n  p: b\n");
    assert!(std.clone().with_local(&[("c.yaml", ok.as_str())]).is_ok());
    let bad = ty("c", 1, "parts:\n  p: ghost\n");
    let err = std.with_local(&[("c.yaml", bad.as_str())]).unwrap_err();
    assert_eq!(err.iter().map(|d| d.code).collect::<Vec<_>>(), ["U18"]);
}
