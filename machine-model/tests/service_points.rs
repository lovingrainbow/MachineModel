#![allow(clippy::assert_is_empty)]

use machine_model::catalog::UnitCatalog;
use machine_model::resolve::{ResolvedModel, resolve};
use machine_model::schema::Scan;
use machine_model::writable::{Override, ServicePoint, WritableSource, evaluate};

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn resolved() -> ResolvedModel {
    let model = machine_model::load(FIXTURE).expect("fixture 必須通過載入安全規則");
    let catalog = UnitCatalog::standard();
    let diags = machine_model::analyze::check(&model, &catalog);
    assert!(diags.is_empty(), "{diags:#?}");
    resolve(&model, &catalog, FIXTURE)
}

fn point(id: &str, address: &str, ty: &str) -> ServicePoint {
    ServicePoint {
        id: id.to_owned(),
        address: address.to_owned(),
        ty: ty.to_owned(),
        writable: false,
        scan: None,
        min: None,
        max: None,
    }
}

fn writable_point(id: &str, address: &str, ty: &str) -> ServicePoint {
    ServicePoint {
        writable: true,
        min: Some(0.0),
        max: Some(100.0),
        ..point(id, address, ty)
    }
}

fn codes(report: &machine_model::writable::WriteReport) -> Vec<&'static str> {
    report.guardrail_errors.iter().map(|d| d.code).collect()
}

fn item<'a>(
    report: &'a machine_model::writable::WriteReport,
    id: &str,
) -> &'a machine_model::writable::FinalWritableItem {
    report.items.iter().find(|i| i.tag_id == id).unwrap()
}

#[test]
fn read_only_point_is_listed_as_service_and_not_reviewed() {
    let r = resolved();
    let report = evaluate(&r, &[], &[point("plc1.Extra_Speed", "D900", "int16")]);
    assert!(
        report.guardrail_errors.is_empty(),
        "{:?}",
        report.guardrail_errors
    );
    let it = item(&report, "plc1.Extra_Speed");
    assert_eq!(it.source, WritableSource::Service);
    assert!(!it.final_writable);
    assert!(!it.requires_human_review);
    assert!(report.review_required_tags.is_empty());
}

#[test]
fn writable_point_with_range_is_applied_and_reviewed() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[],
        &[writable_point("plc1.Extra_Speed", "D900", "int16")],
    );
    assert!(
        report.guardrail_errors.is_empty(),
        "{:?}",
        report.guardrail_errors
    );
    let it = item(&report, "plc1.Extra_Speed");
    assert!(it.final_writable);
    assert!(it.requires_human_review);
    assert_eq!(report.review_required_tags, ["plc1.Extra_Speed"]);
}

#[test]
fn ds5_id_repeats_model_tag_or_another_point() {
    let r = resolved();
    let report = evaluate(&r, &[], &[point("plc1.M_Spare", "M300", "bool")]);
    assert_eq!(codes(&report), ["DS5"]);
    assert!(
        report
            .items
            .iter()
            .all(|i| i.source != WritableSource::Service)
    );

    let report = evaluate(
        &r,
        &[],
        &[
            point("plc1.Extra", "D900", "int16"),
            point("plc1.Extra", "D902", "int16"),
        ],
    );
    assert_eq!(codes(&report), ["DS5"]);
}

#[test]
fn ds6_plc_must_exist_and_id_must_have_prefix() {
    let r = resolved();
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc9.Extra", "D900", "int16")])),
        ["DS6"]
    );
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("NoPrefix", "D900", "int16")])),
        ["DS6"]
    );
}

#[test]
fn ds7_bad_or_out_of_range_address_and_type_mismatch() {
    let r = resolved();
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc1.A", "Q10", "int16")])),
        ["DS7"]
    );
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc1.A", "D99999", "int16")])),
        ["DS7"]
    );
    // 字組類別上的 bool 必須有位元後綴
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc1.A", "D900", "bool")])),
        ["DS7"]
    );
}

#[test]
fn ds8_unknown_type_is_rejected() {
    let r = resolved();
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc1.A", "D900", "int99")])),
        ["DS8"]
    );
}

#[test]
fn ds1_read_only_class_point_cannot_be_writable() {
    let r = resolved();
    let report = evaluate(&r, &[], &[writable_point("plc1.Input", "X20", "bool")]);
    assert_eq!(codes(&report), ["DS1"]);
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Input"));
}

#[test]
fn ds10_writable_point_needs_min_and_max() {
    let r = resolved();
    let no_min = ServicePoint {
        min: None,
        ..writable_point("plc1.A", "D900", "int16")
    };
    assert_eq!(codes(&evaluate(&r, &[], &[no_min])), ["DS10"]);
}

#[test]
fn ds9_point_overlapping_model_tag_is_rejected() {
    let r = resolved();
    // M130 已被 plc1.M_Spare 佔用（位元類）
    let report = evaluate(&r, &[], &[point("plc1.Extra_Bit", "M130", "bool")]);
    assert_eq!(codes(&report), ["DS9"]);
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Extra_Bit"));

    // int32 佔 D200、D201，同時與 plc1.D_ModeCmd（D200）、plc1.D_ModeAct（D201）重疊：多字組也要判定，
    // 每個重疊的模型點位各報一筆 DS9。
    let report = evaluate(&r, &[], &[point("plc1.Extra_Long", "D200", "int32")]);
    let found = codes(&report);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().all(|c| *c == "DS9"));
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Extra_Long"));
}

#[test]
fn ds9_point_overlapping_another_point_rejects_the_later_one_only() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[],
        &[
            point("plc1.First", "D900", "int32"),
            point("plc1.Second", "D901", "int16"),
        ],
    );
    assert_eq!(codes(&report), ["DS9"]);
    assert!(report.items.iter().any(|i| i.tag_id == "plc1.First"));
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Second"));
}

#[test]
fn ds9_bit_in_word_class_overlaps_model_word() {
    let r = resolved();
    // D200 已被 plc1.D_ModeCmd（整個字組）佔用，D200.3 落在其中。
    let report = evaluate(&r, &[], &[point("plc1.Extra_Bit", "D200.3", "bool")]);
    assert_eq!(codes(&report), ["DS9"]);
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Extra_Bit"));
}

#[test]
fn ds9_same_bit_twice_rejects_the_later_one_but_other_bits_are_fine() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[],
        &[
            point("plc1.Bit_A", "D950.3", "bool"),
            point("plc1.Bit_B", "D950.3", "bool"),
            point("plc1.Bit_C", "D950.4", "bool"),
        ],
    );
    assert_eq!(codes(&report), ["DS9"]);
    assert!(report.items.iter().any(|i| i.tag_id == "plc1.Bit_A"));
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Bit_B"));
    assert!(report.items.iter().any(|i| i.tag_id == "plc1.Bit_C"));
}

#[test]
fn ds9_bit_inside_another_points_word_rejects_the_bit_point() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[],
        &[
            point("plc1.Word", "D900", "int16"),
            point("plc1.Bit", "D900.3", "bool"),
        ],
    );
    assert_eq!(codes(&report), ["DS9"]);
    assert!(report.items.iter().any(|i| i.tag_id == "plc1.Word"));
    assert!(report.items.iter().all(|i| i.tag_id != "plc1.Bit"));
}

#[test]
fn ds7_span_beyond_class_limit_is_rejected() {
    let r = resolved();
    // D 類別上限為 D7999；int32 從 D7999 起佔 D7999、D8000，超出上限。
    assert_eq!(
        codes(&evaluate(&r, &[], &[point("plc1.A", "D7999", "int32")])),
        ["DS7"]
    );
    // 剛好放得下的邊界。
    assert!(
        evaluate(&r, &[], &[point("plc1.A", "D7998", "int32")])
            .guardrail_errors
            .is_empty()
    );
}

/// 被 DS9 拒絕的點位不佔位址：只與被拒點位重疊的後續點位仍會被採用。
#[test]
fn ds9_rejected_point_does_not_block_later_points() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[],
        &[
            point("plc1.A", "D900", "int16"),
            point("plc1.B", "D900", "int32"), // 與 A 重疊 → 被拒
            point("plc1.C", "D901", "int16"), // 只與被拒的 B 重疊 → 仍被採用
        ],
    );
    assert_eq!(codes(&report), ["DS9"]);
    let kept: Vec<&str> = report
        .items
        .iter()
        .filter(|i| i.source == WritableSource::Service)
        .map(|i| i.tag_id.as_str())
        .collect();
    assert_eq!(kept, ["plc1.A", "plc1.C"]);
}

#[test]
fn ds11_override_cannot_target_a_service_point() {
    let r = resolved();
    let report = evaluate(
        &r,
        &[Override {
            tag_id: "plc1.Extra_Speed",
            writable: true,
        }],
        &[writable_point("plc1.Extra_Speed", "D900", "int16")],
    );
    assert_eq!(codes(&report), ["DS11"]);
}

#[test]
fn scan_defaults_to_normal_and_is_kept_when_given() {
    assert_eq!(point("plc1.A", "D900", "int16").scan_rate(), Scan::Normal);
    let fast = ServicePoint {
        scan: Some(Scan::Fast),
        ..point("plc1.A", "D900", "int16")
    };
    assert_eq!(fast.scan_rate(), Scan::Fast);
}
