//! Unit 類型檔（規格 §4）：型別、解析與 SC2（結構）／SC3（前綴與必要屬性）規則。
//!
//! 這裡的檢查是程式碼版；`schema` feature 之後會用 JSON Schema 再驗一次，兩者必須一致。

use serde::Deserialize;

use crate::diag::Diagnostic;
use crate::naming::{self, Prefix};
use crate::schema::{Scalar, is_valid_name};
use crate::strict::StrictMap;
use crate::types::{DataType, ScalarType};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    pub id: String,
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub field: FieldBlock,
    #[serde(default)]
    pub ctrl: StrictMap<CtrlPin>,
    /// part 名 → Unit 類型 id。
    #[serde(default)]
    pub parts: StrictMap<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBlock {
    #[serde(rename = "in", default)]
    pub input: StrictMap<FieldPin>,
    #[serde(rename = "out", default)]
    pub output: StrictMap<FieldPin>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPin {
    #[serde(rename = "type")]
    pub ty: String,
    /// `false` 表示可不接；省略視為必接。
    pub required: Option<bool>,
    pub label: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CtrlPin {
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub label: Option<String>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub message: Option<String>,
}

impl CtrlPin {
    /// 明寫的 `type`，否則該前綴的預設型別；兩者都沒有回 `None`（SC3 會擋下）。
    pub fn type_text(&self, prefix: Prefix) -> Option<&str> {
        self.ty.as_deref().or(prefix.info().default_type)
    }
}

/// 類型內一個名稱所指的東西。
#[derive(Debug, Clone, Copy)]
pub enum PinRef<'a> {
    FieldIn(&'a FieldPin),
    FieldOut(&'a FieldPin),
    Ctrl(Prefix, &'a CtrlPin),
    /// part 宣告的 Unit 類型 id。
    Part(&'a str),
}

impl UnitType {
    /// 依序在 `field.in`、`field.out`、`ctrl`、`parts` 找名稱。名稱重複（U1）時取第一個。
    pub fn lookup(&self, name: &str) -> Option<PinRef<'_>> {
        if let Some(p) = self.field.input.get(name) {
            return Some(PinRef::FieldIn(p));
        }
        if let Some(p) = self.field.output.get(name) {
            return Some(PinRef::FieldOut(p));
        }
        if let Some(p) = self.ctrl.get(name) {
            let (prefix, _) = naming::split(name)?;
            return Some(PinRef::Ctrl(prefix, p));
        }
        self.parts.get(name).map(|t| PinRef::Part(t.as_str()))
    }
}

/// 解析類型檔文字，並套用 SC2／SC3。`label` 只用在診斷位置（通常是檔名）。
pub fn parse_unit_type(label: &str, text: &str) -> Result<UnitType, Vec<Diagnostic>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let ty: UnitType = serde_yaml_ng::from_str(text).map_err(|e| {
        let detail = e.to_string();
        let message = if detail.contains("unknown field") {
            format!("未知欄位（類型檔只接受規格定義的欄位）：{detail}")
        } else {
            format!("類型檔格式或型別錯誤：{detail}")
        };
        let location = e.location().map_or_else(
            || label.to_owned(),
            |l| format!("{label} 第 {} 行第 {} 欄", l.line(), l.column()),
        );
        vec![Diagnostic::error("SC2", location, message)]
    })?;
    let diags = check_structure(&ty);
    if diags.is_empty() { Ok(ty) } else { Err(diags) }
}

/// 以 JSON Schema 再驗一次類型檔（規格 §8.2 SC2／SC3，需要 `schema` feature）。
///
/// 先跑 [`parse_unit_type`] 做重複鍵檢查（`StrictMap` 的重複鍵報錯不能靠
/// YAML→JSON 路徑補抓）；該步失敗時直接回傳那些診斷，不做 Schema 驗證。
/// 通過後把 YAML 轉成 JSON，在記憶體以 `schema_gen` 的 Schema 驗證
/// （不讀磁碟；磁碟檔僅供快照比對）。
#[cfg(feature = "schema")]
pub fn validate_unit_type(text: &str) -> Vec<Diagnostic> {
    let text = crate::strip_bom(text);
    if let Err(diags) = parse_unit_type("類型檔", text) {
        return diags;
    }
    validate_unit_type_schema(text)
}

/// 只做 YAML→JSON→Schema 驗證，不經 [`parse_unit_type`]（規格 §8.2 SC2／SC3）。
///
/// 供一致性測試使用，讓 Schema 能獨立判定；正式入口請用 [`validate_unit_type`]。
#[cfg(feature = "schema")]
pub fn validate_unit_type_schema(text: &str) -> Vec<Diagnostic> {
    let text = crate::strip_bom(text);
    let instance: serde_json::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            return vec![Diagnostic::error(
                "SC2",
                "類型檔",
                format!("類型檔轉換為 JSON 失敗：{e}"),
            )];
        }
    };
    let schema_text = crate::schema_gen::generate_unit_type_schema();
    let schema: serde_json::Value = match serde_json::from_str(&schema_text) {
        Ok(v) => v,
        Err(e) => {
            return vec![Diagnostic::error(
                "SC2",
                "類型檔",
                format!("內部 Schema 產生失敗：{e}"),
            )];
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => {
            return vec![Diagnostic::error(
                "SC2",
                "類型檔",
                format!("內部 Schema 編譯失敗：{e}"),
            )];
        }
    };
    let id = instance
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    validator
        .iter_errors(&instance)
        .map(|e| schema_error_to_diagnostic(&e, id))
        .collect()
}

#[cfg(feature = "schema")]
fn schema_error_to_diagnostic(e: &jsonschema::ValidationError<'_>, id: &str) -> Diagnostic {
    let pointer = e.instance_path().as_str().to_owned();
    let owned: Vec<String> = pointer
        .split('/')
        .skip(1)
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();
    let segs_ref: Vec<&str> = owned.iter().map(String::as_str).collect();
    let root = format!("types[{id}]");
    let (location, code) = match segs_ref.as_slice() {
        ["field", dir @ ("in" | "out"), pin, ..] => (
            format!("{root}.field.{dir}[{pin}]"),
            schema_code_for_field(e),
        ),
        ["ctrl", pin, ..] => (format!("{root}.ctrl[{pin}]"), schema_code_for_ctrl(e)),
        ["parts", part, ..] => (format!("{root}.parts[{part}]"), "SC3"),
        _ => (root, "SC2"),
    };
    Diagnostic::error(code, location, format!("Schema 驗證失敗（{pointer}）：{e}"))
}

#[cfg(feature = "schema")]
fn schema_code_for_field(e: &jsonschema::ValidationError<'_>) -> &'static str {
    let kind = format!("{:?}", e.kind());
    if kind.contains("AdditionalProperties") || kind.contains("Unknown") {
        "SC2"
    } else {
        "SC3"
    }
}

#[cfg(feature = "schema")]
fn schema_code_for_ctrl(e: &jsonschema::ValidationError<'_>) -> &'static str {
    let kind = format!("{:?}", e.kind());
    if kind.contains("UnevaluatedProperties") {
        "SC3"
    } else if kind.contains("AdditionalProperties") || kind.contains("Unknown") {
        "SC2"
    } else {
        "SC3"
    }
}

fn prefix_list() -> String {
    let names = Prefix::ALL.map(|p| p.info().name);
    names.join("、")
}

fn check_structure(t: &UnitType) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let root = format!("types[{}]", t.id);

    if !is_valid_name(&t.id) {
        out.push(Diagnostic::error(
            "SC2",
            &root,
            "類型 id 只能使用英數與底線",
        ));
    }
    if t.version == 0 {
        out.push(Diagnostic::error("SC2", &root, "version 必須是正整數"));
    }
    if t.title.trim().is_empty() {
        out.push(Diagnostic::error("SC2", &root, "title 不得為空"));
    }

    for (dir, map) in [("in", &t.field.input), ("out", &t.field.output)] {
        for (name, pin) in &map.0 {
            let loc = format!("{root}.field.{dir}[{name}]");
            if !is_valid_name(name) {
                out.push(Diagnostic::error("SC2", &loc, "接腳名稱只能使用英數與底線"));
            } else if naming::has_prefix_underscore(name) {
                out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("field 腳名稱不得以控制前綴開頭（{}）", prefix_list()),
                ));
            }
            if let Err(e) = DataType::parse(&pin.ty) {
                out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("type `{}`：{e}（field 腳只接受標量詞彙）", pin.ty),
                ));
            }
        }
    }

    for (name, pin) in &t.ctrl.0 {
        let loc = format!("{root}.ctrl[{name}]");
        if !is_valid_name(name) {
            out.push(Diagnostic::error("SC2", &loc, "接腳名稱只能使用英數與底線"));
            continue;
        }
        let Some((prefix, _)) = naming::split(name) else {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!(
                    "ctrl 腳名稱 `{name}` 必須是 <前綴>_<功能>，前綴只能是：{}",
                    prefix_list()
                ),
            ));
            continue;
        };
        match pin.type_text(prefix) {
            None => out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!("`{}_` 前綴的接腳必須明寫 type", prefix.info().name),
            )),
            Some(text) => match DataType::parse(text) {
                Err(e) => out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("type `{text}`：{e}"),
                )),
                Ok(dt) => {
                    let is_text = matches!(dt.scalar, ScalarType::Text(_)) && !dt.array;
                    if prefix == Prefix::Msg && !is_text {
                        out.push(Diagnostic::error(
                            "SC3",
                            &loc,
                            "`msg_` 接腳的 type 必須是 string:N",
                        ));
                    }
                }
            },
        }
        if prefix == Prefix::Par {
            for (field, present) in [
                ("min", pin.min.is_some()),
                ("max", pin.max.is_some()),
                ("default", pin.default.is_some()),
            ] {
                if !present {
                    out.push(Diagnostic::error(
                        "SC3",
                        &loc,
                        format!("`par_` 接腳必須有 {field}"),
                    ));
                }
            }
        }
        if prefix == Prefix::Error && pin.message.as_deref().is_none_or(|m| m.trim().is_empty()) {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                "`error_` 接腳必須有非空的 message",
            ));
        }
    }

    for (name, ty) in &t.parts.0 {
        let loc = format!("{root}.parts[{name}]");
        if !is_valid_name(name) || !is_valid_name(ty) {
            out.push(Diagnostic::error(
                "SC2",
                &loc,
                "part 名稱與類型 id 只能使用英數與底線",
            ));
        } else if DataType::parse(ty).is_ok() {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!("part 的型別必須是 Unit 類型 id，`{ty}` 是標量詞彙"),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const CYLINDER: &str = "\
id: cylinder_double
version: 1
title: 雙作用氣缸
field:
  in:
    lim_ext: { type: bool }
    lim_ret: { type: bool, required: false }
  out:
    valve_ext: { type: bool }
    valve_ret: { type: bool }
ctrl:
  cmd_ext:       { type: bool }
  cmd_ret:       { type: bool }
  ena_ext:       { type: bool }
  man_ext:       { type: bool, label: 伸出 }
  man_ret:       { type: bool, label: 縮回 }
  at_ext:        { type: bool }
  at_ret:        { type: bool }
  par_ext_time:  { type: int16, unit: ms, min: 0, max: 30000, default: 3000 }
  par_err_time:  { type: int16, unit: ms, min: 0, max: 30000, default: 5000 }
  error_ext:     { message: 伸出逾時 }
  error_ret:     { message: 縮回逾時 }
parts: {}
";

    fn codes(text: &str) -> Vec<&'static str> {
        match parse_unit_type("t.yaml", text) {
            Ok(_) => vec![],
            Err(d) => d.iter().map(|x| x.code).collect(),
        }
    }

    fn with(ctrl: &str) -> String {
        format!("id: t\nversion: 1\ntitle: T\nctrl:\n{ctrl}")
    }

    #[test]
    fn spec_example_parses() {
        let t = parse_unit_type("cylinder_double.yaml", CYLINDER).unwrap();
        assert_eq!(t.id, "cylinder_double");
        assert_eq!(t.version, 1);
        assert_eq!(t.field.input.0.len(), 2);
        assert_eq!(t.field.output.0.len(), 2);
        assert_eq!(t.ctrl.0.len(), 11);
        assert_eq!(t.field.input.get("lim_ret").unwrap().required, Some(false));
    }

    #[test]
    fn crlf_and_bom_are_accepted() {
        let text = format!("\u{feff}{}", CYLINDER.replace('\n', "\r\n"));
        assert!(parse_unit_type("t.yaml", &text).is_ok());
    }

    #[test]
    fn lookup_finds_each_kind_of_pin() {
        let t = parse_unit_type("t.yaml", CYLINDER).unwrap();
        assert!(matches!(t.lookup("lim_ext"), Some(PinRef::FieldIn(_))));
        assert!(matches!(t.lookup("valve_ext"), Some(PinRef::FieldOut(_))));
        assert!(matches!(
            t.lookup("man_ext"),
            Some(PinRef::Ctrl(Prefix::Man, _))
        ));
        assert!(t.lookup("nope").is_none());
        let with_part = CYLINDER.replace("parts: {}", "parts: {sub: other_type}");
        let t = parse_unit_type("t.yaml", &with_part).unwrap();
        assert!(matches!(t.lookup("sub"), Some(PinRef::Part("other_type"))));
    }

    #[test]
    fn type_text_uses_explicit_type_then_prefix_default() {
        let t = parse_unit_type(
            "t.yaml",
            &with("  man_a: {}\n  count_n: {}\n  par_x: { type: int16, min: 0, max: 5, default: 1 }\n"),
        )
        .unwrap();
        let pin = |n: &str| match t.lookup(n) {
            Some(PinRef::Ctrl(p, c)) => c.type_text(p).map(str::to_owned),
            _ => None,
        };
        assert_eq!(pin("man_a").as_deref(), Some("bool"));
        assert_eq!(pin("count_n").as_deref(), Some("int32"));
        assert_eq!(pin("par_x").as_deref(), Some("int16"));
    }

    #[test]
    fn sc2_structure_errors() {
        // 缺必填欄位、未知欄位（含 v1 屬性）、重複鍵、壞 YAML
        for text in [
            "id: t\nversion: 1\n".to_owned(),
            "id: t\nversion: 1\ntitle: T\nbogus: 1\n".to_owned(),
            with("  man_a: { writable: true }\n"),
            with("  man_a: { write_permission: P01 }\n"),
            with("  man_a: {}\n  man_a: {}\n"),
            "id: t\nversion: 1\ntitle: [\n".to_owned(),
            "id: 'bad id'\nversion: 1\ntitle: T\n".to_owned(),
            "id: t\nversion: 0\ntitle: T\n".to_owned(),
            "id: t\nversion: 1\ntitle: ''\n".to_owned(),
        ] {
            assert_eq!(codes(&text), ["SC2"], "{text}");
        }
    }

    #[test]
    fn removed_pin_attributes_are_unknown_fields() {
        // 規格 §4.1 修訂：接腳不再有 kind／level／reset／role（Tag 層級的 role 不在此列）
        for attr in [
            "man_a: { type: bool, kind: momentary }",
            "error_a: { message: x, level: emergency }",
            "error_a: { message: x, reset: manual }",
            "man_a: { type: bool, role: status }",
        ] {
            assert_eq!(codes(&with(&format!("  {attr}\n"))), ["SC2"], "{attr}");
        }
    }

    #[test]
    fn sc3_prefix_rules() {
        // 名稱不符前綴表
        assert_eq!(codes(&with("  lim_ext: { type: bool }\n")), ["SC3"]);
        assert_eq!(codes(&with("  par: {}\n")), ["SC3"]);
        // 需要 type 的前綴
        assert_eq!(codes(&with("  pv_t: {}\n")), ["SC3"]);
        assert_eq!(codes(&with("  msg_m: {}\n")), ["SC3"]);
        // par 需要 min、max、default
        assert_eq!(
            codes(&with("  par_x: { type: int16 }\n")),
            ["SC3", "SC3", "SC3"]
        );
        assert_eq!(
            codes(&with("  par_x: { type: int16, min: 0, max: 5 }\n")),
            ["SC3"]
        );
        // error 需要 message
        assert_eq!(codes(&with("  error_x: {}\n")), ["SC3"]);
        assert_eq!(codes(&with("  error_x: { message: '' }\n")), ["SC3"]);
        // msg 必須是 string:N
        assert_eq!(codes(&with("  msg_m: { type: int16 }\n")), ["SC3"]);
        assert_eq!(
            codes(&with("  msg_m: { type: 'string:16' }\n")),
            Vec::<&str>::new()
        );
        // 型別詞彙
        assert_eq!(codes(&with("  man_a: { type: integer }\n")), ["SC3"]);
        assert_eq!(codes(&with("  pv_t: { type: 'string:0' }\n")), ["SC3"]);
    }

    #[test]
    fn sc3_field_and_part_rules() {
        // field 腳名稱不得用控制前綴
        let bad_name = "id: t\nversion: 1\ntitle: T\nfield:\n  in:\n    man_x: { type: bool }\n";
        assert_eq!(codes(bad_name), ["SC3"]);
        // field 型別必須是標量詞彙
        let bad_type = "id: t\nversion: 1\ntitle: T\nfield:\n  out:\n    v: { type: other_unit }\n";
        assert_eq!(codes(bad_type), ["SC3"]);
        // part 的型別是 Unit 類型 id，不能是標量
        let scalar_part = "id: t\nversion: 1\ntitle: T\nparts:\n  p: bool\n";
        assert_eq!(codes(scalar_part), ["SC3"]);
        let bad_part_name = "id: t\nversion: 1\ntitle: T\nparts:\n  'p-1': other\n";
        assert_eq!(codes(bad_part_name), ["SC2"]);
    }

    #[test]
    fn diagnostics_are_traditional_chinese_with_location() {
        let d = parse_unit_type("cyl.yaml", &with("  lim_ext: {}\n")).unwrap_err();
        let text = d[0].to_string();
        assert!(text.starts_with("[SC3] types[t].ctrl[lim_ext]："), "{text}");
        assert!(text.contains("前綴"), "{text}");
        let d = parse_unit_type("cyl.yaml", "id: t\nbogus: 1\n").unwrap_err();
        assert!(d[0].location.as_deref().unwrap().contains("cyl.yaml"));
    }
}
