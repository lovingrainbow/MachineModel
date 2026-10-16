//! `machinemodel`：`machine.yaml` v2 的設計期檢查命令列工具（規格 §9.5）。
//!
//! 退出碼：0 通過（無 Error 診斷）、1 規則或模型錯誤（含安全護欄違規）、
//! 2 用法或 I/O 錯誤。

use clap::{Parser, Subcommand};

mod check;
mod check_types;
mod dump;
mod write_report;

/// 設計期檢查命令列工具。
#[derive(Debug, Parser)]
#[command(
    name = "machinemodel",
    version,
    about = "machine.yaml v2 設計期檢查工具"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 載入安全規則與 U 規則檢查，可選輸出推導與覆蓋率報告。
    Check(check::Args),
    /// 傾印解析結果（§9.2）為 YAML／JSON。
    Dump(dump::Args),
    /// 檢查類型檔（含 JSON Schema）與標準目錄。
    CheckTypes(check_types::Args),
    /// 列出最終可寫清單與覆寫差異（§9.3）。
    WriteReport(write_report::Args),
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = Cli::parse();
    match cli.command {
        Command::Check(args) => check::run(&args),
        Command::Dump(args) => dump::run(&args),
        Command::CheckTypes(args) => check_types::run(&args),
        Command::WriteReport(args) => write_report::run(&args),
    }
}
