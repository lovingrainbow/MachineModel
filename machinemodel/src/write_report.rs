//! `write-report` 子命令：最終可寫清單、覆寫差異與 §9.3 安全護欄。
//!
//! 服務設定只讀 `[[tag_override]]` 子集（`id`、`writable`），其餘通訊設定（IP、Port…）忽略。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use machine_model::resolve::ResolvedModel;
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

/// 服務設定的子集；未知欄位一律忽略（不得加 `deny_unknown_fields`）。
#[derive(Debug, Deserialize)]
struct ServiceFile {
    #[serde(default)]
    tag_override: Vec<TagOverride>,
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
    match evaluate(&checked.resolved, &service.tag_override) {
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

/// 檢查 §9.3 的四項護欄；有違規回全部違規訊息，否則回報表文字。
fn evaluate(model: &ResolvedModel, overrides: &[TagOverride]) -> Result<String, Vec<String>> {
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();
    for o in overrides {
        if !seen.insert(o.id.as_str()) {
            violations.push(format!("護欄：覆寫重複指定同一個 Tag `{}`", o.id));
        }
    }

    // tag id → (模型推導的可寫性, 覆寫後的可寫性)
    let mut applied: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
    for o in overrides {
        let Some(tag) = model.tag(&o.id) else {
            violations.push(format!("護欄 3：覆寫指向不存在的 Tag `{}`", o.id));
            continue;
        };
        if o.writable && tag.class_read_only {
            violations.push(format!("護欄 1：唯讀類別的 Tag `{}` 不可覆寫為可寫", o.id));
        }
        if o.writable && tag.safety_monitor {
            violations.push(format!("護欄 2：安全監控點 `{}` 不可覆寫為可寫", o.id));
        }
        applied.insert(tag.id.as_str(), (tag.write_candidate, o.writable));
    }
    if !violations.is_empty() {
        return Err(violations);
    }

    let mut out = String::new();
    let mut table = Vec::new();
    for tag in &model.tags {
        if let Some(&(model_writable, final_writable)) = applied.get(tag.id.as_str()) {
            table.push((tag.id.as_str(), final_writable, "override"));
            if model_writable != final_writable {
                writeln!(
                    out,
                    "{}：模型 {} → 覆寫 {}",
                    tag.id, model_writable, final_writable
                )
                .expect("寫入字串不會失敗");
            }
        } else if tag.write_candidate {
            table.push((tag.id.as_str(), true, "model"));
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
