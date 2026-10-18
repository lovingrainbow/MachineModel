# plc-common 共用 crate 與驅動單一化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立獨立的 `plc-common`（家族表、`ScalarValue`、統一的 `ScalarType`），讓 MachineModel、PLCDriver、DataService 的依賴只往下指，並把 `mm-drivers` 收斂為 PLCDriver 內的單一一份。

**Architecture:** `plc-common`（只依賴 `serde`）位於 `projects/plc-common/`。MachineModel 與 DataService 的 `machine-model` 以 re-export 保持舊路徑。`mm-drivers` 以 DataService 現有那份為準搬到 PLCDriver，只依賴 `plc-common`；DataService 刪除自己的複本並改依賴 PLCDriver 的版本。

**Tech Stack:** Rust（`rust-version = "1.85"`、不使用 let-chains）、serde。

**Spec:** `docs/superpowers/specs/2026-10-18-plc-family-shared-crate-design.md`（v4）。前置計畫 `docs/superpowers/plans/2026-10-18-plc-family-shared-crate.md` 已完成：`MM/plc-family`（含 `InovanceEasySerial`）、DataService 的串列驅動都已存在，是本計畫的搬移對象。

路徑代號（皆在 `/mnt/c/Users/wydct/orca/projects/` 下）：`MM/`=MachineModel、`DS/`=DataService、`PD/`=PLCDriver、`PC/`=plc-common。

## Global Constraints

- `plc-common` 只依賴 `serde`（`serde_yaml_ng` 僅限 dev-dependency）；`machine-model` 只依賴 `serde` 與 `serde_yaml_ng`；兩者依賴樹不得含 `tokio`、`tokio-serial`、`serialport`、`mio-serial`、`axum`、`reqwest`、`hyper`、`clap`。
- `mm-drivers` 不得依賴任何 `machine-model`（`cargo tree -p mm-drivers` 不含 `machine-model`）。
- 不使用 let-chains；診斷訊息用繁體中文；`rust-version = "1.85"`。
- `string:N` 的 N 為 1～256；`ScalarValue` 維持 `#[serde(untagged)]`，成員順序 `Bool(bool)`、`Number(f64)`、`Text(String)`，衍生 `Debug, Clone, PartialEq, Serialize, Deserialize`。
- 舊路徑維持可用：`machine_model::family::{PlcFamily, …}`、`machine_model::family_table`、`machine_model::datatype::ScalarType`、`machine_model::model::ScalarValue`、`machine_model::types::ScalarType`。
- 舊別名 `inovance_easy`（→`InovanceEasyTcp`）、`mitsubishi_fx3g`、`mitsubishi_fx3s` 繼續可解析。
- 19200-8-E-1／RTU 為文件值，未經實機驗證，註解須標明。
- **不 commit**；`projects/PLCDriver_backup_20261018.tar.gz` 不得刪除；DS 的既有未提交修改在原檔上保留，不得對 DS 執行 `git checkout/stash/clean/reset`，不得對 DS 整檔跑 `cargo fmt`。
- 不修既有的寫入幀問題與 Inovance 位址映射；不合併兩套 `machine-model`；不動 `DataType`。

## Review Focus

1. 型別身分：DS 內 `machine_model::family::PlcFamily` 與 `mm_drivers` 的家族型別是同一個（Task 6 以 `driver_for_family` 測試與 `cargo tree` 唯一 `plc-common` 驗證）。
2. `ScalarType` 統一後的邊界：`string:0`、`string:257`、`string:+4`、`string:`、`Bool`（大寫）都回錯誤；`string:256` 合法（Task 2、3）。
3. `ScalarValue` 的 untagged 反序列化行為不變：`true`→Bool、`1.5`→Number、`"x"`→Text（Task 2）。
4. DS 合併進 PD 後沒有遺失 DS 未提交的行為：緊湊逾時測試、串列測試仍在且通過（Task 5、6）。
5. 刪除 PD 的 `crates/machine-model` 後，`driver-workbench` 與 `src-tauri` 仍可編譯（Task 5）。
6. MachineModel 的 `words()` 對 `Text(u16)` 與 `values.rs` 的字串長度檢查在型別改為 `u16` 後行為不變（Task 3）。

---

### Task 1: 建立 `plc-common`，搬入家族表

**Files:**
- Create: `PC/Cargo.toml`, `PC/src/lib.rs`, `PC/src/family.rs`（由 `MM/plc-family/src/lib.rs` 內容搬入）, `PC/tests/dependency_rules.rs`（由 `MM/plc-family/tests/dependency_rules.rs` 改寫）

**Interfaces:**
- Consumes: `MM/plc-family` 的現有公開項目：`Family`（含 `InovanceEasySerial`，共 56 個；`ALL`、`grammar()`、`as_str()`、`Display`）、`Radix`、`Width`、`ClassProfile`、`Grammar`、`OPAQUE_CLASS`。
- Produces: package `plc-common`；`plc_common::family::*` 與根部 `plc_common::{Family, Radix, Width, ClassProfile, Grammar, OPAQUE_CLASS}`（`lib.rs` 內 `pub mod family; pub use family::*;`）。

- [ ] **Step 1: 先寫失敗的檢查**：建立 `PC/tests/dependency_rules.rs`，內容仿 `MM/plc-family/tests/dependency_rules.rs`（測試 `only_serde_allowed`、`manifest_declares_only_serde`；`cargo tree -p plc-common` 指令；禁用名單比原本多 `serialport`、`mio-serial` 之外維持相同）。執行 `cd PC && cargo test --offline` → 預期失敗（尚無 `Cargo.toml`）。
- [ ] **Step 2: 建立 crate**：`PC/Cargo.toml`：`name = "plc-common"`、`version = "0.1.0"`、`edition = "2021"`、`rust-version = "1.85"`、`publish = false`、`[dependencies] serde = { version = "1.0.229", features = ["derive"] }`、`[dev-dependencies] serde_yaml_ng = "0.10.0"`、空的 `[workspace]` 表、`[lints.rust] unsafe_code = "forbid"`、`[lints.clippy]` 與 `MM/Cargo.toml` 的 `[workspace.lints.clippy]` 逐字相同。把 `MM/plc-family/src/lib.rs` 複製為 `PC/src/family.rs`（先以 `grep -n "crate::"` 確認無內部依賴），`PC/src/lib.rs` 為 `pub mod family;` 加 `pub use family::*;`。
- [ ] **Step 3: 驗證**：`cd PC && cargo test --offline && cargo clippy --offline --all-targets -- -D warnings && cargo fmt --check`，預期全過，家族測試（含 56 個家族與別名）皆通過；若 edition 2021 下需最小修正，列入報告。

---

### Task 2: `plc-common` 加入 `ScalarValue` 與統一的 `ScalarType`

**Files:**
- Create: `PC/src/value.rs`, `PC/src/scalar_type.rs`
- Modify: `PC/src/lib.rs`（`pub mod value; pub mod scalar_type;` 與根部 `pub use`）

**Interfaces:**
- Consumes: 參考實作 `DS/crates/machine-model/src/lib.rs` 的 `model::ScalarValue`（含 `as_f64`）與 `datatype::ScalarType`（解析、`is_numeric`、`is_string`、`Display`）；規則來源 `MM/machine-model/src/types.rs` 的 `parse_scalar`（純數字檢查）。
- Produces:
  - `plc_common::ScalarValue`（見 Global Constraints；`pub fn as_f64(&self) -> Option<f64>`）。
  - `plc_common::ScalarType { Bool, Int16, Uint16, Int32, Uint32, Int64, Uint64, Float32, Float64, Text(u16) }`，衍生 `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`；`pub fn parse(raw: &str) -> Result<ScalarType, ScalarTypeError>`、`pub fn is_numeric(&self) -> bool`、`pub fn is_string(&self) -> bool`、`impl Display`（`Text(16)` 輸出 `string:16`）。
  - `plc_common::ScalarTypeError { Unknown(String), BadStringLength(String) }`，衍生 `Debug, Clone, PartialEq, Eq`，`impl Display`（繁體中文：`不認得的標量型別 `x``、`字串長度 `n` 無效，必須是 1 到 256 的整數`）、`impl std::error::Error`。

- [ ] **Step 1: 寫失敗測試**（放在各模組 `#[cfg(test)]`）：
  - `scalar_type`：`parse("string:256") == Ok(Text(256))`；`parse("string:16")` 與 `Display` 往返得 `string:16`；`parse("string:0")`、`"string:257"`、`"string:+4"`、`"string:"`、`"string:65536"` 都是 `Err(BadStringLength(_))`；`parse("Bool")`、`""`、`"int"` 是 `Err(Unknown(_))`；九個固定詞彙往返；`ScalarType` 是 `Copy`（以 `let a = ScalarType::Bool; let b = a; assert_eq!(a, b)`）；`is_numeric`／`is_string` 的真值表。
  - `value`：`serde_yaml_ng` 解析 `true`→`Bool(true)`、`1.5`→`Number(1.5)`、`x`→`Text("x")`；`as_f64` 對 `Number` 回 `Some`、其餘 `None`。
- [ ] **Step 2: 驗證失敗**：`cd PC && cargo test --offline` → 預期編譯失敗（模組不存在）。
- [ ] **Step 3: 實作**上述介面。`parse` 的字串分支：`strip_prefix("string:")`，N 非純 ASCII 數字（含空字串、正負號）→ `BadStringLength`；轉 `u16` 失敗或不在 1..=256 → `BadStringLength`；其他不認得 → `Unknown`。
- [ ] **Step 4: 驗證**：`cargo test --offline && cargo clippy --offline --all-targets -- -D warnings && cargo fmt --check` 全過；`cargo test --offline --test dependency_rules` 仍只含 `serde`。

---

### Task 3: MachineModel 改用 `plc-common`

**Files:**
- Modify: `MM/Cargo.toml`（`members` 移除 `"plc-family"`）, `MM/machine-model/Cargo.toml`（`plc-common = { path = "../../plc-common" }`，取代 `plc-family` 依賴）, `MM/machine-model/src/family.rs`（`pub use plc_common::family::*;`）, `MM/machine-model/src/types.rs`, `MM/machine-model/src/unit_type.rs`, `MM/machine-model/src/rules/values.rs`, `MM/machine-model/src/analyze/pins.rs`, `MM/machine-model/src/rules/addresses.rs`（如受影響）, `MM/AGENTS.md`（家族表位置改為 `plc-common/src/family.rs`、說明依賴方向與並排需求）, `MM/2026-10-16-unit-tag-model-design.md`（檔案位置同步；§3.2 補「`string:N` 的 N 為 1～256」；其他「55」已於前置計畫處理）
- Delete: `MM/plc-family/`

**Interfaces:**
- Consumes: Task 1、2 的 `plc_common::{ScalarType, ScalarTypeError, family::*}`。
- Produces: `machine_model::types::ScalarType`（= `plc_common::ScalarType` 的 re-export，`pub use plc_common::ScalarType;`）；`DataType`、`TypeError`、`MAX_ARRAY_LEN`、`words()`、`int_bounds()` 保留在 `types.rs`，`TypeError::{Unknown, BadStringLength}` 由 `ScalarTypeError` 轉換而來（`impl From<ScalarTypeError> for TypeError`，字串內容相同）。

- [ ] **Step 1: 建立基準並寫失敗測試**：`cd MM && cargo test --workspace` 記錄基準（預期全過）。在 `types.rs` 的測試新增 `string_length_is_capped_at_256`：`DataType::parse("string:256")` 為 `Ok`、`"string:257"` 為 `Err(TypeError::BadStringLength(_))`；`word_widths` 測試補 `string:256` 的 `words() == 128`。
- [ ] **Step 2: 驗證失敗**：新測試中的 257 案例失敗（目前無上限）。
- [ ] **Step 3: 實作**：移除 `types.rs` 自己的 `ScalarType` 與 `parse_scalar`，改用 `plc_common::ScalarType::parse` 並以 `From` 轉成 `TypeError`；`Text(usize)` 的使用處改 `Text(u16)`：`words()` 的 `n.div_ceil(2)` 改用 `usize::from(n).div_ceil(2)`，`rules/values.rs` 的 `s.len() <= n` 改 `s.len() <= usize::from(n)`，`unit_type.rs`、`pins.rs`、`addresses.rs` 依編譯錯誤調整。`DataType.len`／`MAX_ARRAY_LEN`（500）不動。刪除 `MM/plc-family/`，移除 `Cargo.toml` 成員。更新 AGENTS.md 與兩份規格文字。
- [ ] **Step 4: 驗證**：`cd MM && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` 全過（測試數＝基準減去隨 `plc-family` 搬走的測試，加上新增者；說明差額）；`cargo tree -p machine-model -e normal,build | grep -E "tokio|serialport|mio-serial"` 無輸出；`find /mnt/c/Users/wydct/orca/projects -maxdepth 2 -type d \( -name plc-family -o -name plc-common \)` 只剩 `plc-common`。

---

### Task 4: DataService 的 `machine-model` 改用 `plc-common`

**Files:**
- Modify: `DS/crates/machine-model/Cargo.toml`（`plc-family` 依賴改為 `plc-common = { path = "../../../plc-common" }`）, `DS/crates/machine-model/src/lib.rs`

**Interfaces:**
- Consumes: Task 2 的 `plc_common::{ScalarValue, ScalarType, ScalarTypeError}`、Task 1 的家族項目。
- Produces: 路徑不變：`machine_model::family_table`（`pub use plc_common as family_table;` 之外，`family_table::Radix/Width` 仍可解析，故使用 `pub use plc_common::family as family_table;`）、`machine_model::family::PlcFamily`、`machine_model::datatype::ScalarType`（`pub use plc_common::ScalarType;`）、`machine_model::model::ScalarValue`（`pub use plc_common::ScalarValue;`）。

- [ ] **Step 1: 建立基準**：`cd DS && cargo test -p machine-model -p mm-drivers -p plc-data-service -p datasvc-demo`，記錄各檔通過數（預期全過）。
- [ ] **Step 2: 寫失敗測試**：在 `lib.rs` 測試區新增 `scalar_type_is_the_shared_one`：`let a: plc_common::ScalarType = crate::datatype::ScalarType::Int16;` 能編譯並相等；`string_type_cap_is_256`：`ScalarType::parse("string:257")` 為 `Err`。
- [ ] **Step 3: 實作**：`datatype` 模組刪除自己的 `ScalarType`／`parse`／`is_numeric`／`is_string`／`Display`，改 re-export；所有 `ScalarType::String(n)` 改 `ScalarType::Text(n)`；`ScalarType::parse` 的錯誤由 `ScalarTypeError` 轉成原本所需的 `String`（`.map_err(|e| e.to_string())`）以維持 `DataType::parse` 產生的 `SC3` 診斷文字；`model` 模組刪除 `ScalarValue` 定義與 `impl`，改 re-export。`Display for PlcFamily` 已不在此 crate。
- [ ] **Step 4: 驗證**：同基準指令全過；各 crate 測試數與基準相同（`machine-model` 依搬走的測試調整並說明）；`cargo tree -p machine-model` 的 `plc-common` 子樹只有 `serde`。

---

### Task 5: 把 `mm-drivers` 單一化到 PLCDriver

**Files:**
- Modify: `PD/crates/mm-drivers/src/lib.rs`（以 `DS/crates/mm-drivers/src/lib.rs` 現況為準覆蓋後調整 import）, `PD/crates/mm-drivers/Cargo.toml`（移除 `machine-model`，加 `plc-common = { path = "../../../plc-common" }`）, `PD/Cargo.toml`（`members` 移除 `"crates/machine-model"`）, `PD/crates/driver-workbench/Cargo.toml`、`PD/src-tauri/Cargo.toml`（`machine-model` 依賴改 `plc-common`）, `PD/crates/driver-workbench/src/main.rs`（約 L481）, `PD/src-tauri/src/*.rs`（如有引用）, `PD/README.md`、`PD/USAGE.md`（指稱驅動／家族數量或 `machine-model` 之處）
- Create: `PD/crates/mm-drivers/tests/{family_consistency,tight_timeout,serial_rtu}.rs`（自 DS 複製）；`PD/crates/mm-drivers/tests/no_machine_model.rs`
- Delete: `PD/crates/machine-model/`
- 覆蓋: `PD/crates/mm-drivers/tests/plc_driver_tests.rs`（以 DS 版本為準）、`PD/crates/mm-drivers/src/serial_and_protocols.rs`（兩邊已逐字相同，不需動）

**Interfaces:**
- Consumes: DS 現有 `mm-drivers`（`DriverKind`（76 個，含 `InovanceEasySerial`）、`DriverConfig`（含 `serial: Option<SerialPortConfig>`）、`UniversalPlcDriver`、`PlcDriver`（含 `set_timeout_ms`）、`list_driver_catalog()`、`build_protocol_frame()`）；Task 1、2 的 `plc_common::{Family, ScalarValue}`。
- Produces: crate `mm-drivers`（PD 版）對外介面與 DS 現版相同，唯 `PlcFamily` 為 `plc_common::Family` 的別名（`use plc_common::Family as PlcFamily;`），`ScalarValue` 為 `plc_common::ScalarValue`；`pub use` 這兩者以便下游使用。

- [ ] **Step 1: 備份確認與差異審查**：確認 `projects/PLCDriver_backup_20261018.tar.gz` 存在。`diff PD/crates/mm-drivers/src/lib.rs DS/crates/mm-drivers/src/lib.rs`，列出**以 `<` 開頭、PD 有而 DS 沒有**的行；預期只有 `InovanceEasy` 舊名、未重構的 `Simulator => 0`／`Link` 之前的 `TcpStream` 欄位及註解數字，不含 PD 獨有行為。若發現 PD 獨有的行為，停止並回報。
- [ ] **Step 2: 寫失敗測試** `PD/crates/mm-drivers/tests/no_machine_model.rs`：測試 `drivers_do_not_depend_on_machine_model`，執行 `cargo tree -p mm-drivers --edges normal,build --prefix none --color never`（`current_dir(env!("CARGO_MANIFEST_DIR"))`），斷言輸出的套件名不含 `machine-model`，且包含 `plc-common`。執行 `cd PD && cargo test --offline -p mm-drivers --test no_machine_model` → 預期失敗。
- [ ] **Step 3: 搬移與調整**：複製 DS 的 `lib.rs` 與四個測試檔到 PD（`plc_driver_tests.rs` 覆蓋）；`use machine_model::family::PlcFamily` 改 `use plc_common::Family as PlcFamily`，`use machine_model::model::ScalarValue` 改 `use plc_common::ScalarValue`（測試檔同樣），刪除未使用的 `use machine_model::datatype::ScalarType`；`Cargo.toml` 依賴照介面調整（`tokio`、`tokio-serial`、`serde_json`、`thiserror` 沿用）。刪除 `PD/crates/machine-model/` 並更新 `PD/Cargo.toml`；`driver-workbench`、`src-tauri` 的 `machine-model` 依賴改 `plc-common`，其 `PlcFamily::InovanceEasy` 與 `machine_model::` 引用改為 `plc_common::Family::InovanceEasyTcp`／`plc_common::ScalarValue`。
- [ ] **Step 4: 同步文字**：`grep -rn "machine-model\|machine_model\|55\|56" PD/README.md PD/USAGE.md` 篩出需更新者（專案架構目錄不再有 `crates/machine-model/`、驅動共 76 個、PLC 共 56 種）並更新，列入報告。
- [ ] **Step 5: 驗證**：`cd PD && cargo test --offline -p mm-drivers -p driver-workbench`（`mm-drivers` 測試數應等於 DS 現況：`plc_driver_tests` 13、`tight_timeout` 1、`family_consistency` 5、`serial_rtu` 含新增的重連測試）全過；`cargo check --offline -p plc-driver-tauri` 通過；`cargo tree -p mm-drivers` 不含 `machine-model`。回報須說明串列測試實際執行數、僅用 pty 模擬、未對實體 PLC 驗證。

---

### Task 6: DataService 改依賴 PLCDriver 的 `mm-drivers` 並刪除複本

**Files:**
- Modify: `DS/Cargo.toml`（`members` 移除 `"crates/mm-drivers"`）, `DS/crates/plc-data-service/Cargo.toml`、`DS/crates/datasvc-demo/Cargo.toml`（`mm-drivers = { path = "../../../PLCDriver/crates/mm-drivers" }`）, 其他引用 `mm-drivers` 的 `Cargo.toml`（`grep -rn "mm-drivers" DS --include=Cargo.toml`）, 文件中指向 `crates/mm-drivers` 的路徑（`grep -rn "crates/mm-drivers" DS --include=*.md`）
- Delete: `DS/crates/mm-drivers/`

**Interfaces:**
- Consumes: Task 5 的 `PD/crates/mm-drivers`；Task 4 的 `machine_model::family::PlcFamily`（= `plc_common::Family`）。
- Produces: `plc-data-service`、`datasvc-demo` 對 `mm_drivers::*` 的使用不變。

- [ ] **Step 1: 建立基準**：`cd DS && cargo test -p machine-model -p plc-data-service -p datasvc-demo`，記錄結果（`mm-drivers` 的測試此後在 PD 執行）。
- [ ] **Step 2: 寫失敗測試**：在 `DS/crates/datasvc-demo/src/model.rs` 測試區新增 `family_type_is_shared_with_drivers`：`let f: machine_model::family::PlcFamily = machine_model::family::PlcFamily::InovanceEasySerial; let cfg = mm_drivers::DriverConfig { plc_family: f, ..mm_drivers::DriverConfig::default() };` 能編譯（型別身分），且 `driver_for_family(f) == mm_drivers::DriverKind::InovanceEasySerial`。此時 DS 仍指向自己的複本、型別不同 → 預期編譯失敗或在 Task 4 之後已通過；若已通過，仍保留為回歸測試，並在報告說明。
- [ ] **Step 3: 切換**：依賴改指 PD，刪除 `DS/crates/mm-drivers/`，更新 workspace `members`。**先確認** Task 5 已把 DS 該目錄內所有內容（`src/`、`tests/`）搬到 PD（以 `diff -r` 比對 DS 與 PD 的 `mm-drivers/src`，只應有 import 差異），再刪除。
- [ ] **Step 4: 驗證**：`cd DS && cargo test -p machine-model -p plc-data-service -p datasvc-demo` 全過；`cargo check -p plc-data-service -p datasvc-demo`（`src-tauri` 如能編譯也 `cargo check`，否則說明）；`cargo tree -p plc-data-service` 的 `plc-common` 只出現一份且路徑為 `/mnt/c/Users/wydct/orca/projects/plc-common`；`find /mnt/c/Users/wydct/orca/projects -maxdepth 4 -type d -name mm-drivers -not -path "*/target/*"` 只剩 `PLCDriver/crates/mm-drivers`。

---

## 自我檢查

- 規格覆蓋（v4）：G1→Task 1、2；G2→Task 5、6；G3→Task 1（`model-debugger` 取自 `Family::ALL`）、Task 3；G4→Task 5（搬入既有串列實作）；G5→Task 5（`family_consistency`）；G6→Task 2、3、4。§4 `mm-drivers` 不依賴 `machine-model`→Task 5 的 `no_machine_model.rs`。
- 型別一致：`ScalarType::Text(u16)`、`ScalarTypeError`、`plc_common::Family`、`plc_common::ScalarValue` 在 Task 2 定義，Task 3–6 皆照此使用。
- 仍不處理（列入最終回報）：兩份 `machine-model` 的分歧、寫入幀與 Inovance 位址映射、`datasvc-demo` 互動模式的序列埠佔用、`DataType` 統一。
