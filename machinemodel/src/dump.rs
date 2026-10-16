//! `dump` 子命令：傾印解析與推導後的 `ResolvedModel`（規格 §9.2），預設 YAML，可選 JSON。

use clap::ValueEnum;

use crate::check;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Yaml,
    Json,
}

/// `machinemodel dump <machine.yaml>` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 機台設定檔（`machine.yaml` v2）。
    pub machine: std::path::PathBuf,
    /// 輸出格式（預設 yaml）。
    #[arg(short, long, value_enum, default_value_t = Format::Yaml)]
    pub format: Format,
}

/// 執行傾印：0 成功、1 規則錯誤（不輸出部分結果）、2 I/O 錯誤。
pub fn run(args: &Args) -> i32 {
    let text = match std::fs::read_to_string(&args.machine) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("無法讀取 {}：{e}", args.machine.display());
            return 2;
        }
    };
    let catalog = match check::catalog_beside(&args.machine) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let checked = match machine_model::load_checked(&text, &catalog) {
        Ok(c) => c,
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };
    for d in &checked.warnings {
        eprintln!("{d}");
    }
    let rendered = match args.format {
        Format::Yaml => serde_yaml_ng::to_string(&checked.resolved).map_err(|e| e.to_string()),
        Format::Json => serde_json::to_string_pretty(&checked.resolved).map_err(|e| e.to_string()),
    };
    match rendered {
        Ok(out) => {
            println!("{}", out.trim_end());
            0
        }
        Err(e) => {
            eprintln!("無法輸出傾印結果：{e}");
            2
        }
    }
}
