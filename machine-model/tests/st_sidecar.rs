//! `.st` 旁檔的載入、覆蓋與釘選（規格 §4.3、§9.1）。測試夾具不代表任何標準類型的內容。

use machine_model::catalog::UnitCatalog;
use machine_model::diag::Diagnostic;
use machine_model::hash::{content_hash, type_hash};

const CYLINDER_YAML: &str = include_str!("../../standards/unit-types/cylinder_double.yaml");

const CYLINDER_ST: &str = "\
FUNCTION_BLOCK cylinder_double
VAR_INPUT
    lim_ext : BOOL;
    lim_ret : BOOL;
END_VAR
VAR_OUTPUT
    valve_ext : BOOL;
    valve_ret : BOOL;
END_VAR
VAR_IN_OUT
    ctrl : ST_cylinder_double;
END_VAR
valve_ext := ctrl.cmd_ext;
END_FUNCTION_BLOCK
";

fn catalog() -> UnitCatalog {
    UnitCatalog::from_sources(&[("cylinder_double.yaml", CYLINDER_YAML)])
        .unwrap()
        .0
}

fn codes(d: &[Diagnostic]) -> Vec<&'static str> {
    d.iter().map(|x| x.code).collect()
}

#[test]
fn catalog_without_sidecar_has_none() {
    assert_eq!(catalog().st_source("cylinder_double"), None);
}

#[test]
fn sidecar_is_attached_and_readable() {
    let (cat, diags) = catalog()
        .with_st(&[("cylinder_double", CYLINDER_ST)])
        .unwrap();
    assert_eq!(diags, []);
    assert_eq!(cat.st_source("cylinder_double"), Some(CYLINDER_ST));
}

#[test]
fn sidecar_without_type_file_is_sc5() {
    let d = catalog().with_st(&[("ghost", "x")]).unwrap_err();
    assert_eq!(codes(&d), ["SC5"]);
    assert!(d[0].message.contains("ghost.yaml"), "{}", d[0]);
}

#[test]
fn duplicate_sidecar_is_sc5() {
    let d = catalog()
        .with_st(&[
            ("cylinder_double", CYLINDER_ST),
            ("cylinder_double", CYLINDER_ST),
        ])
        .unwrap_err();
    assert_eq!(codes(&d), ["SC5"]);
}

#[test]
fn st_rules_run_when_attaching() {
    let bad = CYLINDER_ST.replace(
        "END_VAR\nvalve_ext",
        "END_VAR\nvalve_ext := ctrl.nope;\nvalve_ext",
    );
    let d = catalog().with_st(&[("cylinder_double", &bad)]).unwrap_err();
    assert_eq!(codes(&d), ["U22"]);
}

#[test]
fn local_type_overriding_a_standard_type_drops_its_sidecar() {
    let (cat, _) = catalog()
        .with_st(&[("cylinder_double", CYLINDER_ST)])
        .unwrap();
    let local = CYLINDER_YAML.replace("version: 1", "version: 2");
    let (cat, _) = cat.with_local(&[("local.yaml", local.as_str())]).unwrap();
    assert_eq!(cat.get("cylinder_double").unwrap().version, 2);
    assert_eq!(cat.st_source("cylinder_double"), None);
}

#[test]
fn local_type_without_override_keeps_other_sidecars() {
    let (cat, _) = catalog()
        .with_st(&[("cylinder_double", CYLINDER_ST)])
        .unwrap();
    let other = "id: other\nversion: 1\ntitle: T\n";
    let (cat, _) = cat.with_local(&[("other.yaml", other)]).unwrap();
    assert_eq!(cat.st_source("cylinder_double"), Some(CYLINDER_ST));
}

#[test]
fn type_hash_covers_the_sidecar() {
    // 沒有旁檔時與舊的雜湊相同，既有釘選不受影響。
    assert_eq!(type_hash(CYLINDER_YAML, None), content_hash(CYLINDER_YAML));
    let with = type_hash(CYLINDER_YAML, Some(CYLINDER_ST));
    assert_ne!(with, content_hash(CYLINDER_YAML));
    assert_ne!(with, type_hash(CYLINDER_YAML, Some("changed")));
    // 換行與 BOM 不影響。
    let crlf = CYLINDER_ST.replace('\n', "\r\n");
    assert_eq!(with, type_hash(CYLINDER_YAML, Some(&crlf)));
}
