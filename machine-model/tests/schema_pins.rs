#![cfg(feature = "schema")]

use machine_model::schema_gen::generate_unit_type_schema;

/// 版本庫中的 Schema 檔案必須等於產生器最新輸出。
#[test]
fn pinned_schema_matches_generator() {
    let on_disk = include_str!("../../standards/schema/unit-type.schema.json");
    let generated = generate_unit_type_schema();
    assert_eq!(
        on_disk, generated,
        "standards/schema/unit-type.schema.json 與產生器輸出不一致；請執行 print_schema 更新"
    );
}

/// 手動執行：產生 standards/schema/unit-type.schema.json 的內容。
#[test]
#[ignore = "手動執行：印出最新 schema"]
fn print_schema() {
    print!("{}", generate_unit_type_schema());
}

/// 規格 §4.1 修訂：ctrl 接腳的 schema 不再有 kind／level／reset／role（Tag 層級的 role 不在此列）。
#[test]
fn schema_drops_removed_pin_attributes() {
    let schema: serde_json::Value =
        serde_json::from_str(&generate_unit_type_schema()).expect("產生器輸出必須是 JSON");
    let patterns = schema["properties"]["ctrl"]["patternProperties"]
        .as_object()
        .expect("ctrl 必須以 patternProperties 逐前綴定義");
    for (pattern, def) in patterns {
        let props = def["properties"]
            .as_object()
            .expect("每個前綴都要有 properties");
        for gone in ["kind", "level", "reset", "role"] {
            assert!(
                !props.contains_key(gone),
                "{pattern} 仍定義已移除的屬性 {gone}"
            );
        }
    }
}

/// 規格 §4.2：每個 ctrl 前綴都接受可選的 unit（label 與 unit 不分前綴）。
#[test]
fn every_ctrl_prefix_accepts_unit() {
    let schema: serde_json::Value =
        serde_json::from_str(&generate_unit_type_schema()).expect("產生器輸出必須是 JSON");
    let patterns = schema["properties"]["ctrl"]["patternProperties"]
        .as_object()
        .expect("ctrl 必須以 patternProperties 逐前綴定義");
    for (pattern, def) in patterns {
        let props = def["properties"]
            .as_object()
            .expect("每個前綴都要有 properties");
        assert!(props.contains_key("unit"), "{pattern} 缺少 unit");
    }
}
