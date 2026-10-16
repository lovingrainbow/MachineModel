//! 實例層規則（U2～U11、U13、U14，規格 §8.3）：需要類型目錄。
//! 類型層規則（U1、U12、U15、U17～U19）在 [`crate::catalog`] 建目錄時檢查。
//!
//! 先決條件：模型已通過載入安全規則（[`crate::load`]）。本模組對違反先決條件的輸入
//! 不會 panic，但診斷可能不完整。
//! 注意：規格 U1 的「units[].id 唯一」已在載入安全規則由 [`crate::rules::r26_duplicate_unit_id`]
//! 負責檢查；本模組的 [`check`] 假設模型已通過 [`crate::load`]。單元類型內名稱唯一性（U1）
//! 則由 [`crate::catalog`] 建構時檢查。

mod graph;
mod pins;

use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

/// 全部實例層規則：先圖規則，再接腳規則。
pub fn check(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = check_graph(model, catalog);
    out.extend(check_pins(model, catalog));
    out
}

/// U2、U3、U5、U7、U8、U9。
pub fn check_graph(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    graph::check(model, catalog, &mut out);
    out
}

/// U4、U6、U10、U11、U13、U14。
pub fn check_pins(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    pins::check(model, catalog, &mut out);
    out
}
