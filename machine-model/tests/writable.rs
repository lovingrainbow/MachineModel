#![allow(clippy::assert_is_empty)]

use machine_model::catalog::UnitCatalog;
use machine_model::resolve::{ResolvedModel, resolve};
use machine_model::writable::{Override, WritableSource, evaluate};

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn resolved() -> ResolvedModel {
    let model = machine_model::load(FIXTURE).expect("fixture 必須通過載入安全規則");
    let catalog = UnitCatalog::standard();
    let diags = machine_model::analyze::check(&model, &catalog);
    assert!(diags.is_empty(), "{diags:#?}");
    resolve(&model, &catalog, FIXTURE)
}

fn ov(tag_id: &str, writable: bool) -> Override<'_> {
    Override { tag_id, writable }
}

fn codes(report: &machine_model::writable::WriteReport) -> Vec<&'static str> {
    report.guardrail_errors.iter().map(|d| d.code).collect()
}

#[test]
fn without_overrides_final_equals_write_candidate() {
    let r = resolved();
    let report = evaluate(&r, &[], &[]);
    assert!(report.guardrail_errors.is_empty());
    assert!(report.review_required_tags.is_empty());
    for item in &report.items {
        assert_eq!(item.final_writable, item.write_candidate, "{}", item.tag_id);
        assert_eq!(item.source, WritableSource::Model, "{}", item.tag_id);
    }
}

#[test]
fn ds1_read_only_class_cannot_be_made_writable() {
    let r = resolved();
    let report = evaluate(&r, &[ov("plc1.X_Cyl1_Ext_LS", true)], &[]);
    assert_eq!(codes(&report), ["DS1"]);
    let item = report
        .items
        .iter()
        .find(|i| i.tag_id == "plc1.X_Cyl1_Ext_LS")
        .unwrap();
    assert!(!item.final_writable, "違規的覆寫不得套用");
    assert_eq!(item.source, WritableSource::Model);
}

#[test]
fn ds2_safety_monitor_cannot_be_made_writable() {
    let r = resolved();
    let report = evaluate(&r, &[ov("plc1.M_Estop", true)], &[]);
    assert_eq!(codes(&report), ["DS2"]);
    let item = report
        .items
        .iter()
        .find(|i| i.tag_id == "plc1.M_Estop")
        .unwrap();
    assert!(!item.final_writable, "違規的覆寫不得套用");
}

#[test]
fn ds3_unknown_tag_is_rejected() {
    let r = resolved();
    let report = evaluate(&r, &[ov("plc1.NOPE", true)], &[]);
    assert_eq!(codes(&report), ["DS3"]);
}

#[test]
fn ds4_duplicate_override_applies_to_neither_occurrence() {
    let r = resolved();
    // 第一筆想開放，第二筆想關閉：兩筆都不得套用，也不能靜默取最後一筆。
    let report = evaluate(
        &r,
        &[ov("plc1.M_Spare", true), ov("plc1.M_Spare", false)],
        &[],
    );
    assert_eq!(codes(&report), ["DS4"]);
    let item = report
        .items
        .iter()
        .find(|i| i.tag_id == "plc1.M_Spare")
        .unwrap();
    assert_eq!(item.source, WritableSource::Model);
    assert!(!item.final_writable);
}

#[test]
fn valid_override_to_false_is_applied_and_not_reviewed() {
    let r = resolved();
    let report = evaluate(&r, &[ov("plc1.M_Cyl1_ManExt", false)], &[]);
    assert!(report.guardrail_errors.is_empty());
    let item = report
        .items
        .iter()
        .find(|i| i.tag_id == "plc1.M_Cyl1_ManExt")
        .unwrap();
    assert!(item.write_candidate);
    assert!(!item.final_writable);
    assert_eq!(item.source, WritableSource::Override);
    assert!(!item.requires_human_review);
}

#[test]
fn valid_override_to_true_on_non_candidate_requires_review() {
    let r = resolved();
    let report = evaluate(&r, &[ov("plc1.M_Spare", true)], &[]);
    assert!(report.guardrail_errors.is_empty());
    let item = report
        .items
        .iter()
        .find(|i| i.tag_id == "plc1.M_Spare")
        .unwrap();
    assert!(!item.write_candidate);
    assert!(item.final_writable);
    assert_eq!(item.source, WritableSource::Override);
    assert!(item.requires_human_review);
    assert_eq!(report.review_required_tags, ["plc1.M_Spare"]);
}
