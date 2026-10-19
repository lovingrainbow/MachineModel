//! 載入安全規則（規格 §8.3）。`PlcDataService` 載入與命令列工具共用同一份。
//!
//! R22（未知欄位）與 R27 的解析階段檢查在 [`crate::parse`]；其餘規則在這裡。
//! 引用是否存在、單親、無環、類型符合性需要類型目錄，屬 U 規則（Plan 2）。

mod addresses;
mod structure;
mod units;
mod values;

pub(crate) use addresses::{Occupancy, collisions, occupancy, type_fits};
pub(crate) use structure::version_error;

use crate::SCHEMA_VERSION;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

/// 執行全部載入安全規則。順序固定：版本、機台與 PLC、id、單元、欄位值、位址。
pub fn run(model: &MachineModel) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if model.schema_version != SCHEMA_VERSION {
        out.push(version_error(model.schema_version, SCHEMA_VERSION));
    }
    let plc_ids = structure::check_machine_and_plcs(&model.machine, &mut out);
    let ids = model
        .tags
        .iter()
        .map(|t| ("tags", t.id.as_str()))
        .chain(model.variables.iter().map(|v| ("variables", v.id.as_str())));
    structure::check_ids(ids, &plc_ids, &mut out);
    units::check(model, &plc_ids, &mut out);
    values::check(model, &mut out);
    addresses::check(model, &mut out);
    out
}
