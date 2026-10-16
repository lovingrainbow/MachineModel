mod common;

use common::{TREE, graph_codes};

#[test]
fn a_valid_tree_has_no_graph_diagnostics() {
    assert_eq!(graph_codes(TREE), Vec::<&str>::new());
}

#[test]
fn u2_type_must_exist_and_version_must_match() {
    let unknown = TREE.replace(
        "type: leaf, type_version: 1}\n  - {id: leaf2",
        "type: ghost, type_version: 1}\n  - {id: leaf2",
    );
    assert_eq!(graph_codes(&unknown), ["U2"]);
    // 實例釘舊版（類型目錄是 1，實例寫 2 或 0 以外的不符版本）
    let stale = TREE.replace(
        "id: leaf2, type: leaf, type_version: 1",
        "id: leaf2, type: leaf, type_version: 2",
    );
    assert_eq!(graph_codes(&stale), ["U2"]);
}

#[test]
fn u3_bind_keys_must_be_pins_or_parts_of_the_type() {
    let bad = TREE.replace(
        "id: leaf1, type: leaf, type_version: 1",
        "id: leaf1, type: leaf, type_version: 1, bind: {zzz: null}",
    );
    assert_eq!(graph_codes(&bad), ["U3"]);
    // 接腳名是合法的 key（接腳規則在 Task 6，這裡不報）
    let pin = TREE.replace(
        "id: leaf1, type: leaf, type_version: 1",
        "id: leaf1, type: leaf, type_version: 1, bind: {i1: null}",
    );
    assert_eq!(graph_codes(&pin), Vec::<&str>::new());
}

#[test]
fn u5_part_must_bind_an_existing_unit_of_the_declared_type() {
    // 不存在的單元
    let ghost = TREE.replace("a: leaf1", "a: nobody");
    assert!(graph_codes(&ghost).contains(&"U5"));
    // 類型不符：part a 宣告 leaf，卻綁了 mid 類型的單元
    let wrong = TREE.replace("a: leaf1", "a: mid1");
    assert!(graph_codes(&wrong).contains(&"U5"));
    // part 綁 Tag 而不是 Unit
    let tag = TREE.replace("a: leaf1", "a: plc1.T");
    assert!(graph_codes(&tag).contains(&"U5"));
    // null 是刻意不接，合法
    let null = TREE.replace("a: leaf1", "a: null");
    assert_eq!(graph_codes(&null), ["U8"]); // leaf1 不再被任何人綁 → 無法由 machine 到達
}

#[test]
fn u7_a_unit_has_at_most_one_parent() {
    let two = TREE.replace("b: leaf2", "b: leaf1");
    let codes = graph_codes(&two);
    assert!(codes.contains(&"U7"), "{codes:?}");
}

#[test]
fn u8_unreachable_units() {
    let orphan = format!("{TREE}  - {{id: orphan, type: leaf, type_version: 1}}\n");
    assert_eq!(graph_codes(&orphan), ["U8"]);
}

#[test]
fn u8_cycles_do_not_hang_and_are_reported() {
    // mid1 與 mid2 互相當對方的 part（同時也是 U5 類型不符）
    let units = "  - {id: machine, type: root, type_version: 1, bind: {m: mid1} }
  - {id: mid1, type: mid, type_version: 1, bind: {a: mid2} }
  - {id: mid2, type: mid, type_version: 1, bind: {a: mid1} }
";
    let codes = graph_codes(units);
    assert!(codes.contains(&"U8"), "{codes:?}");
    // 自我綁定
    let selfie = TREE.replace(
        "id: leaf1, type: leaf, type_version: 1",
        "id: leaf1, type: mid, type_version: 1, bind: {a: leaf1}",
    );
    let codes = graph_codes(&selfie);
    assert!(codes.contains(&"U8"), "{codes:?}");
}

#[test]
fn u9_machine_must_exist_and_be_the_root() {
    let no_machine = "  - {id: mid1, type: mid, type_version: 1, bind: {a: leaf1, b: leaf2}}
  - {id: leaf1, type: leaf, type_version: 1}
  - {id: leaf2, type: leaf, type_version: 1}
";
    assert_eq!(graph_codes(no_machine), ["U9"]);
    // machine 被別人當子層
    let child = TREE.replace("a: leaf1", "a: machine");
    assert!(graph_codes(&child).contains(&"U9"));
}

#[test]
fn diagnostics_are_traditional_chinese_with_location() {
    use common::{catalog, doc};
    let model =
        machine_model::parse(&doc("units:\n  - {id: u, type: ghost, type_version: 1}\n")).unwrap();
    let d = machine_model::analyze::check_graph(&model, &catalog());
    let text = d.iter().find(|x| x.code == "U2").unwrap().to_string();
    assert!(text.starts_with("[U2] units[u]："), "{text}");
    assert!(text.contains("類型目錄"), "{text}");
}
