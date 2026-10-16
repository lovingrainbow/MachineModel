//! 退出碼規範：0 成功、1 規則錯誤、2 用法或 I/O 錯誤。

use std::process::Command;

fn binary() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_machinemodel"))
}

#[test]
fn unknown_argument_exits_2() {
    let out = Command::new(binary())
        .arg("--nope")
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(2), "未知參數應回 2");
}

#[test]
fn missing_machine_file_exits_2() {
    let out = Command::new(binary())
        .args(["check", "/nonexistent-machine.yaml"])
        .output()
        .expect("無法執行 machinemodel");
    assert_eq!(out.status.code(), Some(2), "找不到檔案應回 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("無法讀取"), "{stderr}");
}
