//! Unit 類型目錄（規格 §9.1）：載入多份類型檔、疊加本地類型、套用類型層規則。

use std::collections::BTreeMap;

use crate::diag::Diagnostic;
use crate::type_rules;
use crate::unit_type::{UnitType, parse_unit_type};

/// 成功時附帶警告與提示（沒有錯誤）；失敗時是全部診斷。
pub type Built = Result<(UnitCatalog, Vec<Diagnostic>), Vec<Diagnostic>>;

#[derive(Debug, Clone, PartialEq)]
pub struct UnitCatalog {
    types: BTreeMap<String, UnitType>,
}

/// 解析一層來源；同層 id 重複是 `U17`。
fn parse_layer(sources: &[(&str, &str)], out: &mut Vec<Diagnostic>) -> Vec<UnitType> {
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    let mut layer = Vec::new();
    for (label, text) in sources {
        match parse_unit_type(label, text) {
            Err(d) => out.extend(d),
            Ok(t) => {
                if let Some(first) = seen.insert(t.id.clone(), label) {
                    out.push(Diagnostic::error(
                        "U17",
                        format!("types[{}]", t.id),
                        format!("類型 id `{}` 在 `{first}` 與 `{label}` 重複", t.id),
                    ));
                } else {
                    layer.push(t);
                }
            }
        }
    }
    layer
}

fn assemble(base: Vec<UnitType>, local: Vec<UnitType>, mut diags: Vec<Diagnostic>) -> Built {
    let mut types = BTreeMap::new();
    for t in base {
        types.insert(t.id.clone(), t);
    }
    for t in local {
        let id = t.id.clone();
        if types.insert(id.clone(), t).is_some() {
            diags.push(Diagnostic::info(
                "U17",
                format!("types[{id}]"),
                "本地類型覆蓋同 id 的標準類型（實例的 type_version 必須對應本地版本）",
            ));
        }
    }
    diags.extend(type_rules::check(&types));
    if diags.iter().any(Diagnostic::is_error) {
        Err(diags)
    } else {
        Ok((UnitCatalog { types }, diags))
    }
}

impl UnitCatalog {
    pub fn get(&self, id: &str) -> Option<&UnitType> {
        self.types.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &UnitType> {
        self.types.values()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// 由 `(來源名稱, 類型檔文字)` 建目錄。
    pub fn from_sources(sources: &[(&str, &str)]) -> Built {
        let mut diags = Vec::new();
        let layer = parse_layer(sources, &mut diags);
        assemble(layer, Vec::new(), diags)
    }

    /// 疊加機台本地類型；同 id 覆蓋並產生 `Info`。合併後的目錄整體重新檢查。
    pub fn with_local(self, locals: &[(&str, &str)]) -> Built {
        let mut diags = Vec::new();
        let local = parse_layer(locals, &mut diags);
        assemble(self.types.into_values().collect(), local, diags)
    }
}
