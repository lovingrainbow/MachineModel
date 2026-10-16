#![allow(clippy::assert_is_empty)]

mod common;

use common::{codes, doc, tag_codes};

#[test]
fn r2_types() {
    for ty in ["int", "string:0", "int16[501]", "string:4[2]", "Bool"] {
        let entry = format!("{{id: plc1.A, address: D1, type: '{ty}'}}");
        assert_eq!(tag_codes(&[&entry]), ["R2"], "{ty}");
    }
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: 'int16[10]'}"],),
        Vec::<&str>::new()
    );
}

#[test]
fn r2_applies_to_variables() {
    let body = "variables:\n  - {id: plc1.V, type: nonsense}\n";
    assert_eq!(codes(&doc(body)), ["R2"]);
}

#[test]
fn r5_min_must_be_below_max() {
    for (min, max) in [(5, 5), (6, 5)] {
        let entry = format!("{{id: plc1.A, address: D1, type: int16, min: {min}, max: {max}}}");
        assert_eq!(tag_codes(&[&entry]), ["R5"], "{min}/{max}");
    }
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, min: 4, max: 5}"],),
        Vec::<&str>::new()
    );
}

#[test]
fn r6_default_must_fit_type_and_range() {
    let bad = [
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: 11}",
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: -1}",
        "{id: plc1.A, address: D1, type: int16, default: 40000}",
        "{id: plc1.A, address: D1, type: int16, default: true}",
        "{id: plc1.A, address: D1, type: int16, default: 1.5}",
        "{id: plc1.A, address: M1, type: bool, default: 1}",
        "{id: plc1.A, address: D1, type: 'int16[2]', default: 1}",
        "{id: plc1.A, address: D1, type: 'string:3', default: abcd}",
        "{id: plc1.A, address: D1, type: float32, min: 0.0, max: 1.0, default: 1.5}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 0.25}",
        "{id: plc1.A, address: D1, type: uint16, default: -1}",
        "{id: plc1.A, address: D1, type: uint32, default: -1}",
        "{id: plc1.A, address: D1, type: 'string:3', default: 'éé'}",
    ];
    for c in bad {
        assert_eq!(tag_codes(&[c]), ["R6"], "{c}");
    }
    let ok = [
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: 10}",
        "{id: plc1.A, address: D2, type: float32, default: 2}",
        "{id: plc1.A, address: D4, type: float32, default: 0.5}",
        "{id: plc1.A, address: M1, type: bool, default: true}",
        "{id: plc1.A, address: D1, type: 'string:3', default: abc}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 0.5}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 1.0}",
        "{id: plc1.A, address: D1, type: 'string:3', default: 'é'}",
        "{id: plc1.A, address: D1, type: uint16, default: 65535}",
        "{id: plc1.A, address: D1, type: float64, default: 1.5}",
    ];
    for c in ok {
        assert_eq!(tag_codes(&[c]), Vec::<&str>::new(), "{c}");
    }
}

#[test]
fn r6_applies_to_variables_too() {
    let body = "variables:\n  - {id: plc1.V, type: int16, default: 40000}\n";
    assert_eq!(codes(&doc(body)), ["R6"]);
}

#[test]
fn r9_scale_zero() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, scale: 0}"]),
        ["R9"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, scale: 0.1}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r17_address_required() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: '', type: bool}"]),
        ["R17"]
    );
}
