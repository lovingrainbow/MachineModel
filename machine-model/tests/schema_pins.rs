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
