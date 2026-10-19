//! `write-report` 子命令：最終可寫清單、覆寫差異與 §9.3 安全護欄。
//!
//! 判定規則在 `machine_model::writable`（與 `PlcDataService` 共用）；這裡只解析服務設定並輸出報表。
//! 服務設定讀 `[[tag_override]]` 與 `[[point]]`；其餘通訊設定（IP、Port…）忽略。

use std::fmt::Write as _;

use machine_model::resolve::ResolvedModel;
use machine_model::writable::{Override, ServicePoint, WritableSource, evaluate};
use serde::Deserialize;

use crate::check;

/// `machinemodel write-report <machine.yaml> --service <service.toml>` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 機台設定檔（`machine.yaml` v2）。
    pub machine: std::path::PathBuf,
    /// 服務設定檔（含 `[[tag_override]]`）。
    #[arg(long)]
    pub service: std::path::PathBuf,
}

/// 服務設定的子集；未知的通訊欄位一律忽略（不得加 `deny_unknown_fields`）。
#[derive(Debug, Deserialize)]
struct ServiceFile {
    #[serde(default)]
    tag_override: Vec<TagOverride>,
    #[serde(default)]
    point: Vec<ServicePoint>,
}

#[derive(Debug, Clone, Deserialize)]
struct TagOverride {
    id: String,
    writable: bool,
}

/// 執行報告：0 無違規、1 護欄違規、2 I/O 或服務設定語法錯誤。
pub fn run(args: &Args) -> i32 {
    let machine_text = match std::fs::read_to_string(&args.machine) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("無法讀取 {}：{e}", args.machine.display());
            return 2;
        }
    };
    let service_text = match std::fs::read_to_string(&args.service) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("無法讀取 {}：{e}", args.service.display());
            return 2;
        }
    };
    let service: ServiceFile = match toml::from_str(&service_text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("服務設定 {} 解析失敗：{e}", args.service.display());
            return 2;
        }
    };
    let catalog = match check::catalog_beside(&args.machine) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let checked = match machine_model::load_checked(&machine_text, &catalog) {
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
    match report(&checked.resolved, &service.tag_override, &service.point) {
        Err(violations) => {
            for v in &violations {
                eprintln!("{v}");
            }
            1
        }
        Ok(report) => {
            print!("{report}");
            0
        }
    }
}

/// 套用 §9.3 護欄後輸出報表；有違規回全部違規訊息，否則回報表文字。
fn report(
    model: &ResolvedModel,
    overrides: &[TagOverride],
    points: &[ServicePoint],
) -> Result<String, Vec<String>> {
    let requests: Vec<Override<'_>> = overrides
        .iter()
        .map(|o| Override {
            tag_id: &o.id,
            writable: o.writable,
        })
        .collect();
    let result = evaluate(model, &requests, points);
    if !result.guardrail_errors.is_empty() {
        return Err(result
            .guardrail_errors
            .iter()
            .map(ToString::to_string)
            .collect());
    }

    let mut out = String::new();
    let mut table = Vec::new();
    for item in &result.items {
        match item.source {
            WritableSource::Override => {
                table.push((item.tag_id.as_str(), item.final_writable, item.source));
                if item.write_candidate != item.final_writable {
                    writeln!(
                        out,
                        "{}：模型 {} → 覆寫 {}",
                        item.tag_id, item.write_candidate, item.final_writable
                    )
                    .expect("寫入字串不會失敗");
                }
            }
            WritableSource::Model if item.write_candidate => {
                table.push((item.tag_id.as_str(), true, item.source));
            }
            WritableSource::Model => {}
            WritableSource::Service => {
                table.push((item.tag_id.as_str(), item.final_writable, item.source));
            }
        }
    }

    let mut text = String::from("最終可寫清單（來源：model＝模型推導，override＝服務覆寫）\n");
    writeln!(text, "{:<40} {:<8} source", "tag", "writable").expect("寫入字串不會失敗");
    for (id, writable, source) in &table {
        writeln!(text, "{id:<40} {writable:<8} {source}").expect("寫入字串不會失敗");
    }
    text.push_str("\n覆寫差異（覆寫改變了模型推導的結果）\n");
    if out.is_empty() {
        text.push_str("（無）\n");
    } else {
        text.push_str(&out);
    }
    Ok(text)
}
