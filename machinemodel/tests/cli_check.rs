//! `check` 子命令測試。

use std::process::Command;

fn binary() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_machinemodel"))
}

fn fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../machine-model/tests/fixtures/valid_units.yaml")
}

fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).expect("寫入暫存檔");
    path
}

#[test]
fn valid_model_exits_0() {
    let out = Command::new(binary())
        .args(["check", &fixture().display().to_string()])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn broken_binding_exits_1() {
    let bad = write_temp(
        "machinemodel-test-bad.yaml",
        "schema_version: 2\nmachine:\n  id: demo\n  plcs:\n    - { id: plc1, family: inovance_easy }\nunits:\n  - { id: machine, type: machine_ctrl, type_version: 2, bind: { stn: ghost } }\ntags: []\n",
    );
    let out = Command::new(binary())
        .args(["check", &bad.display().to_string()])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(1), "違規模型應回 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("U5"), "{stderr}");
}

#[test]
fn report_and_coverage_print_to_stdout() {
    let out = Command::new(binary())
        .args([
            "check",
            &fixture().display().to_string(),
            "--report",
            "--coverage",
        ])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("plc1.M_Cyl1_ManExt"), "{stdout}");
    assert!(stdout.contains("覆蓋率"), "{stdout}");
}

/// 每個測試用獨立的暫存目錄（位於 cargo 的 target 暫存區）。
fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建立暫存目錄");
    dir
}

const PUMP_TYPE: &str = "id: pump_x\nversion: 1\ntitle: 泵浦\nctrl:\n  cmd_run: { type: bool }\n";

const PUMP_MODEL: &str = "schema_version: 2\nmachine:\n  id: demo\n  plcs:\n    - { id: plc1, family: inovance_easy }\nunits:\n  - { id: machine, type: pump_x, type_version: 1 }\ntags: []\n";

fn check_path(machine: &std::path::Path) -> std::process::Output {
    Command::new(binary())
        .args(["check", &machine.display().to_string()])
        .output()
        .expect("無法執行 machinemodel")
}

#[test]
fn types_dir_next_to_machine_is_read_without_a_flag() {
    let dir = fresh_dir("cli-check-types-dir");
    std::fs::create_dir(dir.join("types")).unwrap();
    std::fs::write(dir.join("types/pump_x.yaml"), PUMP_TYPE).unwrap();
    let machine = dir.join("machine.yaml");
    std::fs::write(&machine, PUMP_MODEL).unwrap();
    let out = check_path(&machine);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn local_type_missing_from_types_dir_exits_1() {
    let dir = fresh_dir("cli-check-no-types-dir");
    let machine = dir.join("machine.yaml");
    std::fs::write(&machine, PUMP_MODEL).unwrap();
    let out = check_path(&machine);
    assert_eq!(
        out.status.code(),
        Some(1),
        "沒有 types/ 時 pump_x 未知，應回 1"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("pump_x"), "{stderr}");
}

#[test]
fn types_path_that_is_a_file_exits_2() {
    let dir = fresh_dir("cli-check-types-is-file");
    std::fs::write(dir.join("types"), "not a directory").unwrap();
    let machine = dir.join("machine.yaml");
    std::fs::write(&machine, PUMP_MODEL).unwrap();
    let out = check_path(&machine);
    assert_eq!(out.status.code(), Some(2), "types 不是目錄應回 2");
}

#[test]
fn local_type_with_standard_id_overrides_with_notice() {
    let dir = fresh_dir("cli-check-override");
    std::fs::create_dir(dir.join("types")).unwrap();
    let standard = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../standards/unit-types/cylinder_double.yaml");
    std::fs::copy(&standard, dir.join("types/cylinder_double.yaml")).unwrap();
    let machine = dir.join("machine.yaml");
    std::fs::copy(fixture(), &machine).unwrap();
    let out = check_path(&machine);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("覆蓋同 id 的標準類型"), "{stderr}");
}

#[test]
fn local_types_flag_is_removed_exits_2() {
    // 規格 §9.5 修訂：不再提供 --local-types，類型目錄只由 machine.yaml 旁的 types/ 自動讀取
    let dir = fresh_dir("cli-check-flag-removed");
    std::fs::create_dir(dir.join("types")).unwrap();
    let out = Command::new(binary())
        .args([
            "check",
            &fixture().display().to_string(),
            "--local-types",
            &dir.display().to_string(),
        ])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(2), "已移除的旗標應回 2");
}
