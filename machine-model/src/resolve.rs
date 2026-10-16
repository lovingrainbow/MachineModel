//! 推導與解析結果（規格 §6、§9.2）：把 `machine.yaml` 與類型目錄變成下游直接使用的資料。
//!
//! 這裡只輸出模型層的事實（`role`、`write_candidate`…）。最終 `writable` 與人員權限
//! 不在本專案（規格 §6.3）。

use std::collections::BTreeMap;

use crate::address;
use crate::catalog::UnitCatalog;
use crate::family::Family;
use crate::hash::{content_hash, hex};
use crate::naming::{self, BindPolicy, Prefix};
use crate::schema::{
    BindRef, MachineModel, Polarity, Role, Scalar, Scan, Tag, Variable, classify_bind, split_id,
};
use crate::strict::Ext;
use crate::unit_type::{CtrlPin, Kind, Level, PinRef, Reset};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModel {
    pub machine_id: String,
    pub plcs: Vec<ResolvedPlc>,
    pub units: Vec<ResolvedUnit>,
    pub tags: Vec<ResolvedTag>,
    pub variables: Vec<Variable>,
    pub pins: Vec<ResolvedPin>,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPlc {
    pub id: String,
    pub family: Family,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUnit {
    pub id: String,
    pub ty: String,
    pub type_version: u32,
    pub parent: Option<String>,
    /// 由 `machine` 起以 `/` 連接，例如 `machine/st1/cyl1`。
    pub path: String,
    pub bind: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleSource {
    /// Tag 明確寫了 `role`。
    Explicit,
    /// 由綁定的 ctrl 腳的前綴推測。
    Pin(Prefix),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagBinding {
    pub unit_id: String,
    pub unit_path: String,
    pub pin: String,
    /// `None` 表示綁在 field 腳。
    pub prefix: Option<Prefix>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTag {
    pub id: String,
    pub address: String,
    pub ty: String,
    pub symbol: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub scan: Scan,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub deadband: Option<f64>,
    pub unit: Option<String>,
    pub readback: Option<bool>,
    pub retain: Option<bool>,
    pub safety_monitor: bool,
    pub spare: bool,
    pub polarity: Option<Polarity>,
    pub ext: Ext,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub role: Option<Role>,
    pub role_source: RoleSource,
    /// 規格 §6.2：若綁定的接腳中有任何一個為 `man_`／`par_` 腳才是 `true`；沒綁定一律 `false`（fail-closed）。
    pub write_candidate: bool,
    /// 綁定此 Tag 的所有接腳清單（規格 §9.2）。
    pub bindings: Vec<TagBinding>,
    /// 位址類別是否為唯讀輸入；位址無法解析時為 `false`。
    pub class_read_only: bool,
}

impl ResolvedTag {
    /// 取得第一個綁定（若有）。
    pub fn binding(&self) -> Option<&TagBinding> {
        self.bindings.first()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPin {
    pub unit_id: String,
    pub unit_path: String,
    pub pin: String,
    pub prefix: Prefix,
    pub function: String,
    pub label: String,
    pub ty: String,
    pub tag: Option<String>,
    pub kind: Option<Kind>,
    pub level: Option<Level>,
    pub reset: Option<Reset>,
    pub message: Option<String>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// `machine.yaml` 的內容雜湊（忽略換行差異）。
    pub source_hash: String,
    /// 實際用到的類型 id → version。
    pub type_versions: BTreeMap<String, u32>,
}

impl ResolvedModel {
    pub fn tag(&self, id: &str) -> Option<&ResolvedTag> {
        self.tags.iter().find(|t| t.id == id)
    }

    fn with_prefix(&self, wanted: &'static [Prefix]) -> impl Iterator<Item = &ResolvedPin> {
        self.pins.iter().filter(move |p| wanted.contains(&p.prefix))
    }

    pub fn operations(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Man])
    }

    pub fn alarms(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Error])
    }

    pub fn parameters(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Par])
    }

    pub fn values(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Pv, Prefix::Count])
    }

    pub fn messages(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Msg])
    }

    /// 規格 U16：沒綁定的選擇性 ctrl 腳（純接線的 `cmd_`／`ena_` 不算）。
    pub fn coverage_gaps(&self) -> Vec<&ResolvedPin> {
        self.pins
            .iter()
            .filter(|p| p.tag.is_none() && p.prefix.info().bind == BindPolicy::Optional)
            .collect()
    }
}

/// 一個 Tag 被哪個接腳綁定。
struct Binder<'a> {
    unit_id: &'a str,
    pin: &'a str,
    prefix: Option<Prefix>,
    ctrl: Option<&'a CtrlPin>,
}

pub fn resolve(model: &MachineModel, catalog: &UnitCatalog, source_text: &str) -> ResolvedModel {
    // 父層：part 綁的子單元 → 父單元 id（取第一個）
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    // Tag id → 綁定它的所有接腳清單
    let mut binders: BTreeMap<&str, Vec<Binder<'_>>> = BTreeMap::new();
    let mut type_versions: BTreeMap<String, u32> = BTreeMap::new();

    for u in &model.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue;
        };
        type_versions.insert(ty.id.clone(), ty.version);
        for (key, value) in &u.bind.0 {
            let (Some(pin), Some(value)) = (ty.lookup(key), value.as_deref()) else {
                continue;
            };
            match (pin, classify_bind(value)) {
                (PinRef::Part(_), Some(BindRef::Unit(child))) => {
                    parent.entry(child).or_insert(u.id.as_str());
                }
                (PinRef::FieldIn(_) | PinRef::FieldOut(_), Some(BindRef::Tag(id))) => {
                    binders.entry(id).or_default().push(Binder {
                        unit_id: &u.id,
                        pin: key,
                        prefix: None,
                        ctrl: None,
                    });
                }
                (PinRef::Ctrl(prefix, ctrl), Some(BindRef::Tag(id))) => {
                    binders.entry(id).or_default().push(Binder {
                        unit_id: &u.id,
                        pin: key,
                        prefix: Some(prefix),
                        ctrl: Some(ctrl),
                    });
                }
                _ => {}
            }
        }
    }

    let path_of = |id: &str| -> String {
        let mut parts = vec![id];
        let mut cur = id;
        for _ in 0..model.units.len() {
            let Some(&p) = parent.get(cur) else {
                break;
            };
            parts.push(p);
            cur = p;
        }
        parts.reverse();
        parts.join("/")
    };

    let units = model
        .units
        .iter()
        .map(|u| ResolvedUnit {
            id: u.id.clone(),
            ty: u.ty.clone(),
            type_version: u.type_version,
            parent: parent.get(u.id.as_str()).map(|p| (*p).to_owned()),
            path: path_of(&u.id),
            bind: u.bind.0.clone(),
        })
        .collect();

    let plcs: BTreeMap<&str, &crate::schema::Plc> = model
        .machine
        .plcs
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let tags = model
        .tags
        .iter()
        .map(|t| {
            let bs = binders
                .get(t.id.as_str())
                .map_or(&[] as &[_], Vec::as_slice);
            resolve_tag(t, bs, &plcs, &path_of)
        })
        .collect();

    ResolvedModel {
        machine_id: model.machine.id.clone(),
        plcs: model
            .machine
            .plcs
            .iter()
            .map(|p| ResolvedPlc {
                id: p.id.clone(),
                family: p.family,
                online: p.online,
            })
            .collect(),
        units,
        tags,
        variables: model.variables.clone(),
        pins: resolve_pins(model, catalog, &path_of),
        fingerprint: Fingerprint {
            source_hash: hex(content_hash(source_text)),
            type_versions,
        },
    }
}

fn resolve_tag(
    t: &Tag,
    tag_binders: &[Binder<'_>],
    plcs: &BTreeMap<&str, &crate::schema::Plc>,
    path_of: &dyn Fn(&str) -> String,
) -> ResolvedTag {
    let pin_role = tag_binders.iter().find_map(|b| {
        let prefix = b.prefix?;
        Some((
            b.ctrl.and_then(|c| c.role).unwrap_or(prefix.info().role),
            prefix,
        ))
    });
    let (role, role_source) = match (t.role, pin_role) {
        (Some(r), _) => (Some(r), RoleSource::Explicit),
        (None, Some((r, prefix))) => (Some(r), RoleSource::Pin(prefix)),
        (None, None) => (None, RoleSource::Unknown),
    };
    let write_candidate = tag_binders
        .iter()
        .any(|b| b.prefix.is_some_and(|p| p.info().write_candidate));
    let pin = tag_binders.iter().find_map(|b| b.ctrl);
    let class_read_only = split_id(&t.id)
        .and_then(|(plc, _)| plcs.get(plc))
        .and_then(|plc| address::parse(plc.family, &t.address).ok())
        .is_some_and(|a| a.class.read_only);

    let bindings = tag_binders
        .iter()
        .map(|b| TagBinding {
            unit_id: b.unit_id.to_owned(),
            unit_path: path_of(b.unit_id),
            pin: b.pin.to_owned(),
            prefix: b.prefix,
        })
        .collect();

    ResolvedTag {
        id: t.id.clone(),
        address: t.address.clone(),
        ty: t.ty.clone(),
        symbol: t.symbol().map(str::to_owned),
        label: t.label.clone(),
        description: t.description.clone(),
        scan: t.scan(),
        scale: t.scale,
        offset: t.offset,
        deadband: t.deadband,
        unit: t.unit.clone().or_else(|| pin.and_then(|p| p.unit.clone())),
        readback: t.readback,
        retain: t.retain,
        safety_monitor: t.safety_monitor.unwrap_or(false),
        spare: t.spare.unwrap_or(false),
        polarity: t.polarity,
        ext: t.ext.clone(),
        min: t.min.or_else(|| pin.and_then(|p| p.min)),
        max: t.max.or_else(|| pin.and_then(|p| p.max)),
        default: t
            .default
            .clone()
            .or_else(|| pin.and_then(|p| p.default.clone())),
        role,
        role_source,
        write_candidate,
        bindings,
        class_read_only,
    }
}

fn resolve_pins(
    model: &MachineModel,
    catalog: &UnitCatalog,
    path_of: &dyn Fn(&str) -> String,
) -> Vec<ResolvedPin> {
    let mut out = Vec::new();
    for u in &model.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue;
        };
        for (name, pin) in &ty.ctrl.0 {
            let Some((prefix, rest)) = naming::split(name) else {
                continue;
            };
            let function = naming::function_of(rest);
            let tag = match u.bind.get(name) {
                Some(Some(v)) => match classify_bind(v) {
                    Some(BindRef::Tag(id)) => Some(id.to_owned()),
                    _ => None,
                },
                _ => None,
            };
            let is_error = prefix == Prefix::Error;
            out.push(ResolvedPin {
                unit_id: u.id.clone(),
                unit_path: path_of(&u.id),
                pin: name.clone(),
                prefix,
                function: function.to_owned(),
                label: pin
                    .label
                    .clone()
                    .or_else(|| naming::default_label(function).map(str::to_owned))
                    .unwrap_or_else(|| function.to_owned()),
                ty: pin.type_text(prefix).unwrap_or_default().to_owned(),
                tag,
                kind: pin
                    .kind
                    .or((prefix == Prefix::Man).then_some(Kind::Momentary)),
                level: pin.level.or(is_error.then_some(Level::Abnormal)),
                reset: pin.reset.or(is_error.then_some(Reset::Manual)),
                message: pin.message.clone(),
                unit: pin.unit.clone(),
                min: pin.min,
                max: pin.max,
                default: pin.default.clone(),
            });
        }
    }
    out
}
