//! 最終 `writable` 判定（規格 §9.3）：覆寫護欄 DS1～DS4、服務點位 DS5～DS11。
//!
//! 模型只輸出 `write_candidate`；最終狀態由 `PlcDataService` 持有（規格 §6.3）。本模組只放判定規則，
//! 讓 `machinemodel write-report` 與 `PlcDataService` 共用同一份。`service.toml` 的解析留在呼叫端，
//! 所以本模組不依賴 `toml`；服務點位的結構由 serde 直接反序列化成 [`ServicePoint`]。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::address;
use crate::diag::Diagnostic;
use crate::resolve::{ResolvedModel, ResolvedPlc, ResolvedTag};
use crate::rules::{Occupancy, collisions, occupancy, type_fits};
use crate::schema::{Scan, split_id};
use crate::types::DataType;

/// 一筆覆寫請求（`service.toml` 的 `[[tag_override]]`，由呼叫端解析後傳入）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Override<'a> {
    pub tag_id: &'a str,
    pub writable: bool,
}

/// `service.toml` 的 `[[point]]`：不在機台模型中的服務點位。
///
/// 未知欄位一律拒絕；安全監控點只能在機台模型中宣告，因此沒有 `safety_monitor` 欄位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServicePoint {
    /// 格式同 `machine.yaml`：`<plc>.<名稱>`。
    pub id: String,
    pub address: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// 預設 `false`（fail-closed）。
    #[serde(default)]
    pub writable: bool,
    /// 省略時為 `normal`。
    #[serde(default)]
    pub scan: Option<Scan>,
    /// `writable = true` 時必填。
    #[serde(default)]
    pub min: Option<f64>,
    /// `writable = true` 時必填。
    #[serde(default)]
    pub max: Option<f64>,
}

impl ServicePoint {
    /// 實際使用的掃描速率（省略時為 `normal`）。
    pub fn scan_rate(&self) -> Scan {
        self.scan.unwrap_or(Scan::Normal)
    }
}

/// 最終 `writable` 的來源（§9.3 護欄 4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritableSource {
    Model,
    Override,
    Service,
}

impl fmt::Display for WritableSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WritableSource::Model => write!(f, "model"),
            WritableSource::Override => write!(f, "override"),
            WritableSource::Service => write!(f, "service"),
        }
    }
}

/// 單一點位的最終可寫決策。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalWritableItem {
    pub tag_id: String,
    pub address: String,
    pub write_candidate: bool,
    pub final_writable: bool,
    pub source: WritableSource,
    /// 「最終可寫但不是模型候選」的點位，供設計期人工審查（§9.3）。
    pub requires_human_review: bool,
}

/// 判定結果。違規的覆寫與服務點位不套用，違規回報在 `guardrail_errors`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub items: Vec<FinalWritableItem>,
    /// 列出「最終可寫但不是模型候選 (`final_writable && !write_candidate`)」的點位 ID。
    pub review_required_tags: Vec<String>,
    pub guardrail_errors: Vec<Diagnostic>,
}

/// 依模型、覆寫請求與服務點位計算最終可寫性。
///
/// 覆寫：
/// - DS1：唯讀位址類別的點位不可覆寫為可寫。
/// - DS2：`safety_monitor: true` 的點位不可覆寫為可寫。
/// - DS3：覆寫引用的點位必須存在於模型中。
/// - DS4：同一點位出現多次時，所有出現都不套用（不靜默取最後一筆）。
/// - DS11：覆寫不能指向服務點位（服務點位的可寫性只由它自己的 `writable` 決定）。
///
/// 服務點位（DS5～DS10、DS1、DS9）見 [`check_points`]。
pub fn evaluate(
    model: &ResolvedModel,
    overrides: &[Override<'_>],
    points: &[ServicePoint],
) -> WriteReport {
    let mut guardrail_errors = Vec::new();
    let tags: BTreeMap<&str, &ResolvedTag> =
        model.tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let point_ids: BTreeSet<&str> = points.iter().map(|p| p.id.as_str()).collect();

    let valid_points = check_points(model, points, &mut guardrail_errors);

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for ov in overrides {
        *counts.entry(ov.tag_id).or_insert(0) += 1;
    }
    let duplicated: BTreeSet<&str> = counts
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, _)| *id)
        .collect();
    for id in &duplicated {
        guardrail_errors.push(Diagnostic::error(
            "DS4",
            format!("tag_override[{id}]"),
            format!("覆寫重複：Tag '{id}' 被指定多次，該 Tag 的所有覆寫都不套用"),
        ));
    }

    let mut applied: BTreeMap<&str, bool> = BTreeMap::new();
    for (idx, ov) in overrides.iter().enumerate() {
        if duplicated.contains(ov.tag_id) {
            continue;
        }
        let location = format!("tag_override[{idx}]({})", ov.tag_id);
        if point_ids.contains(ov.tag_id) {
            guardrail_errors.push(Diagnostic::error(
                "DS11",
                location,
                format!(
                    "護欄 5：'{}' 是服務點位，覆寫不適用；請直接設定該服務點位的 writable",
                    ov.tag_id
                ),
            ));
            continue;
        }
        let Some(tag) = tags.get(ov.tag_id) else {
            guardrail_errors.push(Diagnostic::error(
                "DS3",
                location,
                format!("護欄 3：覆寫引用了模型中不存在的 Tag '{}'", ov.tag_id),
            ));
            continue;
        };
        if ov.writable && tag.class_read_only {
            guardrail_errors.push(Diagnostic::error(
                "DS1",
                location,
                format!(
                    "護欄 1：Tag '{}' 位於唯讀輸入位址類別（{}），不得覆寫為 writable = true",
                    ov.tag_id, tag.address
                ),
            ));
            continue;
        }
        if ov.writable && tag.safety_monitor {
            guardrail_errors.push(Diagnostic::error(
                "DS2",
                location,
                format!(
                    "護欄 2：Tag '{}' 為 safety_monitor: true 的安全監控點，不得覆寫為 writable = true",
                    ov.tag_id
                ),
            ));
            continue;
        }
        applied.insert(ov.tag_id, ov.writable);
    }

    let mut items = Vec::with_capacity(model.tags.len() + valid_points.len());
    let mut review_required_tags = Vec::new();
    for tag in &model.tags {
        let (final_writable, source) = match applied.get(tag.id.as_str()) {
            Some(&writable) => (writable, WritableSource::Override),
            None => (tag.write_candidate, WritableSource::Model),
        };
        let requires_human_review = final_writable && !tag.write_candidate;
        if requires_human_review {
            review_required_tags.push(tag.id.clone());
        }
        items.push(FinalWritableItem {
            tag_id: tag.id.clone(),
            address: tag.address.clone(),
            write_candidate: tag.write_candidate,
            final_writable,
            source,
            requires_human_review,
        });
    }
    for &idx in &valid_points {
        let point = &points[idx];
        let requires_human_review = point.writable;
        if requires_human_review {
            review_required_tags.push(point.id.clone());
        }
        items.push(FinalWritableItem {
            tag_id: point.id.clone(),
            address: point.address.clone(),
            write_candidate: false,
            final_writable: point.writable,
            source: WritableSource::Service,
            requires_human_review,
        });
    }

    WriteReport {
        items,
        review_required_tags,
        guardrail_errors,
    }
}

/// 一個點位在某 PLC 某位址類別上的佔用（模型點位或服務點位）。
struct Placement<'a> {
    plc: &'a str,
    class: &'static str,
    id: &'a str,
    address: &'a str,
    occ: Occupancy,
    /// 服務點位的索引；模型點位為 `None`。
    point: Option<usize>,
}

/// 驗證服務點位，回傳通過驗證（且無重疊）的點位索引。
///
/// - DS5：id 與機台模型或其他服務點位重複；
/// - DS6：id 不是 `<plc>.<名稱>`，或 PLC 不在模型中；
/// - DS7：位址語法錯、超出家族範圍、位址類別為非空語法，或型別與位址類別不相容；
/// - DS8：型別不存在；
/// - DS1：唯讀位址類別的點位設為 `writable = true`；
/// - DS10：`writable = true` 卻沒有 `min` 與 `max`；
/// - DS9：位址與模型點位或其他服務點位重疊（同 PLC、同位址類別，判定同 R8）；服務點位之間依宣告順序，先宣告者保留，被拒的點位不佔位址。
fn check_points(
    model: &ResolvedModel,
    points: &[ServicePoint],
    errors: &mut Vec<Diagnostic>,
) -> Vec<usize> {
    let plcs: BTreeMap<&str, &ResolvedPlc> =
        model.plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    let model_ids: BTreeSet<&str> = model.tags.iter().map(|t| t.id.as_str()).collect();

    let mut placements: Vec<Placement<'_>> = Vec::new();
    for tag in &model.tags {
        if let Some(p) = tag_placement(&plcs, tag) {
            placements.push(p);
        }
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut candidates: Vec<Placement<'_>> = Vec::new();
    for (idx, p) in points.iter().enumerate() {
        let loc = format!("service.toml:point[{idx}]({})", p.id);
        if model_ids.contains(p.id.as_str()) || !seen.insert(p.id.as_str()) {
            errors.push(Diagnostic::error(
                "DS5",
                loc,
                format!("服務點位 id '{}' 與機台模型或其他服務點位重複", p.id),
            ));
            continue;
        }
        let Some((plc_id, _)) = split_id(&p.id) else {
            errors.push(Diagnostic::error(
                "DS6",
                loc,
                format!("服務點位 id '{}' 必須是 <plc>.<名稱> 格式", p.id),
            ));
            continue;
        };
        let Some(plc) = plcs.get(plc_id) else {
            errors.push(Diagnostic::error(
                "DS6",
                loc,
                format!("服務點位 '{}' 引用的 PLC '{plc_id}' 不在機台模型中", p.id),
            ));
            continue;
        };
        let addr = match address::parse(plc.family, &p.address) {
            Ok(a) => a,
            Err(e) => {
                errors.push(Diagnostic::error(
                    "DS7",
                    loc,
                    format!("服務點位位址 `{}`：{e}", p.address),
                ));
                continue;
            }
        };
        let class = addr.class;
        if class.opaque {
            errors.push(Diagnostic::error(
                "DS7",
                loc,
                format!(
                    "服務點位位址 `{}` 沒有可辨識的位址類別（非空語法不可用於服務點位）",
                    p.address
                ),
            ));
            continue;
        }
        let ty = match DataType::parse(&p.ty) {
            Ok(t) => t,
            Err(e) => {
                errors.push(Diagnostic::error(
                    "DS8",
                    loc,
                    format!("服務點位 '{}' 的型別 '{}' 不存在：{e}", p.id, p.ty),
                ));
                continue;
            }
        };
        if let Err(msg) = type_fits(class, &addr, &ty) {
            errors.push(Diagnostic::error("DS7", loc, msg));
            continue;
        }
        let occ = match occupancy(class, &addr, &ty) {
            Ok(o) => o,
            Err(msg) => {
                errors.push(Diagnostic::error("DS7", loc, msg));
                continue;
            }
        };
        if p.writable && class.read_only {
            errors.push(Diagnostic::error(
                "DS1",
                loc,
                format!(
                    "護欄 1：服務點位 '{}' 位於唯讀輸入位址類別（{}），不得設為 writable = true",
                    p.id, p.address
                ),
            ));
            continue;
        }
        if p.writable && (p.min.is_none() || p.max.is_none()) {
            errors.push(Diagnostic::error(
                "DS10",
                loc,
                format!(
                    "服務點位 '{}' 設為 writable = true 時必須同時設定 min 與 max",
                    p.id
                ),
            ));
            continue;
        }
        candidates.push(Placement {
            plc: plc_id,
            class: class.name,
            id: &p.id,
            address: &p.address,
            occ,
            point: Some(idx),
        });
    }

    // DS9：依宣告順序逐一加入；被拒絕的點位不佔位址，不會連帶擋住後面的點位。
    let mut valid: Vec<usize> = Vec::new();
    for cand in candidates {
        let mut group: Vec<&Placement<'_>> = placements
            .iter()
            .filter(|q| q.plc == cand.plc && q.class == cand.class)
            .collect();
        group.push(&cand);
        let me = group.len() - 1;
        let items: Vec<(&str, Occupancy)> = group.iter().map(|q| (q.id, q.occ)).collect();
        let mut clash = false;
        for (later, earlier) in collisions(&items) {
            let other = if later == me {
                group[earlier]
            } else if earlier == me {
                group[later]
            } else {
                continue;
            };
            clash = true;
            let Some(pi) = cand.point else { continue };
            errors.push(Diagnostic::error(
                "DS9",
                format!("service.toml:point[{pi}]({})", cand.id),
                format!(
                    "{} 的 {} 類別位址 `{}` 與 `{}`（{}）重疊",
                    cand.plc, cand.class, cand.address, other.id, other.address
                ),
            ));
        }
        if !clash {
            if let Some(pi) = cand.point {
                valid.push(pi);
            }
            placements.push(cand);
        }
    }
    valid
}

/// 模型點位的佔用；位址為空、語法不通過或類別為非空語法時回傳 `None`（由 R18、R19 等負責回報）。
fn tag_placement<'a>(
    plcs: &BTreeMap<&str, &ResolvedPlc>,
    tag: &'a ResolvedTag,
) -> Option<Placement<'a>> {
    if tag.address.is_empty() {
        return None;
    }
    let (plc_id, _) = split_id(&tag.id)?;
    let plc = plcs.get(plc_id)?;
    let addr = address::parse(plc.family, &tag.address).ok()?;
    if addr.class.opaque {
        return None;
    }
    let ty = DataType::parse(&tag.ty).ok()?;
    let occ = occupancy(addr.class, &addr, &ty).ok()?;
    Some(Placement {
        plc: plc_id,
        class: addr.class.name,
        id: &tag.id,
        address: &tag.address,
        occ,
        point: None,
    })
}
