//! `check-types` 子命令：類型檔與標準目錄的設計期檢查（規格 §9.5）。
//!
//! 無參數時檢查內嵌標準類型；給 `<DIR>` 時把該目錄的類型疊加到標準目錄上（同 id 覆蓋），
//! 再對合併後的目錄執行類型層規則（U1、U12、U15、U18、U19），並對每個來源檔做 SC2／SC3 驗證。

use machine_model::catalog::UnitCatalog;
use machine_model::standard::standard_sources;
use machine_model::unit_type::validate_unit_type;

use crate::check;

/// `machinemodel check-types [<DIR>]` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 本地類型目錄；省略時只檢查內嵌標準類型。
    pub dir: Option<std::path::PathBuf>,
}

/// 執行類型檢查：0 全部有效、1 有錯誤、2 目錄讀取失敗。
pub fn run(args: &Args) -> i32 {
    let standard: Vec<(String, String)> = standard_sources()
        .iter()
        .map(|(n, t)| ((*n).to_owned(), (*t).to_owned()))
        .collect();
    let locals = match &args.dir {
        None => Vec::new(),
        Some(dir) => match check::read_type_files(dir) {
            Ok(files) => files,
            Err(code) => return code,
        },
    };
    let mut ok = true;
    for (name, text) in standard.iter().chain(&locals) {
        let errs = validate_unit_type(text);
        for d in &errs {
            eprintln!("{name}：{d}");
        }
        ok &= errs.is_empty();
    }
    let merged = merge_catalog(&standard, &locals);
    let catalog = match merged {
        Ok((catalog, warnings)) => {
            for d in &warnings {
                eprintln!("{d}");
            }
            catalog
        }
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };
    if !ok {
        return 1;
    }
    for t in catalog.iter() {
        println!("通過：{} version {}", t.id, t.version);
    }
    0
}

/// 以標準來源建目錄，再疊加本地來源（同 id 覆蓋並產生提示；同層重複是 U17）。
fn merge_catalog(
    standard: &[(String, String)],
    locals: &[(String, String)],
) -> machine_model::catalog::Built {
    let std_refs: Vec<(&str, &str)> = standard
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let local_refs: Vec<(&str, &str)> = locals
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    match UnitCatalog::from_sources(&std_refs) {
        Ok((base, _)) => base.with_local(&local_refs),
        Err(diags) => Err(diags),
    }
}
