//! `dump` 子命令（Task 6 實作完整邏輯；此處為 Task 4 骨架）。

/// `machinemodel dump <machine.yaml>` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 機台設定檔（`machine.yaml` v2）。
    pub machine: std::path::PathBuf,
}

/// 執行傾印。I/O 失敗回 2。
pub fn run(args: Args) -> i32 {
    match std::fs::read_to_string(&args.machine) {
        Err(e) => {
            eprintln!("無法讀取 {}：{e}", args.machine.display());
            2
        }
        Ok(_) => 0,
    }
}
