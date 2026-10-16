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

#[test]
fn missing_local_types_dir_exits_2() {
    let out = Command::new(binary())
        .args([
            "check",
            &fixture().display().to_string(),
            "--local-types",
            "/nonexistent-types-dir",
        ])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(2), "缺失目錄應回 2");
}
