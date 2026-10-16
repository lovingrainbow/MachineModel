//! 內嵌的標準 Unit 類型（規格 §9.1）：編譯時以 `include_str!` 帶進函式庫，
//! 現場只需要單一 `machine.yaml`。新增標準類型 = 新增檔案並加進 [`STANDARD`]，
//! 同時更新 `standards/unit-types.pins`。

use crate::catalog::UnitCatalog;

static STANDARD: &[(&str, &str)] = &[
    (
        "unit-types/cylinder_double.yaml",
        include_str!("../../standards/unit-types/cylinder_double.yaml"),
    ),
    (
        "unit-types/machine_ctrl.yaml",
        include_str!("../../standards/unit-types/machine_ctrl.yaml"),
    ),
    (
        "unit-types/station_load.yaml",
        include_str!("../../standards/unit-types/station_load.yaml"),
    ),
];

/// `(來源名稱, 內容)`，順序固定（依檔名）。
pub fn standard_sources() -> &'static [(&'static str, &'static str)] {
    STANDARD
}

impl UnitCatalog {
    /// 只含內嵌標準類型的目錄。內嵌內容有誤時 panic——`tests/standard_pins.rs` 保證不會發生。
    pub fn standard() -> Self {
        Self::from_sources(STANDARD)
            .map(|(catalog, _)| catalog)
            .expect("內嵌的標準類型必須通過檢查")
    }
}
