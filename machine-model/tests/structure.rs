#![allow(clippy::assert_is_empty)]

mod common;

use common::{HEAD, codes, doc, tag_codes, tags};

fn unit_codes(units: &str) -> Vec<&'static str> {
    codes(&doc(&format!("units:\n{units}")))
}

#[test]
fn r21_machine_and_plc_ids() {
    for id in ["a b", "貼標", "", "a/b"] {
        let text = HEAD.replace("id: m1", &format!("id: '{id}'"));
        assert_eq!(codes(&text), ["R21"], "{id:?}");
    }
    assert_eq!(
        codes(&HEAD.replace("id: m1", "id: demo-2_a")),
        Vec::<&str>::new()
    );
    for plc in ["plc0", "plc", "PLC1", "plc01"] {
        let text = HEAD.replace("id: plc2", &format!("id: {plc}"));
        assert_eq!(codes(&text), ["R21"], "{plc}");
    }
    let dup = HEAD.replace("id: plc2", "id: plc1");
    assert_eq!(codes(&dup), ["R21"]);
}

#[test]
fn r1_tag_id_format_prefix_and_uniqueness() {
    for id in ["X000", "plc9.A", "plc1.a-b", "plc1.貼標", "plc1.", "plc0.A"] {
        let entry = format!("{{id: '{id}', address: M1, type: bool}}");
        assert_eq!(tag_codes(&[&entry]), ["R1"], "{id}");
    }
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: M1, type: bool}",
            "{id: plc1.A, address: M2, type: bool}",
        ]),
        ["R1"]
    );
}

#[test]
fn r1_tags_and_variables_share_a_namespace() {
    let body = format!(
        "{}variables:\n  - {{id: plc1.A, type: int16}}\n",
        tags(&["{id: plc1.A, address: M1, type: bool}"])
    );
    assert_eq!(codes(&doc(&body)), ["R1"]);
}

#[test]
fn r25_variables_must_not_have_an_address() {
    let body = "variables:\n  - {id: plc1.V, type: int16, address: D1}\n";
    assert_eq!(codes(&doc(body)), ["R25"]);
}

#[test]
fn r26_unit_instance_ids() {
    let unit = |id: &str| format!("  - {{id: {id}, type: t, type_version: 1}}\n");
    assert_eq!(unit_codes(&unit("unassigned")), ["R26"]);
    assert_eq!(unit_codes(&unit("'st-1'")), ["R26"]);
    assert_eq!(unit_codes(&unit("'貼標'")), ["R26"]);
    assert_eq!(
        unit_codes(&format!("{}{}", unit("st1"), unit("st1"))),
        ["R26"]
    );
    // `machine` 是根，允許宣告
    assert_eq!(unit_codes(&unit("machine")), Vec::<&str>::new());
}

#[test]
fn r28_type_version_type_and_bind_syntax() {
    let unit = |ty: &str, ver: &str, bind: &str| {
        format!("  - {{id: u1, type: {ty}, type_version: {ver}, bind: {{{bind}}}}}\n")
    };
    // 合法：Tag、Unit、null 三種值
    assert_eq!(
        unit_codes(&unit("t", "1", "a: plc1.A, b: null, c: st1")),
        Vec::<&str>::new()
    );
    assert_eq!(unit_codes(&unit("'bad-type'", "1", "")), ["R28"]);
    assert_eq!(unit_codes(&unit("t", "0", "")), ["R28"]);
    assert_eq!(unit_codes(&unit("t", "1", "'bad-key': st1")), ["R28"]);
    for bad in ["''", "'st-1'", "'a b'", "'plc1.'", "'x.y'", "'貼標'"] {
        assert_eq!(
            unit_codes(&unit("t", "1", &format!("a: {bad}"))),
            ["R28"],
            "{bad}"
        );
    }
    // 引用了沒有宣告的 PLC
    assert_eq!(unit_codes(&unit("t", "1", "a: plc9.A")), ["R28"]);
}

#[test]
fn diagnostics_carry_chinese_message_code_and_location() {
    let diags = machine_model::load(&doc(
        "variables:\n  - {id: plc1.V, type: int16, address: D1}\n",
    ))
    .unwrap_err();
    let text = diags[0].to_string();
    assert!(text.starts_with("[R25] variables[plc1.V]："), "{text}");
    assert!(text.contains("不得有 address"), "{text}");
}
