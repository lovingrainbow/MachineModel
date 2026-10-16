//! `check-types` 子命令（Task 6 實作完整邏輯；此處為 Task 4 骨架）。

/// `machinemodel check-types [<DIR>]` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 本地類型目錄；省略時只檢查內嵌標準類型。
    pub dir: Option<std::path::PathBuf>,
}

/// 執行類型檢查。
pub fn run(_args: Args) -> i32 {
    0
}
