//! `machine.yaml` v2（Unit／Tag 模型）的型別、解析與載入安全規則。
//!
//! 執行期（`PlcDataService`）與設計期工具共用本 crate。規格見專案根目錄
//! `2026-10-16-unit-tag-model-design.md`。

pub mod address;
pub mod analyze;
pub mod catalog;
pub mod diag;
pub mod family;
pub mod hash;
pub mod naming;
pub mod resolve;
pub mod rules;
pub mod schema;
#[cfg(feature = "schema")]
pub mod schema_gen;
pub mod st_check;
pub mod standard;
pub mod strict;
mod type_rules;
pub mod types;
pub mod unit_type;

use diag::Diagnostic;
use schema::MachineModel;
use serde::Deserialize;

/// 本 crate 支援的 `schema_version`。
pub const SCHEMA_VERSION: u32 = 2;

#[derive(Deserialize)]
struct Probe {
    schema_version: Option<u32>,
}

pub(crate) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// 解析 `machine.yaml` v2 文字。
///
/// - 開頭的 UTF-8 BOM（Windows 編輯器常加）會被忽略。
/// - `schema_version` 不是 [`SCHEMA_VERSION`] 時回 `R27`，不再往下解析。
/// - 未知欄位回 `R22`；其他語法或型別錯誤回 `PARSE`。
pub fn parse(text: &str) -> Result<MachineModel, Diagnostic> {
    let text = strip_bom(text);

    let declared = serde_yaml_ng::from_str::<Probe>(text)
        .ok()
        .and_then(|p| p.schema_version);
    if let Some(v) = declared {
        if v != SCHEMA_VERSION {
            return Err(rules::version_error(v, SCHEMA_VERSION));
        }
    }

    serde_yaml_ng::from_str::<MachineModel>(text).map_err(|e| {
        let detail = e.to_string();
        // R22 的判定依賴 serde 的英文用語；由 R22 相關測試釘住。
        let (code, message) = if detail.contains("unknown field") {
            (
                "R22",
                format!("未知欄位（只接受規格定義的欄位，擴充請放在 ext）：{detail}"),
            )
        } else {
            ("PARSE", format!("YAML 格式或型別錯誤：{detail}"))
        };
        let location = e.location().map_or_else(
            || "文件".to_owned(),
            |l| format!("第 {} 行第 {} 欄", l.line(), l.column()),
        );
        Diagnostic::error(code, location, message)
    })
}

/// 對已解析的模型執行全部載入安全規則。
pub fn validate(model: &MachineModel) -> Vec<Diagnostic> {
    rules::run(model)
}

/// 解析並驗證。沒有任何錯誤時回模型；否則回全部診斷。
pub fn load(text: &str) -> Result<MachineModel, Vec<Diagnostic>> {
    let model = parse(text).map_err(|d| vec![d])?;
    let diagnostics = validate(&model);
    if diagnostics.iter().any(Diagnostic::is_error) {
        Err(diagnostics)
    } else {
        Ok(model)
    }
}

/// [`load_checked`] 的成功結果。
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub resolved: resolve::ResolvedModel,
    /// `Warning` 與 `Info` 診斷（沒有 `Error`）。
    pub warnings: Vec<Diagnostic>,
}

/// 完整載入：載入安全規則 → 類型目錄上的 U 規則 → 推導。
///
/// 載入安全規則有錯時只回那些錯誤，不再執行 U 規則（避免連鎖雜訊）。
pub fn load_checked(
    text: &str,
    catalog: &catalog::UnitCatalog,
) -> Result<Checked, Vec<Diagnostic>> {
    let model = load(text)?;
    let diagnostics = analyze::check(&model, catalog);
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(diagnostics);
    }
    Ok(Checked {
        resolved: resolve::resolve(&model, catalog, text),
        warnings: diagnostics,
    })
}
