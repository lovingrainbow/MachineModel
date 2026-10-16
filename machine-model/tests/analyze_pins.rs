mod common;

use common::{L1_OK, pin_codes};

#[test]
fn a_fully_bound_tree_has_no_pin_diagnostics() {
    assert_eq!(pin_codes(L1_OK, &[]), Vec::<&str>::new());
}

#[test]
fn u6_required_pins_must_be_bound() {
    // 少了必接的 field 腳 o1
    assert_eq!(
        pin_codes("i1: plc1.L1_in, error_e: plc1.L1_err", &[]),
        ["U6"]
    );
    // 少了 error 腳；或明寫 null 也不行
    assert_eq!(pin_codes("i1: plc1.L1_in, o1: plc1.L1_out", &[]), ["U6"]);
    assert_eq!(
        pin_codes("i1: plc1.L1_in, o1: plc1.L1_out, error_e: null", &[]),
        ["U6"]
    );
    // field 腳明寫 null 視為刻意不接；required: false 的 i2 可以省略
    assert_eq!(
        pin_codes("i1: null, o1: plc1.L1_out, error_e: plc1.L1_err", &[]),
        Vec::<&str>::new()
    );
}

#[test]
fn u4_field_pins_check_existence_direction_and_type() {
    // Tag 不存在
    assert_eq!(
        pin_codes("i1: plc1.NOPE, o1: plc1.L1_out, error_e: plc1.L1_err", &[]),
        ["U4"]
    );
    // field.in 綁到輸出位址（Y）
    assert_eq!(
        pin_codes("i1: plc1.L1_out, o1: plc1.L1_in, error_e: plc1.L1_err", &[]),
        ["U4", "U4"]
    );
    // field.in 綁到非輸入類別（M）
    assert_eq!(
        pin_codes("i1: plc1.L1_err, o1: plc1.L1_out, error_e: plc1.L1_in", &[]),
        ["U4"]
    );
    // field.out 綁到非實體輸出類別（M）
    assert_eq!(
        pin_codes("i1: plc1.L1_in, o1: plc1.L1_err, error_e: plc1.L2_err", &[]),
        ["U4"]
    );
    // 型別不符：int16 的 Tag 綁 bool 腳
    assert_eq!(
        pin_codes(
            "i1: plc1.W, o1: plc1.L1_out, error_e: plc1.L1_err",
            &["{id: plc1.W, address: D1, type: int16}"]
        )
        .into_iter()
        .filter(|c| *c == "U4")
        .count(),
        2 // 型別不符，且 D 不是輸入位址
    );
    // field 腳綁 Unit id
    assert_eq!(
        pin_codes("i1: leaf2, o1: plc1.L1_out, error_e: plc1.L1_err", &[]),
        ["U4"]
    );
}

#[test]
fn u10_ctrl_pins_check_existence_and_type() {
    let with = |pin: &str, tags: &[&str]| pin_codes(&format!("{L1_OK}, {pin}"), tags);
    assert_eq!(with("man_go: plc1.NOPE", &[]), ["U10"]);
    assert_eq!(
        with(
            "man_go: plc1.W",
            &["{id: plc1.W, address: D1, type: int16}"]
        ),
        ["U10"]
    );
    assert_eq!(with("man_go: leaf2", &[]), ["U10"]);
    // 合法：bool 腳綁 M 位元；null 是刻意不接
    assert_eq!(
        with(
            "man_go: plc1.G",
            &["{id: plc1.G, address: M300, type: bool}"]
        ),
        Vec::<&str>::new()
    );
    assert_eq!(with("man_go: null", &[]), Vec::<&str>::new());
    // par_t 是 int16；綁 uint16 要被擋（型別必須完全相同）
    assert_eq!(
        with(
            "par_t: plc1.P",
            &["{id: plc1.P, address: D1, type: uint16}"]
        ),
        ["U10"]
    );
    assert_eq!(
        with("par_t: plc1.P", &["{id: plc1.P, address: D1, type: int16}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn u11_multi_binding_restrictions() {
    // 寫入候選（man_）與唯讀狀態（at_）混綁同一 Tag
    let mixed_rw = pin_codes(
        &format!("{L1_OK}, man_go: plc1.G, at_ok: plc1.G"),
        &["{id: plc1.G, address: M300, type: bool}"],
    );
    assert_eq!(mixed_rw, ["U11"]);

    // 跨單元：leaf1 的 man_go 綁到 leaf2 已用的 L2_err（alarm 唯讀腳）
    let cross = pin_codes(&format!("{L1_OK}, man_go: plc1.L2_err"), &[]);
    assert_eq!(cross, ["U11"]);

    // field 腳與 ctrl 腳混用同一個 Tag
    let mixed_field_ctrl = pin_codes(&format!("{L1_OK}, at_ok: plc1.L1_in"), &[]);
    assert_eq!(mixed_field_ctrl, ["U11"]);

    // 實體輸出 field.out 腳被兩個單元共用（搶同一個 Y 點）
    let shared_out = pin_codes("i1: plc1.L1_in, o1: plc1.L2_out, error_e: plc1.L1_err", &[]);
    assert_eq!(shared_out, ["U11"]);
}

#[test]
fn u11_allows_shared_inputs_parameters_and_operations() {
    // 兩個單元共用 field.in（感測輸入，如 X）
    let shared_in = pin_codes("i1: plc1.L2_in, o1: plc1.L1_out, error_e: plc1.L1_err", &[]);
    assert_eq!(shared_in, Vec::<&str>::new());

    // 兩個單元共用 par_（參數，如 D）
    let shared_par = pin_codes(
        &format!("{L1_OK}, par_t: plc1.P"),
        &["{id: plc1.P, address: D1, type: int16}"],
    );
    assert_eq!(shared_par, Vec::<&str>::new());

    // 兩個單元共用 man_（同動操作按鈕，如 M）
    let shared_man = pin_codes(
        &format!("{L1_OK}, man_go: plc1.G"),
        &["{id: plc1.G, address: M300, type: bool}"],
    );
    assert_eq!(shared_man, Vec::<&str>::new());
}

#[test]
fn u11_does_not_duplicate_errors_for_non_existent_tags() {
    // 幽靈 Tag 只報 U4/U10 不存在，不應再報 U11
    let ghost = pin_codes(
        "i1: plc1.NOPE, o1: plc1.L1_out, error_e: plc1.L1_err, man_go: plc1.NOPE",
        &[],
    );
    assert!(!ghost.contains(&"U11"), "{ghost:?}");
}

#[test]
fn u13_operation_pins_must_not_bind_read_only_or_safety_monitor_tags() {
    let with = |pin: &str, tags: &[&str]| pin_codes(&format!("{L1_OK}, {pin}"), tags);
    // man_ 綁 X 類別
    assert_eq!(
        with(
            "man_go: plc1.XX",
            &["{id: plc1.XX, address: X30, type: bool}"]
        ),
        ["U13"]
    );
    // man_ 綁 safety_monitor
    assert_eq!(
        with(
            "man_go: plc1.S",
            &["{id: plc1.S, address: M300, type: bool, safety_monitor: true}"]
        ),
        ["U13"]
    );
    // 兩個條件都成立：各報一次
    assert_eq!(
        with(
            "man_go: plc1.XS",
            &["{id: plc1.XS, address: X31, type: bool, safety_monitor: true}"]
        ),
        ["U13", "U13"]
    );
    // par_ 同樣受限（X 是位元類別，型別也不符，所以同時有 U10）
    let par = with(
        "par_t: plc1.XP",
        &["{id: plc1.XP, address: X32, type: int16}"],
    );
    assert!(par.contains(&"U13"), "{par:?}");
    // 非候選腳（at_）綁 X 或 safety_monitor 都可以
    assert_eq!(
        with(
            "at_ok: plc1.AX",
            &["{id: plc1.AX, address: X33, type: bool, safety_monitor: true}"]
        ),
        Vec::<&str>::new()
    );
}

#[test]
fn u14_explicit_role_that_disagrees_with_the_pin_is_a_warning() {
    let with = |tag: &str, pin: &str| pin_codes(&format!("{L1_OK}, {pin}"), &[tag]);
    // pv_temp 的前綴 role 是 process_value；Tag 明寫 command → 提示
    assert_eq!(
        with(
            "{id: plc1.T, address: D1, type: float32, role: command}",
            "pv_temp: plc1.T"
        ),
        ["U14"]
    );
    // 一致就不提示
    assert_eq!(
        with(
            "{id: plc1.T, address: D1, type: float32, role: process_value}",
            "pv_temp: plc1.T"
        ),
        Vec::<&str>::new()
    );
}

#[test]
fn u14_is_a_warning_not_an_error() {
    use common::{BASE_TAGS, L1_OK, catalog, doc, tags};
    let units = format!(
        "units:\n  - {{id: machine, type: root, type_version: 1, bind: {{m: mid1}}}}\n  - {{id: mid1, type: mid, type_version: 1, bind: {{a: leaf1, b: leaf2}}}}\n  - {{id: leaf1, type: leaf, type_version: 1, bind: {{{L1_OK}, pv_temp: plc1.T}}}}\n  - {{id: leaf2, type: leaf, type_version: 1, bind: {{i1: plc1.L2_in, o1: plc1.L2_out, error_e: plc1.L2_err}}}}\n"
    );
    let mut entries: Vec<&str> = BASE_TAGS.to_vec();
    entries.push("{id: plc1.T, address: D1, type: float32, role: command}");
    let model = machine_model::parse(&doc(&format!("{units}{}", tags(&entries)))).unwrap();
    let diags = machine_model::analyze::check_pins(&model, &catalog());
    assert_eq!(diags.len(), 1);
    assert!(!diags[0].is_error());
}
