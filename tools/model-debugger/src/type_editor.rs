//! Unit 類型庫編輯器 (Unit Type Editor)
//!
//! 提供視覺化表單與 YAML 原始碼雙向編輯，支援新建、編輯、複製、刪除自訂 Unit 類型，
//! 並即時呼叫 machine_model 核心的 SC2 / SC3 規範檢核。

use machine_model::diag::Diagnostic;
use machine_model::naming;
use machine_model::schema::Scalar;
use machine_model::unit_type::{UnitType, parse_unit_type};

use crate::model::escape_yaml_str;

/// 編輯中暫存的 Unit 類型草稿
#[derive(Debug, Clone, PartialEq)]
pub struct UnitTypeDraft {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub field_in: Vec<FieldPinDraft>,
    pub field_out: Vec<FieldPinDraft>,
    pub ctrl: Vec<CtrlPinDraft>,
    pub parts: Vec<PartSlotDraft>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPinDraft {
    pub name: String,
    pub ty: String,
    pub required: bool,
    pub label: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CtrlPinDraft {
    pub prefix: String,
    pub suffix: String,
    pub ty: String,
    pub label: String,
    pub unit: String,
    pub min: String,
    pub max: String,
    pub default_val: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartSlotDraft {
    pub slot_name: String,
    pub target_type: String,
}

impl Default for UnitTypeDraft {
    fn default() -> Self {
        Self::new_empty("new_unit_type")
    }
}

impl UnitTypeDraft {
    pub fn new_empty(id: &str) -> Self {
        Self {
            id: id.to_string(),
            version: 1,
            title: "新單元類型".to_string(),
            field_in: vec![FieldPinDraft {
                name: "sens1".to_string(),
                ty: "bool".to_string(),
                required: true,
                label: "感應器".to_string(),
                unit: String::new(),
            }],
            field_out: vec![FieldPinDraft {
                name: "act1".to_string(),
                ty: "bool".to_string(),
                required: true,
                label: "致動器".to_string(),
                unit: String::new(),
            }],
            ctrl: vec![
                CtrlPinDraft {
                    prefix: "man".to_string(),
                    suffix: "trigger".to_string(),
                    ty: "bool".to_string(),
                    label: "手動觸發".to_string(),
                    unit: String::new(),
                    min: String::new(),
                    max: String::new(),
                    default_val: String::new(),
                    message: String::new(),
                },
                CtrlPinDraft {
                    prefix: "at".to_string(),
                    suffix: "done".to_string(),
                    ty: "bool".to_string(),
                    label: "完成狀態".to_string(),
                    unit: String::new(),
                    min: String::new(),
                    max: String::new(),
                    default_val: String::new(),
                    message: String::new(),
                },
                CtrlPinDraft {
                    prefix: "error".to_string(),
                    suffix: "timeout".to_string(),
                    ty: "bool".to_string(),
                    label: "動作逾時".to_string(),
                    unit: String::new(),
                    min: String::new(),
                    max: String::new(),
                    default_val: String::new(),
                    message: "單元動作逾時異常".to_string(),
                },
            ],
            parts: Vec::new(),
        }
    }

    /// 從現有的 UnitType 實例轉成草稿
    pub fn from_unit_type(ut: &UnitType) -> Self {
        let mut field_in = Vec::new();
        for (name, pin) in &ut.field.input.0 {
            field_in.push(FieldPinDraft {
                name: name.clone(),
                ty: pin.ty.clone(),
                required: pin.required.unwrap_or(true),
                label: pin.label.clone().unwrap_or_default(),
                unit: pin.unit.clone().unwrap_or_default(),
            });
        }

        let mut field_out = Vec::new();
        for (name, pin) in &ut.field.output.0 {
            field_out.push(FieldPinDraft {
                name: name.clone(),
                ty: pin.ty.clone(),
                required: pin.required.unwrap_or(true),
                label: pin.label.clone().unwrap_or_default(),
                unit: pin.unit.clone().unwrap_or_default(),
            });
        }

        let mut ctrl = Vec::new();
        for (name, pin) in &ut.ctrl.0 {
            if let Some((prefix, suffix)) = naming::split(name) {
                let ty_str = pin.type_text(prefix).unwrap_or("bool").to_string();
                let default_str = match &pin.default {
                    Some(Scalar::Bool(b)) => b.to_string(),
                    Some(Scalar::Int(i)) => i.to_string(),
                    Some(Scalar::Float(f)) => f.to_string(),
                    Some(Scalar::Text(s)) => s.clone(),
                    None => String::new(),
                };

                ctrl.push(CtrlPinDraft {
                    prefix: prefix.info().name.to_string(),
                    suffix: suffix.to_string(),
                    ty: ty_str,
                    label: pin.label.clone().unwrap_or_default(),
                    unit: pin.unit.clone().unwrap_or_default(),
                    min: pin.min.map(|v| v.to_string()).unwrap_or_default(),
                    max: pin.max.map(|v| v.to_string()).unwrap_or_default(),
                    default_val: default_str,
                    message: pin.message.clone().unwrap_or_default(),
                });
            }
        }

        let mut parts = Vec::new();
        for (name, target) in &ut.parts.0 {
            parts.push(PartSlotDraft {
                slot_name: name.clone(),
                target_type: target.clone(),
            });
        }

        Self {
            id: ut.id.clone(),
            version: ut.version,
            title: ut.title.clone(),
            field_in,
            field_out,
            ctrl,
            parts,
        }
    }

    /// 從 YAML 文字解析並轉換為草稿
    pub fn from_yaml_str(text: &str) -> Result<Self, Vec<Diagnostic>> {
        let ut = parse_unit_type("draft", text)?;
        Ok(Self::from_unit_type(&ut))
    }

    /// 輸出為嚴格符合 machine.yaml v2 標準之 Unit 類型 YAML
    pub fn to_yaml_string(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("id: {}\n", self.id.trim()));
        out.push_str(&format!("version: {}\n", self.version.max(1)));
        out.push_str(&format!("title: {}\n", escape_yaml_str(self.title.trim())));

        // field 區塊
        if !self.field_in.is_empty() || !self.field_out.is_empty() {
            out.push_str("field:\n");
            if !self.field_in.is_empty() {
                out.push_str("  in:\n");
                for p in &self.field_in {
                    let name = p.name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    let mut props = vec![format!("type: {}", p.ty.trim())];
                    if !p.required {
                        props.push("required: false".to_string());
                    }
                    if !p.label.trim().is_empty() {
                        props.push(format!("label: {}", escape_yaml_str(p.label.trim())));
                    }
                    if !p.unit.trim().is_empty() {
                        props.push(format!("unit: {}", p.unit.trim()));
                    }
                    out.push_str(&format!("    {}: {{ {} }}\n", name, props.join(", ")));
                }
            }
            if !self.field_out.is_empty() {
                out.push_str("  out:\n");
                for p in &self.field_out {
                    let name = p.name.trim();
                    if name.is_empty() {
                        continue;
                    }
                    let mut props = vec![format!("type: {}", p.ty.trim())];
                    if !p.required {
                        props.push("required: false".to_string());
                    }
                    if !p.label.trim().is_empty() {
                        props.push(format!("label: {}", escape_yaml_str(p.label.trim())));
                    }
                    if !p.unit.trim().is_empty() {
                        props.push(format!("unit: {}", p.unit.trim()));
                    }
                    out.push_str(&format!("    {}: {{ {} }}\n", name, props.join(", ")));
                }
            }
        }

        // ctrl 區塊
        if !self.ctrl.is_empty() {
            out.push_str("ctrl:\n");
            for p in &self.ctrl {
                let s = p.suffix.trim();
                if s.is_empty() {
                    continue;
                }
                let prefix = p.prefix.trim();
                let full_name = format!("{}_{}", prefix, s);
                let mut props = Vec::new();

                if prefix == "par" {
                    let ty = if p.ty.trim().is_empty() {
                        "int16"
                    } else {
                        p.ty.trim()
                    };
                    props.push(format!("type: {}", ty));
                    if !p.unit.trim().is_empty() {
                        props.push(format!("unit: {}", p.unit.trim()));
                    }
                    if let Ok(min_f) = p.min.trim().parse::<f64>() {
                        props.push(format!("min: {}", min_f));
                    }
                    if let Ok(max_f) = p.max.trim().parse::<f64>() {
                        props.push(format!("max: {}", max_f));
                    }
                    if !p.default_val.trim().is_empty() {
                        props.push(format!("default: {}", p.default_val.trim()));
                    }
                } else if prefix == "error" {
                    let msg = if p.message.trim().is_empty() {
                        "警報"
                    } else {
                        p.message.trim()
                    };
                    props.push(format!("message: {}", escape_yaml_str(msg)));
                } else if prefix == "pv" || prefix == "msg" {
                    let ty = if p.ty.trim().is_empty() {
                        if prefix == "pv" {
                            "float32"
                        } else {
                            "string:32"
                        }
                    } else {
                        p.ty.trim()
                    };
                    props.push(format!("type: {}", ty));
                    if !p.unit.trim().is_empty() {
                        props.push(format!("unit: {}", p.unit.trim()));
                    }
                } else {
                    if !p.ty.trim().is_empty() && p.ty.trim() != "bool" {
                        props.push(format!("type: {}", p.ty.trim()));
                    }
                    if !p.unit.trim().is_empty() {
                        props.push(format!("unit: {}", p.unit.trim()));
                    }
                }

                if !p.label.trim().is_empty() {
                    props.push(format!("label: {}", escape_yaml_str(p.label.trim())));
                }

                if props.is_empty() {
                    out.push_str(&format!("  {}: {{}}\n", full_name));
                } else {
                    out.push_str(&format!("  {}: {{ {} }}\n", full_name, props.join(", ")));
                }
            }
        }

        // parts 區塊
        if !self.parts.is_empty() {
            out.push_str("parts:\n");
            for part in &self.parts {
                let s = part.slot_name.trim();
                let t = part.target_type.trim();
                if !s.is_empty() && !t.is_empty() {
                    out.push_str(&format!("  {}: {}\n", s, t));
                }
            }
        } else {
            out.push_str("parts: {}\n");
        }

        out
    }

    /// 呼叫 machine_model 核心進行嚴格 SC2 / SC3 驗證
    pub fn validate(&self) -> Result<UnitType, Vec<Diagnostic>> {
        let yaml = self.to_yaml_string();
        parse_unit_type(&self.id, &yaml)
    }
}

/// 編輯器當前檢視模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeEditTab {
    VisualForm,
    RawYaml,
}

/// 類型編輯彈窗的互動狀態
#[derive(Debug, Clone)]
pub struct TypeEditorModalState {
    pub is_open: bool,
    pub is_new: bool,
    pub original_id: String,
    pub draft: UnitTypeDraft,
    pub raw_yaml: String,
    pub current_tab: TypeEditTab,
    pub validation_errors: Vec<Diagnostic>,
    pub save_error: Option<String>,
}

impl Default for TypeEditorModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_new: true,
            original_id: String::new(),
            draft: UnitTypeDraft::default(),
            raw_yaml: String::new(),
            current_tab: TypeEditTab::VisualForm,
            validation_errors: Vec::new(),
            save_error: None,
        }
    }
}

impl TypeEditorModalState {
    pub fn open_new(&mut self) {
        self.is_open = true;
        self.is_new = true;
        self.original_id = String::new();
        self.draft = UnitTypeDraft::default();
        self.raw_yaml = self.draft.to_yaml_string();
        self.current_tab = TypeEditTab::VisualForm;
        self.validation_errors = Vec::new();
        self.save_error = None;
    }

    pub fn open_edit(&mut self, ut: &UnitType) {
        self.is_open = true;
        self.is_new = false;
        self.original_id = ut.id.clone();
        self.draft = UnitTypeDraft::from_unit_type(ut);
        self.raw_yaml = self.draft.to_yaml_string();
        self.current_tab = TypeEditTab::VisualForm;
        self.validation_errors = Vec::new();
        self.save_error = None;
    }

    pub fn open_clone(&mut self, ut: &UnitType) {
        self.is_open = true;
        self.is_new = true;
        self.original_id = String::new();
        let mut draft = UnitTypeDraft::from_unit_type(ut);
        draft.id = format!("{}_custom", ut.id);
        draft.title = format!("{} (自訂複製版)", ut.title);
        draft.version = 1;
        self.draft = draft;
        self.raw_yaml = self.draft.to_yaml_string();
        self.current_tab = TypeEditTab::VisualForm;
        self.validation_errors = Vec::new();
        self.save_error = None;
    }

    /// 同步視覺表單至 YAML 字串
    pub fn sync_form_to_yaml(&mut self) {
        self.raw_yaml = self.draft.to_yaml_string();
        self.validate();
    }

    /// 從 YAML 字串重新同步至視覺表單
    pub fn sync_yaml_to_form(&mut self) -> Result<(), Vec<Diagnostic>> {
        match UnitTypeDraft::from_yaml_str(&self.raw_yaml) {
            Ok(d) => {
                self.draft = d;
                self.validation_errors.clear();
                self.save_error = None;
                Ok(())
            }
            Err(diags) => {
                self.validation_errors = diags.clone();
                Err(diags)
            }
        }
    }

    /// 執行即時驗證
    pub fn validate(&mut self) {
        match self.draft.validate() {
            Ok(_) => {
                self.validation_errors.clear();
            }
            Err(diags) => {
                self.validation_errors = diags;
            }
        }
    }
}
