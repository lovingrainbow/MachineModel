#![allow(clippy::assert_is_empty)]

mod common;

use common::{codes, doc, tag_codes, tags};

#[test]
fn r18_address_syntax_and_range() {
    for addr in ["X008", "X2000", "D8000", "Q1", "D", "R10.16", "M10.1"] {
        let entry = format!("{{id: plc1.A, address: '{addr}', type: bool}}");
        assert!(tag_codes(&[&entry]).contains(&"R18"), "{addr}");
    }
    // D 與 R 接受位元後綴
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'D10.1', type: bool}"]),
        Vec::<&str>::new()
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'D10.1', type: int16}"]),
        ["R19"]
    );
    // 三菱沒有 D 類別（以家族表為準）
    assert!(tag_codes(&["{id: plc2.A, address: D100, type: int16}"]).contains(&"R18"));
}

#[test]
fn r18_span_must_stay_inside_the_class() {
    assert!(tag_codes(&["{id: plc1.A, address: D7999, type: int32}"]).contains(&"R18"));
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D7998, type: int32}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r19_type_must_fit_address_class() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: M10, type: int16}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D10, type: bool}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: R10.3, type: int16}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'R10.3', type: 'bool[2]'}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: R10.3, type: bool}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r24_polarity_only_on_boolean_inputs() {
    assert_eq!(
        tag_codes(&["{id: plc1.Y1, address: Y1, type: bool, polarity: NO}"]),
        ["R24"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.D1, address: D1, type: int16, polarity: NC}"]),
        ["R24"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.X1, address: X1, type: bool, polarity: NC}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r8_overlap() {
    // 同一位址的別名（X010 與 X10 是同一點）
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: X010, type: bool}",
            "{id: plc1.B, address: X10, type: bool}",
        ]),
        ["R8"]
    );
    // int32 佔 D100、D101
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: int32}",
            "{id: plc1.B, address: D101, type: int16}",
        ]),
        ["R8"]
    );
    // 陣列與字串
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: 'int16[4]'}",
            "{id: plc1.B, address: D103, type: 'string:4'}",
        ]),
        ["R8"]
    );
    // 位元後綴：同一位元重疊
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: R10.3, type: bool}",
            "{id: plc1.B, address: R10.3, type: bool}",
        ]),
        ["R8"]
    );
    // 位元與涵蓋該字組的 tag 重疊
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: R10, type: int16}",
            "{id: plc1.B, address: R10.3, type: bool}",
        ]),
        ["R8"]
    );
    // 不重疊：相鄰位址、不同位元、不同 PLC 的相同位址
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: int16}",
            "{id: plc1.B, address: D101, type: int16}",
            "{id: plc1.C, address: R10.3, type: bool}",
            "{id: plc1.E, address: R10.4, type: bool}",
            "{id: plc2.A, address: X007, type: bool}",
            "{id: plc1.F, address: X007, type: bool}",
        ]),
        Vec::<&str>::new()
    );
}

#[test]
fn mitsubishi_octal_io_loads() {
    let body = tags(&[
        "{id: plc2.X007, address: X007, type: bool}",
        "{id: plc2.X010, address: X010, type: bool}",
        "{id: plc2.Y017, address: Y017, type: bool}",
    ]);
    assert_eq!(codes(&doc(&body)), Vec::<&str>::new());
}

#[test]
fn v2_has_no_r20_the_writable_concept_does_not_exist() {
    // X 類別本來就不可寫；v2 的 Tag 沒有 writable，所以什麼都不會被擋，也沒有 R20。
    assert_eq!(
        tag_codes(&["{id: plc1.X1, address: X1, type: bool}"]),
        Vec::<&str>::new()
    );
}
