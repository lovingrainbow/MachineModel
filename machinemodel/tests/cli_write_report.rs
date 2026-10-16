//! `write-report` 子命令與 §9.3 安全護欄測試。

use std::process::{Command, Output};

fn binary() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_machinemodel"))
}

fn fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../machine-model/tests/fixtures/valid_units.yaml")
}

/// 每個測試用獨立的暫存目錄（位於 cargo 的 target 暫存區），內含一份 machine.yaml。
fn machine_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建立暫存目錄");
    std::fs::copy(fixture(), dir.join("machine.yaml")).expect("複製 fixture");
    dir
}

/// 寫入 service.toml 並執行 `write-report`。
fn report(dir: &std::path::Path, service: &str) -> Output {
    let service_path = dir.join("service.toml");
    std::fs::write(&service_path, service).expect("寫入 service.toml");
    Command::new(binary())
        .args([
            "write-report",
            &dir.join("machine.yaml").display().to_string(),
            "--service",
            &service_path.display().to_string(),
        ])
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

#[test]
fn legal_override_prints_final_list_with_override_source() {
    let dir = machine_dir("cli-report-legal");
    let out = report(
        &dir,
        "[[tag_override]]\nid = \"plc1.M_Cyl1_ManExt\"\nwritable = false\n",
    );
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("plc1.M_Cyl1_ManExt"), "{text}");
    assert!(text.contains("override"), "{text}");
}

#[test]
fn guard1_read_only_class_cannot_be_made_writable() {
    let dir = machine_dir("cli-report-guard1");
    let out = report(
        &dir,
        "[[tag_override]]\nid = \"plc1.X_Cyl1_Ext_LS\"\nwritable = true\n",
    );
    assert_eq!(code(&out), Some(1), "{}", stdout(&out));
    assert!(stderr(&out).contains("唯讀"), "{}", stderr(&out));
}

#[test]
fn guard2_safety_monitor_cannot_be_made_writable() {
    let dir = machine_dir("cli-report-guard2");
    let out = report(
        &dir,
        "[[tag_override]]\nid = \"plc1.M_Estop\"\nwritable = true\n",
    );
    assert_eq!(code(&out), Some(1), "{}", stdout(&out));
    assert!(stderr(&out).contains("安全監控"), "{}", stderr(&out));
}

#[test]
fn guard3_override_must_reference_an_existing_tag() {
    let dir = machine_dir("cli-report-guard3");
    let out = report(
        &dir,
        "[[tag_override]]\nid = \"plc1.Ghost\"\nwritable = true\n",
    );
    assert_eq!(code(&out), Some(1), "{}", stdout(&out));
    assert!(stderr(&out).contains("plc1.Ghost"), "{}", stderr(&out));
}

#[test]
fn duplicate_override_ids_are_a_guard_violation() {
    let dir = machine_dir("cli-report-dup");
    let out = report(
        &dir,
        "[[tag_override]]\nid = \"plc1.M_Cyl1_ManExt\"\nwritable = false\n\n[[tag_override]]\nid = \"plc1.M_Cyl1_ManExt\"\nwritable = false\n",
    );
    assert_eq!(code(&out), Some(1), "{}", stdout(&out));
}

#[test]
fn unrelated_service_settings_are_ignored() {
    // 只讀 [[tag_override]] 子集；IP、Port 等通訊設定不得造成失敗
    let dir = machine_dir("cli-report-subset");
    let out = report(
        &dir,
        "[comm]\nip = \"192.168.0.10\"\nport = 502\nbaud = 9600\n\n[[tag_override]]\nid = \"plc1.M_Cyl1_ManExt\"\nwritable = false\n",
    );
    assert_eq!(code(&out), Some(0), "{}", stderr(&out));
}

#[test]
fn broken_service_toml_exits_2() {
    let dir = machine_dir("cli-report-bad-toml");
    let out = report(&dir, "[[tag_override\n");
    assert_eq!(code(&out), Some(2), "{}", stderr(&out));
}
