# plc-common 共用 crate、驅動單一化與 Inovance Easy 232/485 串列通訊 設計文件

日期：2026-10-18
狀態：v4，已實作並通過審查（未 commit）

## 1. 意圖與原因

新增一款 PLC 目前要在多個 repo 手動改多處：MachineModel 的家族表、DataService 與 PLCDriver 各自的家族表副本、各自一份 `mm-drivers`（`DriverKind`、驅動目錄、報文）。副本靠人工複製，漏改不會在編譯時發現，兩份 `mm-drivers` 也已經分歧（DataService 那份含緊湊逾時等修改，PLCDriver 那份沒有）。

本次目標是讓「新增型號」只需要改一處，其他地方自動生效；把驅動收斂為單一一份；統一家族表與基礎型別；並加入新款 **Inovance Easy 232/485 串列通訊**，使 DataService 能真的透過序列埠與該型號通訊。

## 2. 目標（Goals）

- G1：PLC 家族表、`ScalarValue`、`ScalarType` 只有一個來源，放在獨立的小 crate `plc-common`（`projects/plc-common/`）。
- G2：驅動程式碼（`mm-drivers`）只有一份，放在 PLCDriver（驅動的起點）。DataService 以路徑依賴取用，不再持有複本。
- G3：新增家族後，MachineModel 的 `model-debugger` 下拉選單與驅動目錄都自動出現，不需複製檔案。
- G4：新增 `inovance_easy_serial`，驅動經 RS-232/485（Modbus RTU）實際與實體 PLC 通訊。
- G5：若某個家族沒有對應的驅動，或某個 PLC 驅動沒有對應的家族，測試必須失敗。
- G6：`ScalarType` 在 MachineModel 與 DataService 的 `machine-model` 之間統一為同一個型別。

## 3. 非目標（Non-goals）

- 不做程式碼產生（codegen），`plc-common` 是普通的 Rust 程式碼。
- 不改 `machine-model` 對外 API：`machine_model::family::*`、`machine_model::datatype::ScalarType`、`machine_model::model::ScalarValue` 等路徑維持可用（以 re-export 達成）。
- 不統一 `DataType`（兩邊結構與錯誤型別不同：MachineModel 為 `TypeError`，DataService 為 `Diagnostic` 的 `SC3`）。
- 不合併 MachineModel 與 DataService 的兩套 `machine-model`（DataService 那套多了 `lowering`、`datasvc`、`cli`、`validator`）；另案。
- 不做 ASCII 模式的實際收發（報文函式已存在，本次只用 RTU）。
- 不修既有的寫入幀問題（RTU 寫入把長度放進值欄位、回讀比對寫死為成功）與 Inovance 的 X／Y／R Modbus 位址映射（TCP 與串列路徑相同）。
- 不 commit 任何變更，等使用者要求再提交。

## 4. 反目標（Anti-goals）

- MachineModel 的函式庫本體只依賴 `serde` 與 `serde_yaml_ng`，`plc-common` 是它的（僅 serde）依賴；依據 `2026-10-16-unit-tag-model-design.md` §8.5：執行期 PlcDataService 也會連結此函式庫；`plc-common` 只依賴 `serde`（`serde_yaml_ng` 僅限 dev-dependency）。
- 不得讓 `plc-common` 或 MachineModel 依賴 `tokio-serial`，也不得間接依賴它。`tokio-serial` 會經由 `tokio`、`serialport`、`mio-serial` 拉進依賴樹，因此會違反 `tokio` 禁令。
- `mm-drivers` 不得依賴任何 `machine-model`；它只依賴 `plc-common`。驅動不需要認識規則引擎。
- 理由：(1) 模型層保持純資料與純邏輯，任何宿主都能嵌入；(2) 只有 DataService 能開啟實體埠（`DATA_SERVICE_SPEC.md` 的「單一硬體所有權」）；(3) 工具程式不應被迫編入平台相關的序列埠程式碼；(4) 依賴只往下指，不出現 MachineModel → 驅動 repo 的反向依賴。
- 測試把關（見 §7）：`machine-model` 與 `plc-common` 的依賴樹都必須不含 `tokio-serial`、`serialport`、`mio-serial`，且 `plc-common` 只允許 `serde`。

## 5. 限制（Constraints）

- 各 repo 都**沒有 git remote**（PLCDriver 甚至不是 git repo，改動前已備份為 `projects/PLCDriver_backup_20261018.tar.gz`，不得刪除），只能用本機路徑依賴（使用者已選定）。MachineModel、DataService、PLCDriver 與 `plc-common` 必須並排放在 `orca/projects/` 下。
- 依賴方向（只往下）：

      plc-common            （serde）
        ├─ MachineModel/machine-model
        ├─ PLCDriver/crates/mm-drivers   （serde, tokio, tokio-serial, thiserror）
        │    ├─ PLCDriver/driver-workbench, PLCDriver/src-tauri
        │    └─ DataService/plc-data-service, datasvc-demo
        └─ DataService/crates/machine-model

- DataService 的 `crates/mm-drivers`、`crates/machine-model/src/lib.rs` 等有使用者未提交的修改，`family_table.rs` 等為未追蹤檔。實作時在原檔上修改，不還原、不覆蓋；合併 `mm-drivers` 時以 DataService 現有那份為準（已確認 PLCDriver 那份沒有 DataService 缺少的內容）。
- Inovance Easy 的通訊參數依 `driver3.md`：19200-8-E-1，RTU，CRC-16，位元組序 CDAB。這是文件值，尚未用實機驗證。

## 6. 設計

### 6.1 `plc-common` crate

- 位置：`projects/plc-common/`，獨立 crate（自帶 `[workspace]`，edition 2021，`rust-version = "1.85"`，`publish = false`，lint 設定與 MachineModel 的 workspace lints 相同）。
- 模組：
  - `family`：從 `MachineModel/plc-family` 搬入的家族列舉、`ALL`、`grammar()`、`as_str()`、位址表與既有測試，加上 `impl Display for Family`（因外部 crate 無法為其實作 `Display`）。
  - `value`：`ScalarValue { Bool(bool), Number(f64), Text(String) }` 與 `as_f64()`，自 DataService 的 `machine-model` 搬入。
  - `scalar_type`：統一的 `ScalarType`，見 §6.2。
- 根部 `pub use` 各模組公開項目。
- MachineModel 的 `machine-model/src/family.rs` 改為 `pub use plc_common::family::*;`；`machine-model/src/types.rs` 與 DataService `machine-model` 的對應位置改為 re-export。

### 6.2 統一的 `ScalarType`（G6）

- 成員：`Bool、Int16、Uint16、Int32、Uint32、Int64、Uint64、Float32、Float64、Text(u16)`。字串成員名稱採 MachineModel 的 `Text`（DataService 原為 `String`）。
- 衍生：`Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`。
- `ScalarType::parse(&str) -> Result<ScalarType, ScalarTypeError>`：`string:N` 的 N 必須是純數字且 **1～256**（上限 256 來自 DataService 現行實作；規格 §3.2 同步補上）；`ScalarTypeError { Unknown(String), BadStringLength(String) }`，`Display` 為繁體中文。
- 另有 `is_numeric()`、`is_string()`、`Display`（輸出詞彙，如 `string:16`）。
- `DataType`、`words()`、`int_bounds()` 與陣列規則仍留在各自的 `machine-model`，改用統一的 `ScalarType`。
- 規格 `2026-10-16-unit-tag-model-design.md` §3.2 補上字串長度 1～256。

### 6.3 新增家族 `inovance_easy_serial`

- `Family::InovanceEasySerial`，`as_str()` 回傳 `"inovance_easy_serial"`，位址表沿用 `INOVANCE_EASY`。
- 家族總數 55 → 56；規格 `2026-10-16-unit-tag-model-design.md` 的「55 種」同步改為 56，並在 §3.3 補一句串列驅動說明。

### 6.4 `mm-drivers` 單一化

- 位置：`PLCDriver/crates/mm-drivers/`；內容取自 DataService 現有那份（含 `InovanceEasySerial`、序列埠 I/O、緊湊逾時及其測試）。
- `Cargo.toml`：移除對 `machine-model` 的依賴，加入 `plc-common = { path = "../../../plc-common" }`；`PlcFamily` 來自 `plc_common::Family`、`ScalarValue` 來自 `plc_common`；刪除未使用的 `ScalarType` import。
- DataService：刪除 `crates/mm-drivers/`，`plc-data-service` 與 `datasvc-demo` 改依賴 `../../../PLCDriver/crates/mm-drivers`，並更新 workspace `members`。
- PLCDriver：刪除 `crates/machine-model/`（它只被用來取 `PlcFamily`／`ScalarValue`，兩者已搬入 `plc-common`）；`driver-workbench` 與 `src-tauri` 改依賴 `plc-common`；`InovanceEasy` 呼叫端改為 `InovanceEasyTcp`（序列化別名 `inovance_easy`、`mitsubishi_fx3g`、`mitsubishi_fx3s` 保留）。

### 6.5 驅動內容

- `DriverKind::InovanceEasySerial`（serde 名稱 `inovance_easy_serial`）；`default_byte_order()` → `CDAB`；`default_port()` → `0`；`build_protocol_frame()` 與 `extract_payload_from_response()` 走 RTU 臂；`list_driver_catalog()` 加一筆 PLC 項目（vendor `Inovance 匯川`，位址範例 `D100, M100, X010, Y010, R100`）。
- 序列埠為**同步**開啟：`open_tokio_serial_builder(cfg).timeout(..).open()`，不引入 async runtime。`DriverConfig` 新增 `serial: Option<SerialPortConfig>`（`#[serde(default)]`）。收發以首位元組等 `timeout_ms`、之後以 `inter_frame_silence_ms` 判定幀結束，驗證 CRC-16，例外幀回 `ProtocolError`。`connect()` 開頭先釋放舊連線。
- `loopback_emulation` 行為保留：沒有實體埠時仍用模擬回應，測試不需要硬體。
- `StationLink::Serial` 搭配非串列型號：維持原本警告後使用模擬回應，不變成錯誤。

### 6.6 一致性測試（自動化的關鍵）

- 放在 PLCDriver 的 `mm-drivers/tests/family_consistency.rs`：對 `Family::ALL` 每個家族，serde 轉成 `DriverKind` 必須成功且目錄有 PLC 項目；反向亦然；數量一致（56）。

## 7. 測試策略

- 依賴規則：
  - `MachineModel/machine-model/tests/dependency_rules.rs` 的 `BANNED_BASE` 加入 `tokio-serial`、`serialport`、`mio-serial`。
  - `plc-common/tests/dependency_rules.rs`：依賴樹只允許 `serde` 及其傳遞依賴，且不含 `tokio`、`tokio-serial`、`serialport`、`mio-serial`、`axum`、`reqwest`、`hyper`、`clap`；`Cargo.toml` 的 `[dependencies]` 鍵集合恰為 `{serde}`。
  - `PLCDriver/crates/mm-drivers` 新增測試：`cargo tree -p mm-drivers` 不含 `machine-model`（G2、§4）。
- 各 repo：
  - MachineModel：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check`。
  - plc-common：同上三項（在其目錄）。
  - PLCDriver：`cargo test -p mm-drivers -p driver-workbench`，`cargo check -p plc-driver-tauri`。
  - DataService：`cargo test -p machine-model -p plc-data-service -p datasvc-demo`。
- 串列 I/O：單元測試用 RTU 報文的位元組比對；實際收發用虛擬序列埠（pty）配對，不是實體 PLC，回報中須明說。
- 型別身分：DataService 內 `machine_model::family::PlcFamily` 與 `mm_drivers` 使用的家族型別必須是同一個型別（均為 `plc_common::Family`），以編譯通過與既有 `driver_for_family` 測試確認。

## 8. 風險

- 若任一 repo 或 `plc-common` 移動位置，路徑依賴會失效；MachineModel 單獨 clone 後無法編譯，必須有 `plc-common` 並排。
- `ScalarType` 統一會讓 MachineModel 的字串型別多出 256 上限，並改字串成員為 `u16`。
- DataService 的 `crates/mm-drivers` 與 PLCDriver 合併時，DataService 未提交的修改必須完整保留（以 DataService 現有那份為準）。
- 19200-8-E-1 與 RS-485 方向控制尚未用實機驗證。
- PLCDriver 與 plc-common 目前不是 git repo（沒有版本控制）；DataService 原有的 mm-drivers 修改現在只存在於 PLCDriver 的檔案中，提交前應先將這兩個目錄納入版本控制或備份。
- DataService 單獨無法編譯，必須有並排的 PLCDriver 與 plc-common；DataService 還會讀取 PLCDriver 的 workspace 設定（version.workspace、serde.workspace 以 PLCDriver/Cargo.toml 為準），PLCDriver 的 manifest 損壞會連帶破壞 DataService。
- 已知且本次不處理：寫入幀問題、Inovance 位址映射、`datasvc-demo` 互動模式改設定時舊服務仍佔用序列埠、兩套 `machine-model` 的分歧。

## 9. 交給實作者決定的細節（不改變結果的小決定）

- 序列埠收包逾時與重試次數，沿用 `DriverConfig.timeout_ms` 與既有的 `inter_frame_silence_ms`。
- `StationLink::Serial` 的 `SerialPortConfig` 由 `service.toml` 讀入，欄位沿用既有定義。
- `plc-common` 內部檔案切分（單檔或多模組檔），只要公開項目符合 §6.1。

## 10. 本文件中我做的決定（請核對）

1. 型號名稱 `inovance_easy_serial`，Family 與 DriverKind 同名。
2. 位元組序 CDAB、序列埠的 `default_port` 為 0。
3. 通訊參數 19200-8-E-1 RTU，來源是 `driver3.md`。
4. 底層 crate 名稱 `plc-common`，位置 `projects/plc-common/`，MachineModel、PLCDriver、DataService 都往下依賴它（使用者選定）。
5. `mm-drivers` 單一化到 PLCDriver，DataService 刪除自己的複本（使用者選定）。
6. `ScalarType` 統一、字串長度上限 256、字串成員名 `Text`（使用者選定上限；成員名與 `u16` 型別為我的決定）。
7. 刪除 PLCDriver 的 `crates/machine-model`，因其僅供取用 `PlcFamily`／`ScalarValue`。
8. 不 commit，只修改工作目錄中的檔案；PLCDriver 改動前已備份。
9. 依賴規則：`plc-common` 只允許 `serde`；`machine-model` 與 `plc-common` 明確禁止 `tokio-serial`、`serialport`、`mio-serial`；`mm-drivers` 不得依賴 `machine-model`。

## 11. 修訂紀錄

- v1（2026-10-18）：初稿，含 plc-family 抽離、inovance_easy_serial、串列 I/O、一致性測試。
- v2（2026-10-18）：補充 §4 反目標的理由與測試依據；§7 新增依賴樹測試。
- v3（2026-10-18）：使用者指出驅動的起點是 PLCDriver；`plc-family` 暫定改放 PLCDriver（已被 v4 取代，v3 計畫未執行）。
- v4（2026-10-18）：改為獨立的 `plc-common`（讓依賴只往下），`mm-drivers` 單一化到 PLCDriver 並不再依賴 `machine-model`，`ScalarValue` 與統一的 `ScalarType` 一併放入 `plc-common`，刪除 PLCDriver 的 `machine-model` 複本。v2 的實作（家族表抽離、串列驅動）已完成並通過審查，v4 為在其上的搬移與合併。
