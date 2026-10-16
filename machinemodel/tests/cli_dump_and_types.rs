//! `dump` 與 `check-types` 子命令測試。

use std::process::{Command, Output};

fn binary() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_machinemodel"))
}

fn fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../machine-model/tests/fixtures/valid_units.yaml")
}

/// 每個測試用獨立的暫存目錄（位於 cargo 的 target 暫存區）。
fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建立暫存目錄");
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("無法執行 machinemodel")
}

fn code(out: &Output) -> Option<i32> {
    out.status.code()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

const PUMP_TYPE: &str = "id: pump_x\nversion: 1\ntitle: 泵浦\nctrl:\n  cmd_run: { type: bool }\n";

const BROKEN_TYPE: &str = "id: bad_t\nversion: 1\ntitle: 壞類型\nctrl:\n  lim_ext: {}\n";

#[test]
fn dump_defaults_to_yaml_with_resolved_fields() {
    let out = run(&["dump", &fixture().display().to_string()]);
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("machine_id: demo"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn dump_json_is_parseable_and_carries_machine_id() {
    let out = run(&["dump", &fixture().display().to_string(), "-f", "json"]);
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
    let v: serde_json::Value =
        serde_json::from_str(&stdout(&out)).expect("dump -f json 必須是合法 JSON");
    assert_eq!(v["machine_id"], "demo");
}

#[test]
fn dump_unknown_format_exits_2() {
    let out = run(&["dump", &fixture().display().to_string(), "-f", "xml"]);
    assert_eq!(code(&out), Some(2));
}

#[test]
fn dump_invalid_model_exits_1_without_output() {
    let dir = fresh_dir("cli-dump-invalid");
    let bad = dir.join("machine.yaml");
    std::fs::write(
        &bad,
        "schema_version: 2\nmachine:\n  id: demo\n  plcs:\n    - { id: plc1, family: inovance_easy }\nunits:\n  - { id: machine, type: machine_ctrl, type_version: 2, bind: { stn: ghost } }\ntags: []\n",
    )
    .unwrap();
    let out = run(&["dump", &bad.display().to_string()]);
    assert_eq!(code(&out), Some(1), "{}", stderr(&out));
    assert!(
        stdout(&out).is_empty(),
        "出錯時不應輸出部分結果：{}",
        stdout(&out)
    );
}

#[test]
fn check_types_without_arguments_validates_standard_types() {
    let out = run(&["check-types"]);
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("machine_ctrl"), "{}", stdout(&out));
}

#[test]
fn check_types_dir_with_valid_type_exits_0() {
    let dir = fresh_dir("cli-types-valid");
    std::fs::write(dir.join("pump_x.yaml"), PUMP_TYPE).unwrap();
    let out = run(&["check-types", &dir.display().to_string()]);
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("pump_x"), "{}", stdout(&out));
}

#[test]
fn check_types_dir_with_schema_violation_exits_1() {
    let dir = fresh_dir("cli-types-bad");
    std::fs::write(dir.join("bad_t.yaml"), BROKEN_TYPE).unwrap();
    let out = run(&["check-types", &dir.display().to_string()]);
    assert_eq!(code(&out), Some(1), "{}", stdout(&out));
    assert!(stderr(&out).contains("SC3"), "{}", stderr(&out));
}

#[test]
fn check_types_dir_duplicate_id_is_u17() {
    let dir = fresh_dir("cli-types-dup");
    std::fs::write(dir.join("a.yaml"), PUMP_TYPE).unwrap();
    std::fs::write(dir.join("b.yaml"), PUMP_TYPE).unwrap();
    let out = run(&["check-types", &dir.display().to_string()]);
    assert_eq!(code(&out), Some(1));
    assert!(stderr(&out).contains("U17"), "{}", stderr(&out));
}

#[test]
fn check_types_dir_missing_exits_2() {
    let out = run(&["check-types", "/nonexistent-types-dir-for-test"]);
    assert_eq!(code(&out), Some(2));
}

#[test]
fn check_types_dir_runs_type_layer_rules_on_merged_catalog() {
    // par_ 範圍倒置（min > max）是 U12，由合併後目錄的類型層規則抓出，不是 SC2／SC3
    let dir = fresh_dir("cli-types-u12");
    std::fs::write(
        dir.join("par_x.yaml"),
        "id: par_x\nversion: 1\ntitle: 參數\nctrl:\n  par_v: { type: int16, min: 10, max: 5, default: 7 }\n",
    )
    .unwrap();
    let out = run(&["check-types", &dir.display().to_string()]);
    assert_eq!(code(&out), Some(1));
    assert!(stderr(&out).contains("U12"), "{}", stderr(&out));
}
