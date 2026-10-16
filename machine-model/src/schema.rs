//! `machine.yaml` v2 的型別（規格 §3）。每個 struct 都拒絕未知欄位；`ext` 是唯一的擴充通道。
//!
//! Tag 沒有 `unit_id`（歸屬由父層 `bind` 宣告）、`writable`、`write_permission`
//! （最終可寫由 `PlcDataService` 決定，規格 §6.3）。

use serde::{Deserialize, Serialize};

use crate::family::Family;
use crate::strict::{Ext, StrictMap};

/// 保留的單元實例 id：機台根。
pub const MACHINE_UNIT: &str = "machine";
/// 保留的單元 id：不得宣告。
pub const UNASSIGNED_UNIT: &str = "unassigned";

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineModel {
    pub schema_version: u32,
    pub machine: Machine,
    #[serde(default)]
    pub units: Vec<UnitInstance>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: String,
    pub title: Option<String>,
    #[serde(default)]
    pub plcs: Vec<Plc>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plc {
    pub id: String,
    pub title: Option<String>,
    pub family: Family,
    #[serde(default = "default_true")]
    pub online: bool,
    /// 位址類別或區間；語法由漂移比對解析，這裡只保存。
    #[serde(default)]
    pub diff_scope: Vec<String>,
    pub sources: Option<Sources>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub io_table: Option<IoSource>,
    pub plc_export: Option<ExportSource>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoSource {
    pub file: String,
    pub sheet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSource {
    pub file: String,
    pub reader: Option<String>,
}

/// 單元實例：由 `type`（Unit 類型檔 id）、`type_version` 與 `bind` 組成（規格 §3.1）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitInstance {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// 必填，必須等於類型目錄中該類型的 `version`（U2，Plan 2）；`0` 是 `R28`。
    pub type_version: u32,
    /// 接腳名或 part 名 → 值。值是 Tag id、Unit id，或 `null`（刻意不接）。
    #[serde(default)]
    pub bind: StrictMap<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scan {
    Fast,
    Normal,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ProcessValue,
    Setpoint,
    Command,
    Mode,
    Status,
    Alarm,
    Counter,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Polarity {
    #[serde(rename = "NO")]
    NormallyOpen,
    #[serde(rename = "NC")]
    NormallyClosed,
}

/// `default` 的值。順序影響 untagged 解析：布林、整數、浮點、字串。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// 一個 PLC 位址上的點（規格 §3.2）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub address: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub symbol: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub scan: Option<Scan>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub deadband: Option<f64>,
    /// 工程單位（如 `degC`）。
    pub unit: Option<String>,
    pub readback: Option<bool>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub retain: Option<bool>,
    /// 明確覆寫用，通常省略（規格 §6.1）。
    pub role: Option<Role>,
    pub polarity: Option<Polarity>,
    pub safety_monitor: Option<bool>,
    pub spare: Option<bool>,
    #[serde(default)]
    pub ext: Ext,
}

impl Tag {
    /// 省略時為 `normal`。
    pub fn scan(&self) -> Scan {
        self.scan.unwrap_or(Scan::Normal)
    }

    /// `symbol` 省略時取 `id` 的名稱部分。
    pub fn symbol(&self) -> Option<&str> {
        self.symbol
            .as_deref()
            .or_else(|| split_id(&self.id).map(|(_, name)| name))
    }
}

/// 沒有軟元件位址的 PLC 變數。`address` 欄位只為了在有人誤填時報 `R25`。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    pub id: String,
    pub symbol: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub description: Option<String>,
    pub retain: Option<bool>,
    pub default: Option<Scalar>,
    pub address: Option<String>,
    #[serde(default)]
    pub ext: Ext,
}

/// PLC id 格式：`plc` 加不以 0 開頭的正整數。
pub fn is_valid_plc_id(id: &str) -> bool {
    let Some(n) = id.strip_prefix("plc") else {
        return false;
    };
    !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0')
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// 把 `plc1.Name` 拆成 `("plc1", "Name")`；格式不符回 `None`。
pub fn split_id(id: &str) -> Option<(&str, &str)> {
    let (plc, name) = id.split_once('.')?;
    (is_valid_plc_id(plc) && is_name(name)).then_some((plc, name))
}

/// 單元實例 id、類型 id、接腳名的字元規則：ASCII 英數與底線。
pub fn is_valid_name(s: &str) -> bool {
    is_name(s)
}

/// `bind` 值的種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindRef<'a> {
    Tag(&'a str),
    Unit(&'a str),
}

/// 含 `.` 的是 Tag id（`plc<N>.<Name>`），不含 `.` 的是 Unit id；格式不符回 `None`。
/// 不檢查 PLC 是否已宣告，也不檢查引用是否存在。
pub fn classify_bind(value: &str) -> Option<BindRef<'_>> {
    if value.contains('.') {
        split_id(value).map(|_| BindRef::Tag(value))
    } else {
        is_valid_name(value).then_some(BindRef::Unit(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_id_accepts_only_plc_prefixed_ids() {
        assert_eq!(split_id("plc1.X000"), Some(("plc1", "X000")));
        assert_eq!(
            split_id("plc12.I_LeftGripper_ClosedLimit").unwrap().0,
            "plc12"
        );
        for bad in [
            "X000",
            "plc.X000",
            "plc0.X000",
            "plc01.X",
            "plc1.",
            "plc1.a-b",
            "plc1.貼標",
            "plc1.a.b",
            ".X",
        ] {
            assert_eq!(split_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn tag_defaults() {
        let t: Tag = serde_yaml_ng::from_str("id: plc1.A\naddress: M1\ntype: bool\n").unwrap();
        assert_eq!(t.scan(), Scan::Normal);
        assert_eq!(t.symbol(), Some("A"));
    }

    #[test]
    fn tag_rejects_v1_only_fields() {
        for field in ["writable: true", "write_permission: P01", "unit_id: st1"] {
            let text = format!("id: plc1.A\naddress: M1\ntype: bool\n{field}\n");
            let err = serde_yaml_ng::from_str::<Tag>(&text)
                .unwrap_err()
                .to_string();
            assert!(err.contains("unknown field"), "{field}: {err}");
        }
    }

    #[test]
    fn unit_instance_requires_type_version_and_allows_null_bind() {
        let ok: UnitInstance = serde_yaml_ng::from_str(
            "id: cyl1\ntype: cylinder_double\ntype_version: 1\nbind: {a: plc1.A, b: null, c: st1}\n",
        )
        .unwrap();
        assert_eq!(ok.bind.0["b"], None);
        assert_eq!(ok.bind.0["a"].as_deref(), Some("plc1.A"));
        let missing = serde_yaml_ng::from_str::<UnitInstance>("id: cyl1\ntype: cylinder_double\n");
        assert!(missing.is_err());
    }

    #[test]
    fn classify_bind_distinguishes_tag_unit_and_garbage() {
        assert_eq!(classify_bind("plc1.A"), Some(BindRef::Tag("plc1.A")));
        assert_eq!(classify_bind("st1"), Some(BindRef::Unit("st1")));
        for bad in ["", "st-1", "a b", "plc1.", "x.y", "貼標"] {
            assert_eq!(classify_bind(bad), None, "{bad}");
        }
    }
}
