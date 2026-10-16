//! `write-report` 子命令（Task 7 實作完整邏輯；此處為 Task 4 骨架）.

/// `machinemodel write-report <machine.yaml> --service <service.toml>` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 機台設定檔（`machine.yaml` v2）。
    pub machine: std::path::PathBuf,
    /// 服務設定檔（含 `[[tag_override]]`）。
    #[arg(long)]
    pub service: std::path::PathBuf,
}

/// 執行報告。I/O 失敗回 2。
pub fn run(args: Args) -> i32 {
    for path in [&args.machine, &args.service] {
        if let Err(e) = std::fs::read_to_string(path) {
            eprintln!("無法讀取 {}：{e}", path.display());
            return 2;
        }
    }
    0
}
