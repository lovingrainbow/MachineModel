use std::collections::BTreeMap;

use machine_model::Checked;
use machine_model::catalog::UnitCatalog;
use machine_model::diag::Diagnostic;
use machine_model::family::Family;
use machine_model::schema::{Polarity, Scalar, Scan};
use machine_model::unit_type::PinRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlcConfig {
    pub id: String,
    pub family: Family,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub type_version: u32,
    #[serde(default)]
    pub bind: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagConfig {
    pub id: String,
    pub address: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polarity: Option<Polarity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_monitor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan: Option<Scan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Scalar>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Scalar>,
}

#[derive(Debug, Clone)]
pub struct ProjectModel {
    #[allow(dead_code)]
    pub schema_version: u32,
    pub machine_id: String,
    pub machine_title: Option<String>,
    pub plcs: Vec<PlcConfig>,
    pub units: Vec<UnitConfig>,
    pub tags: Vec<TagConfig>,
    pub variables: Vec<VariableConfig>,
    pub catalog: UnitCatalog,
    pub custom_type_sources: Vec<(String, String)>,
}

impl Default for ProjectModel {
    fn default() -> Self {
        Self::demo()
    }
}

impl ProjectModel {
    /// 建立全新空白專案。
    pub fn new_empty(machine_id: &str, machine_title: Option<String>, family: Family) -> Self {
        let catalog = UnitCatalog::standard();
        let plcs = vec![PlcConfig {
            id: "plc1".to_string(),
            family,
            online: true,
        }];
        Self {
            schema_version: 2,
            machine_id: machine_id.to_string(),
            machine_title,
            plcs,
            units: Vec::new(),
            tags: Vec::new(),
            variables: Vec::new(),
            catalog,
            custom_type_sources: Vec::new(),
        }
    }

    /// 建立示範專案（對應 machine-model valid_units.yaml 規格範例）。
    pub fn demo() -> Self {
        let catalog = UnitCatalog::standard();

        let plcs = vec![PlcConfig {
            id: "plc1".to_string(),
            family: Family::InovanceEasy,
            online: true,
        }];

        let mut machine_bind = BTreeMap::new();
        machine_bind.insert("stn".to_string(), Some("st1".to_string()));
        machine_bind.insert("error_estop".to_string(), Some("plc1.M_Estop".to_string()));
        machine_bind.insert("man_mode".to_string(), Some("plc1.D_ModeCmd".to_string()));
        machine_bind.insert("at_mode".to_string(), Some("plc1.D_ModeAct".to_string()));

        let mut st1_bind = BTreeMap::new();
        st1_bind.insert("cyl".to_string(), Some("cyl1".to_string()));

        let mut cyl1_bind = BTreeMap::new();
        cyl1_bind.insert(
            "lim_ext".to_string(),
            Some("plc1.X_Cyl1_Ext_LS".to_string()),
        );
        cyl1_bind.insert(
            "lim_ret".to_string(),
            Some("plc1.X_Cyl1_Ret_LS".to_string()),
        );
        cyl1_bind.insert("valve_ext".to_string(), Some("plc1.Y_Cyl1_Ext".to_string()));
        cyl1_bind.insert("valve_ret".to_string(), Some("plc1.Y_Cyl1_Ret".to_string()));
        cyl1_bind.insert(
            "man_ext".to_string(),
            Some("plc1.M_Cyl1_ManExt".to_string()),
        );
        cyl1_bind.insert(
            "man_ret".to_string(),
            Some("plc1.M_Cyl1_ManRet".to_string()),
        );
        cyl1_bind.insert(
            "error_ext".to_string(),
            Some("plc1.M_Cyl1_ErrExt".to_string()),
        );
        cyl1_bind.insert(
            "error_ret".to_string(),
            Some("plc1.M_Cyl1_ErrRet".to_string()),
        );
        cyl1_bind.insert(
            "par_ext_time".to_string(),
            Some("plc1.D_Cyl1_ExtTime".to_string()),
        );

        let units = vec![
            UnitConfig {
                id: "machine".to_string(),
                ty: "machine_ctrl".to_string(),
                type_version: 2,
                bind: machine_bind,
            },
            UnitConfig {
                id: "st1".to_string(),
                ty: "station_load".to_string(),
                type_version: 1,
                bind: st1_bind,
            },
            UnitConfig {
                id: "cyl1".to_string(),
                ty: "cylinder_double".to_string(),
                type_version: 1,
                bind: cyl1_bind,
            },
        ];

        let tags = vec![
            TagConfig {
                id: "plc1.X_Cyl1_Ext_LS".to_string(),
                address: "X10".to_string(),
                ty: "bool".to_string(),
                label: Some("氣缸伸出極限感測器".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.X_Cyl1_Ret_LS".to_string(),
                address: "X11".to_string(),
                ty: "bool".to_string(),
                label: Some("氣缸縮回極限感測器".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.Y_Cyl1_Ext".to_string(),
                address: "Y10".to_string(),
                ty: "bool".to_string(),
                label: Some("電磁閥伸出線圈".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.Y_Cyl1_Ret".to_string(),
                address: "Y11".to_string(),
                ty: "bool".to_string(),
                label: Some("電磁閥縮回線圈".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Cyl1_ManExt".to_string(),
                address: "M100".to_string(),
                ty: "bool".to_string(),
                label: Some("手動伸出按鈕".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Cyl1_ManRet".to_string(),
                address: "M101".to_string(),
                ty: "bool".to_string(),
                label: Some("手動縮回按鈕".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Cyl1_ErrExt".to_string(),
                address: "M110".to_string(),
                ty: "bool".to_string(),
                label: Some("伸出逾時警報".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Cyl1_ErrRet".to_string(),
                address: "M111".to_string(),
                ty: "bool".to_string(),
                label: Some("縮回逾時警報".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.D_Cyl1_ExtTime".to_string(),
                address: "D100".to_string(),
                ty: "int16".to_string(),
                label: Some("伸出逾時參數".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: Some("ms".to_string()),
                min: Some(0.0),
                max: Some(30000.0),
                default: Some(Scalar::Int(3000)),
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Estop".to_string(),
                address: "M120".to_string(),
                ty: "bool".to_string(),
                label: Some("緊急停止開關".to_string()),
                description: None,
                polarity: None,
                safety_monitor: Some(true),
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.D_ModeCmd".to_string(),
                address: "D200".to_string(),
                ty: "int16".to_string(),
                label: Some("機台模式指令".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.D_ModeAct".to_string(),
                address: "D201".to_string(),
                ty: "int16".to_string(),
                label: Some("機台當前模式".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: None,
            },
            TagConfig {
                id: "plc1.M_Spare".to_string(),
                address: "M130".to_string(),
                ty: "bool".to_string(),
                label: Some("預留點".to_string()),
                description: None,
                polarity: None,
                safety_monitor: None,
                scan: None,
                unit: None,
                min: None,
                max: None,
                default: None,
                spare: Some(true),
            },
        ];

        Self {
            schema_version: 2,
            machine_id: "demo".to_string(),
            machine_title: Some("示範產線機台".to_string()),
            plcs,
            units,
            tags,
            variables: Vec::new(),
            catalog,
            custom_type_sources: Vec::new(),
        }
    }

    /// 序列化為合法的 machine.yaml v2 規格文字。
    pub fn to_yaml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("schema_version: 2\n");
        out.push_str("machine:\n");
        out.push_str(&format!("  id: {}\n", self.machine_id));
        if let Some(t) = self.machine_title.as_deref().filter(|s| !s.is_empty()) {
            out.push_str(&format!("  title: {}\n", t));
        }
        out.push_str("  plcs:\n");
        for p in &self.plcs {
            out.push_str(&format!(
                "    - {{ id: {}, family: {}{}}}\n",
                p.id,
                p.family.as_str(),
                if p.online { "" } else { ", online: false" }
            ));
        }

        out.push_str("units:\n");
        for u in &self.units {
            out.push_str(&format!("  - id: {}\n", u.id));
            out.push_str(&format!("    type: {}\n", u.ty));
            out.push_str(&format!("    type_version: {}\n", u.type_version));
            if !u.bind.is_empty() {
                out.push_str("    bind:\n");
                for (k, v) in &u.bind {
                    match v {
                        Some(val) => out.push_str(&format!("      {}: {}\n", k, val)),
                        None => out.push_str(&format!("      {}: null\n", k)),
                    }
                }
            }
        }

        out.push_str("tags:\n");
        for t in &self.tags {
            let mut parts = vec![
                format!("id: {}", t.id),
                format!("address: {}", t.address),
                format!("type: {}", t.ty),
            ];
            if let Some(ref lbl) = t.label {
                parts.push(format!("label: {}", escape_yaml_str(lbl)));
            }
            if let Some(ref desc) = t.description {
                parts.push(format!("description: {}", escape_yaml_str(desc)));
            }
            if let Some(ref pol) = t.polarity {
                match pol {
                    Polarity::NormallyOpen => parts.push("polarity: NO".to_string()),
                    Polarity::NormallyClosed => parts.push("polarity: NC".to_string()),
                }
            }
            if let Some(true) = t.safety_monitor {
                parts.push("safety_monitor: true".to_string());
            }
            if let Some(ref scan) = t.scan {
                let s = match scan {
                    Scan::Fast => "fast",
                    Scan::Normal => "normal",
                    Scan::Slow => "slow",
                };
                parts.push(format!("scan: {}", s));
            }
            if let Some(ref u) = t.unit {
                parts.push(format!("unit: {}", u));
            }
            if let Some(m) = t.min {
                parts.push(format!("min: {}", m));
            }
            if let Some(m) = t.max {
                parts.push(format!("max: {}", m));
            }
            if let Some(ref def) = t.default {
                match def {
                    Scalar::Bool(b) => parts.push(format!("default: {}", b)),
                    Scalar::Int(i) => parts.push(format!("default: {}", i)),
                    Scalar::Float(f) => parts.push(format!("default: {}", f)),
                    Scalar::Text(s) => parts.push(format!("default: {}", escape_yaml_str(s))),
                }
            }
            if let Some(true) = t.spare {
                parts.push("spare: true".to_string());
            }

            out.push_str(&format!("  - {{ {} }}\n", parts.join(", ")));
        }

        if !self.variables.is_empty() {
            out.push_str("variables:\n");
            for v in &self.variables {
                let mut parts = vec![format!("id: {}", v.id), format!("type: {}", v.ty)];
                if let Some(ref desc) = v.description {
                    parts.push(format!("description: {}", escape_yaml_str(desc)));
                }
                if let Some(ref def) = v.default {
                    match def {
                        Scalar::Bool(b) => parts.push(format!("default: {}", b)),
                        Scalar::Int(i) => parts.push(format!("default: {}", i)),
                        Scalar::Float(f) => parts.push(format!("default: {}", f)),
                        Scalar::Text(s) => parts.push(format!("default: {}", escape_yaml_str(s))),
                    }
                }
                out.push_str(&format!("  - {{ {} }}\n", parts.join(", ")));
            }
        }

        out
    }

    /// 從 YAML 文字反向載入專案。
    #[allow(dead_code)]
    pub fn from_yaml_str(yaml: &str, catalog: UnitCatalog) -> Result<Self, Vec<Diagnostic>> {
        let raw = machine_model::parse(yaml).map_err(|d| vec![d])?;
        let plcs = raw
            .machine
            .plcs
            .into_iter()
            .map(|p| PlcConfig {
                id: p.id,
                family: p.family,
                online: p.online,
            })
            .collect();

        let units = raw
            .units
            .into_iter()
            .map(|u| UnitConfig {
                id: u.id,
                ty: u.ty,
                type_version: u.type_version,
                bind: u.bind.0,
            })
            .collect();

        let tags = raw
            .tags
            .into_iter()
            .map(|t| TagConfig {
                id: t.id,
                address: t.address,
                ty: t.ty,
                label: t.label,
                description: t.description,
                polarity: t.polarity,
                safety_monitor: t.safety_monitor,
                scan: t.scan,
                unit: t.unit,
                min: t.min,
                max: t.max,
                default: t.default,
                spare: t.spare,
            })
            .collect();

        let variables = raw
            .variables
            .into_iter()
            .map(|v| VariableConfig {
                id: v.id,
                ty: v.ty,
                description: v.description,
                default: v.default,
            })
            .collect();

        Ok(Self {
            schema_version: raw.schema_version,
            machine_id: raw.machine.id,
            machine_title: raw.machine.title,
            plcs,
            units,
            tags,
            variables,
            catalog,
            custom_type_sources: Vec::new(),
        })
    }

    /// 執行實時驗證並回傳 (診斷結果, YAML 文字)。
    pub fn validate(&self) -> (Result<Checked, Vec<Diagnostic>>, String) {
        let yaml = self.to_yaml_string();
        let checked = machine_model::load_checked(&yaml, &self.catalog);
        (checked, yaml)
    }

    /// 新增 Unit 並根據類型目錄自動填充空白的接腳與 part 插槽。
    pub fn add_unit(&mut self, id: String, ty: String, type_version: u32) {
        let mut bind = BTreeMap::new();
        if let Some(ut) = self.catalog.get(&ty) {
            for pin_name in ut.field.input.0.keys() {
                bind.insert(pin_name.clone(), None);
            }
            for pin_name in ut.field.output.0.keys() {
                bind.insert(pin_name.clone(), None);
            }
            for pin_name in ut.ctrl.0.keys() {
                bind.insert(pin_name.clone(), None);
            }
            for part_name in ut.parts.0.keys() {
                bind.insert(part_name.clone(), None);
            }
        }
        self.units.push(UnitConfig {
            id,
            ty,
            type_version,
            bind,
        });
    }

    /// 刪除 Unit，並自動清除其他 Unit 中引用此 Unit 的 part 綁定。
    pub fn delete_unit(&mut self, unit_id: &str) {
        self.units.retain(|u| u.id != unit_id);
        for u in &mut self.units {
            for target in u.bind.values_mut() {
                if target.as_deref() == Some(unit_id) {
                    *target = None;
                }
            }
        }
    }

    /// 重新命名 Unit，並同步更新其他 Unit 的 part 引用。
    pub fn rename_unit(&mut self, old_id: &str, new_id: &str) {
        for u in &mut self.units {
            if u.id == old_id {
                u.id = new_id.to_string();
            }
            for target in u.bind.values_mut() {
                if target.as_deref() == Some(old_id) {
                    *target = Some(new_id.to_string());
                }
            }
        }
    }

    /// 刪除 Tag，並同步解除任何 Unit 綁定到此 Tag 的插槽。
    pub fn delete_tag(&mut self, tag_id: &str) {
        self.tags.retain(|t| t.id != tag_id);
        for u in &mut self.units {
            for target in u.bind.values_mut() {
                if target.as_deref() == Some(tag_id) {
                    *target = None;
                }
            }
        }
    }

    /// 查詢某個 Tag 被哪些 Unit 的哪些插槽綁定。
    pub fn tag_bound_to(&self, tag_id: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for u in &self.units {
            for (slot, target) in &u.bind {
                if target.as_deref() == Some(tag_id) {
                    out.push((u.id.clone(), slot.clone()));
                }
            }
        }
        out
    }

    /// 查詢某個 Unit 是否被作為某父層 Unit 的 part 綁定。
    pub fn unit_parent(&self, unit_id: &str) -> Option<(String, String)> {
        for u in &self.units {
            for (slot, target) in &u.bind {
                if target.as_deref() == Some(unit_id) {
                    return Some((u.id.clone(), slot.clone()));
                }
            }
        }
        None
    }

    /// 推算下一個推薦的可用 PLC 位址（八進位或十進位）。
    pub fn next_available_address(&self, prefix: &str) -> String {
        let is_octal = prefix == "X" || prefix == "Y";
        let mut max_num = -1i64;

        for t in &self.tags {
            if let Some(rest) = t.address.strip_prefix(prefix) {
                let num_str = rest.split('.').next().unwrap_or(rest);
                if is_octal {
                    if let Some(n) = i64::from_str_radix(num_str, 8)
                        .ok()
                        .filter(|&n| n > max_num)
                    {
                        max_num = n;
                    }
                } else if let Some(n) = num_str.parse::<i64>().ok().filter(|&n| n > max_num) {
                    max_num = n;
                }
            }
        }

        let next = if max_num < 0 {
            match prefix {
                "X" | "Y" => 0,
                "M" => 100,
                "D" => 100,
                _ => 0,
            }
        } else {
            max_num + 1
        };

        if is_octal {
            format!("{}{:o}", prefix, next)
        } else {
            format!("{}{}", prefix, next)
        }
    }

    /// 載入自訂 Unit Type YAML。
    pub fn load_custom_type(
        &mut self,
        source_name: &str,
        yaml_text: &str,
    ) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        let type_id = machine_model::unit_type::parse_unit_type(source_name, yaml_text)
            .map(|t| t.id)
            .unwrap_or_else(|_| source_name.trim_end_matches(".yaml").to_string());
        self.save_custom_type(&type_id, yaml_text)
    }

    /// 判斷指定類型是否為核心內嵌之標準類型
    pub fn is_standard_type(&self, id: &str) -> bool {
        UnitCatalog::standard().get(id).is_some()
    }

    /// 取得自訂類型的原始 YAML 文字
    pub fn get_custom_type_yaml(&self, id: &str) -> Option<&str> {
        let alt = format!("{}.yaml", id);
        self.custom_type_sources
            .iter()
            .find(|(s, _)| s == id || s == &alt)
            .map(|(_, y)| y.as_str())
    }

    /// 取得目前所有在專案中使用了此 Unit 類型的單元清單
    pub fn units_using_type(&self, type_id: &str) -> Vec<String> {
        self.units
            .iter()
            .filter(|u| u.ty == type_id)
            .map(|u| u.id.clone())
            .collect()
    }

    /// 新增或更新自訂 Unit 類型
    pub fn save_custom_type(
        &mut self,
        type_id: &str,
        yaml_text: &str,
    ) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
        // 先以核心 parse_unit_type 進行嚴格規格檢核
        let _parsed = machine_model::unit_type::parse_unit_type(type_id, yaml_text)?;

        let source_key = format!("{}.yaml", type_id);
        if let Some(pos) = self
            .custom_type_sources
            .iter()
            .position(|(s, _)| s == &source_key || s == type_id)
        {
            self.custom_type_sources[pos] = (source_key, yaml_text.to_string());
        } else {
            self.custom_type_sources
                .push((source_key, yaml_text.to_string()));
        }

        let sources_refs: Vec<(&str, &str)> = self
            .custom_type_sources
            .iter()
            .map(|(s, y)| (s.as_str(), y.as_str()))
            .collect();
        let (new_catalog, diags) = UnitCatalog::standard().with_local(&sources_refs)?;
        self.catalog = new_catalog;
        Ok(diags)
    }

    /// 刪除自訂 Unit 類型
    pub fn delete_custom_type(&mut self, type_id: &str) -> Result<(), String> {
        if self.is_standard_type(type_id) {
            return Err("核心標準類型不可刪除".to_string());
        }
        let in_use = self.units_using_type(type_id);
        if !in_use.is_empty() {
            return Err(format!(
                "無法刪除：專案中有 Unit 正在使用此類型（{}）",
                in_use.join(", ")
            ));
        }

        let source_key = format!("{}.yaml", type_id);
        let pos = self
            .custom_type_sources
            .iter()
            .position(|(s, _)| s == &source_key || s == type_id)
            .ok_or_else(|| format!("找不到自訂類型 `{}`", type_id))?;
        self.custom_type_sources.remove(pos);

        let sources_refs: Vec<(&str, &str)> = self
            .custom_type_sources
            .iter()
            .map(|(s, y)| (s.as_str(), y.as_str()))
            .collect();
        match UnitCatalog::standard().with_local(&sources_refs) {
            Ok((new_catalog, _)) => {
                self.catalog = new_catalog;
                Ok(())
            }
            Err(diags) => {
                let msgs: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
                Err(msgs.join("; "))
            }
        }
    }

    /// 取得插槽的元資料（類型、必填、說明等）。
    #[allow(dead_code)]
    pub fn slot_info(&self, unit_type_id: &str, slot_name: &str) -> Option<SlotInspection> {
        let ut = self.catalog.get(unit_type_id)?;
        let pref = ut.lookup(slot_name)?;
        match pref {
            PinRef::FieldIn(pin) => Some(SlotInspection {
                category: SlotCategory::FieldIn,
                expected_type: pin.ty.clone(),
                required: pin.required.unwrap_or(true),
                label: pin.label.clone(),
                unit: pin.unit.clone(),
                min: None,
                max: None,
                message: None,
                part_type: None,
            }),
            PinRef::FieldOut(pin) => Some(SlotInspection {
                category: SlotCategory::FieldOut,
                expected_type: pin.ty.clone(),
                required: pin.required.unwrap_or(true),
                label: pin.label.clone(),
                unit: pin.unit.clone(),
                min: None,
                max: None,
                message: None,
                part_type: None,
            }),
            PinRef::Ctrl(prefix, ctrl) => Some(SlotInspection {
                category: SlotCategory::Ctrl(prefix),
                expected_type: ctrl.type_text(prefix).unwrap_or("bool").to_string(),
                required: true,
                label: ctrl.label.clone(),
                unit: ctrl.unit.clone(),
                min: ctrl.min,
                max: ctrl.max,
                message: ctrl.message.clone(),
                part_type: None,
            }),
            PinRef::Part(target_type) => Some(SlotInspection {
                category: SlotCategory::Part,
                expected_type: target_type.to_string(),
                required: true,
                label: None,
                unit: None,
                min: None,
                max: None,
                message: None,
                part_type: Some(target_type.to_string()),
            }),
        }
    }
}

pub fn escape_yaml_str(s: &str) -> String {
    if s.contains(':')
        || s.contains('#')
        || s.contains('{')
        || s.contains('}')
        || s.contains('[')
        || s.contains(']')
        || s.contains(',')
        || s.contains('\n')
    {
        serde_json::to_string(s).unwrap_or_else(|_| s.to_string())
    } else {
        s.to_string()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotCategory {
    FieldIn,
    FieldOut,
    Ctrl(machine_model::naming::Prefix),
    Part,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SlotInspection {
    pub category: SlotCategory,
    pub expected_type: String,
    pub required: bool,
    pub label: Option<String>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub message: Option<String>,
    pub part_type: Option<String>,
}
