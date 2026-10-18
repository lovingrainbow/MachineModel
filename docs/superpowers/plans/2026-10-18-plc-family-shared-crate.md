# PLC 家族表共用化與 Inovance Easy 串列通訊 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 PLC 家族表收斂到單一 crate `plc-family`，並新增 `inovance_easy_serial`，讓 DataService 經 Modbus RTU 實際走序列埠通訊。

**Architecture:** `plc-family`（只依賴 `serde`）放在 MachineModel workspace，`machine-model` 以 `pub use` 轉出，DataService 的 `machine-model` crate 以路徑依賴取用並刪除複本。序列埠 I/O 只在 DataService 的 `mm-drivers`，用 `open_tokio_serial_builder(cfg).open()` 取得**同步**埠（驅動本體是同步的 `std::net::TcpStream` 風格，不引入 async）。

**Tech Stack:** Rust（`rust-version = "1.85"`，不用 let-chains）、serde、tokio-serial 5.4（僅 DataService `mm-drivers`）。

**Spec:** `docs/superpowers/specs/2026-10-18-plc-family-shared-crate-design.md`（兩 repo 路徑：`MachineModel/` 與 `DataService/`，皆在 `orca/projects/` 下；以下路徑 `MM/` = MachineModel、`DS/` = DataService）

## Global Constraints

- `machine-model` 與 `plc-family` 只依賴 `serde`（`machine-model` 另有 `serde_yaml_ng`）；不得直接或間接含 `tokio`、`tokio-serial`、`serialport`、`mio-serial`、`axum`、`reqwest`、`hyper`、`clap`。
- 不使用 let-chains；診斷訊息用繁體中文。
- 讀取端未知欄位仍拒絕；舊別名 `inovance_easy`、`mitsubishi_fx3g`、`mitsubishi_fx3s` 必須繼續可解析。
- Inovance Easy 串列參數：19200-8-E-1、Modbus RTU、CRC-16、位元組序 `CDAB`（文件值 `driver3.md`，**未經實機驗證**，註解須標明）。
- `default_port()` 對串列型號回 `0`。
- 家族總數 55 → 56。
- **不 commit**（規格非目標）。本計畫所有驗證步驟以測試輸出為準，不含 commit 步驟。
- 不還原或覆蓋 DataService 既有未提交修改（`mm-drivers/src/lib.rs`、`machine-model/src/lib.rs`、`family_table.rs` 為未追蹤檔等）；一律在原檔上編輯。

## Review Focus

1. 別名回歸：`inovance_easy` 仍解析為 `InovanceEasyTcp`，不是新的 Serial（Task 3 測試）。
2. 串列回應是 Modbus 例外幀（FC ≥ 0x80）或 CRC 錯誤：回 `Err`，不得回成數值（Task 4）。
3. 序列埠沉默（無回應）：在 `timeout_ms` 內回 `Err`，不得卡住（Task 4）。
4. 回應分兩段到達、間隔小於 `inter_frame_silence_ms`：合併為同一幀（Task 4）。
5. 串列型號但缺 `SerialPortConfig`，或埠名不存在：回 `DriverError`，不得 panic（Task 4）。
6. `StationLink::Serial` 搭配非串列型號（例如 Siemens）：維持原本「警告後用模擬回應」，不得變成錯誤（Task 4）。

---

### Task 1: 抽出 `plc-family` crate 並加依賴規則

**Files:**
- Create: `MM/plc-family/Cargo.toml`, `MM/plc-family/src/lib.rs`（由 `MM/machine-model/src/family.rs` 整檔搬入）, `MM/plc-family/tests/dependency_rules.rs`
- Modify: `MM/Cargo.toml`（members 加 `"plc-family"`）, `MM/machine-model/Cargo.toml`（加 `plc-family = { path = "../plc-family" }`）, `MM/machine-model/src/family.rs`（整檔改為 `pub use plc_family::*;`）, `MM/machine-model/tests/dependency_rules.rs`（`BANNED_BASE`）

**Interfaces:**
- Consumes: 既有 `family.rs` 的公開項目：`Family`（`ALL`、`grammar()`、`as_str()`）、`Radix`、`Width`、`ClassProfile`、`Grammar`、`OPAQUE_CLASS`。
- Produces: crate `plc-family`，路徑 `plc_family::*` 匯出以上全部，**另新增** `impl core::fmt::Display for Family`（輸出 `as_str()`；因 DataService 無法在外部 crate 為 `Family` 實作 `Display`，見 Task 2）。

- [ ] **Step 1: 寫失敗測試**：`MM/plc-family/tests/dependency_rules.rs` 仿 `MM/machine-model/tests/dependency_rules.rs`，`cargo tree -p plc-family --edges normal,build --prefix none --color never --no-default-features`；測試 `only_serde_allowed`：套件名集合去掉 `plc-family` 後，每個都必須屬於 serde 傳遞依賴（`serde`、`serde_core`、`serde_derive`、`proc-macro2`、`quote`、`syn`、`unicode-ident`），且不含 `tokio`、`tokio-serial`、`serialport`、`mio-serial`、`axum`、`reqwest`、`hyper`、`clap`。另一個測試 `manifest_declares_only_serde`：讀 `Cargo.toml` 的 `[dependencies]` 區段，鍵集合恰為 `{serde}`。
- [ ] **Step 2: 驗證失敗**：`cargo test -p plc-family`（尚無此 package）→ 預期錯誤 `package ID specification plc-family did not match`。
- [ ] **Step 3: 建立 crate**：`Cargo.toml` 沿用 workspace 的 version/edition/rust-version/publish，`[dependencies] serde = { workspace = true }`，`[lints] workspace = true`。把 `family.rs` 內容 `git mv` 到 `plc-family/src/lib.rs`（保留原有測試）；先 `grep -n "crate::" plc-family/src/lib.rs` 確認無內部依賴。在 `impl Family` 之外新增 `impl core::fmt::Display for Family`。`machine-model/src/family.rs` 只留 `pub use plc_family::*;`。
- [ ] **Step 4: 擴充既有依賴測試**：`MM/machine-model/tests/dependency_rules.rs` 的 `BANNED_BASE` 加入 `"tokio-serial"`、`"serialport"`、`"mio-serial"`。
- [ ] **Step 5: 驗證**：`cd MM && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`，預期全過；`model-debugger` 無需改動即可編譯（`machine_model::family::Family` 路徑不變）。

---

### Task 2: DataService 改用 `plc-family`

**Files:**
- Modify: `DS/crates/machine-model/Cargo.toml`（加 `plc-family = { path = "../../../MachineModel/plc-family" }`）, `DS/crates/machine-model/src/lib.rs`（約 L132–L146）
- Delete: `DS/crates/machine-model/src/family_table.rs`

**Interfaces:**
- Consumes: Task 1 的 `plc_family::{Family, Radix, Width, ClassProfile, Grammar, OPAQUE_CLASS}` 與其 `Display`。
- Produces: 不變的 `machine_model::family::{PlcFamily, ClassProfile, Grammar, OPAQUE_CLASS}` 與 `machine_model::family_table`（改為 `pub use plc_family as family_table;`，保留舊路徑給 `use super::family_table::Radix/Width`）。

- [ ] **Step 1: 建立基準**：`cd DS && cargo test -p machine-model -p mm-drivers -p plc-data-service -p datasvc-demo`，記下通過數（預期全過；若已有失敗，原樣回報，不處理）。
- [ ] **Step 2: 改寫**：`lib.rs` 的 `pub mod family_table;` 換成 `pub use plc_family as family_table;`；**刪除** `impl fmt::Display for PlcFamily { … }`（否則違反 orphan rule，編譯失敗）；刪除 `family_table.rs`。
- [ ] **Step 3: 驗證**：重跑 Step 1 指令，通過數與基準相同；`cargo tree -p machine-model` 含 `plc-family`，且 `cargo tree -p plc-family`（在 MM）仍只有 serde。

---

### Task 3: 新增家族 `inovance_easy_serial` 與 `DriverKind`／目錄／報文

**Files:**
- Modify: `MM/plc-family/src/lib.rs`（`Family`、`ALL`、`grammar()`、`as_str()`、計數測試與檔首「55」）, `MM/2026-10-16-unit-tag-model-design.md`（L98、L125「55」→「56」，§3.3 補一句：`inovance_easy_serial` 為 RS-232/485 Modbus RTU 驅動，位址表同 `inovance_easy_tcp`）, `DS/crates/mm-drivers/src/lib.rs`
- Create: `DS/crates/mm-drivers/tests/family_consistency.rs`

**Interfaces:**
- Consumes: `Family::ALL`、`INOVANCE_EASY` 位址表（既有）。
- Produces: `Family::InovanceEasySerial`（`as_str() == "inovance_easy_serial"`）；`DriverKind::InovanceEasySerial`（serde 名稱同）；`list_driver_catalog()` 多一筆 PLC 項目。

- [ ] **Step 1: 寫失敗測試**
  - `plc-family`：把 `there_are_55_plc_families_with_unique_names` 改名 `there_are_56_plc_families_with_unique_names`，兩處數字改 56；新增 `inovance_easy_serial_shares_the_easy_address_table`：`Family::InovanceEasySerial.grammar() == Family::InovanceEasyTcp.grammar()`；新增 `legacy_inovance_easy_alias_stays_tcp`：`serde_yaml_ng` 解析 `"inovance_easy"` 得 `InovanceEasyTcp`（`plc-family` 若缺 dev-dependency，用 `serde::de::value::StrDeserializer` 避免新增依賴）。
  - `DS/crates/mm-drivers/tests/family_consistency.rs`，三個測試：`every_family_has_a_driver_kind_and_plc_catalog_entry`（對 `PlcFamily::ALL` 每個，`serde_json::from_value::<DriverKind>(Value::String(f.as_str().into()))` 成功，且 `list_driver_catalog()` 有 `category == "PLC"` 且 `kind` 相同者）；`every_plc_catalog_entry_is_a_family`（反向：PLC 項目序列化名稱可轉 `PlcFamily`）；`plc_counts_match`（`PlcFamily::ALL.len() == PLC 項目數`，兩邊皆 56）。
  - `mm-drivers` 單元測試 `inovance_easy_serial_frame_is_rtu`：`build_protocol_frame(DriverKind::InovanceEasySerial, 1, "D100", 1, false)` 等於 `[0x01,0x03,0x00,0x64,0x00,0x01]` 加上 `modbus_crc16` 的兩個位元組（無 MBAP 表頭）；另 `inovance_easy_serial_defaults`：`default_byte_order()==CDAB`、`default_port()==0`。
- [ ] **Step 2: 驗證失敗**：`cd MM && cargo test -p plc-family`；`cd DS && cargo test -p mm-drivers` → 預期編譯失敗（找不到 `InovanceEasySerial`）。
- [ ] **Step 3: 實作 `plc-family`**：新增 `Family::InovanceEasySerial`（置於匯川區、`InovanceEasyTcp` 之後），`ALL` 補入，`grammar()` 與 `INOVANCE_EASY` 同臂，`as_str()` 回 `"inovance_easy_serial"`；更新檔首與 `ALL` 註解的「55」。
- [ ] **Step 4: 實作 `mm-drivers`**：`DriverKind::InovanceEasySerial`（更新檔內「56 種」註解為 57）；`default_byte_order()` 的 CDAB 臂加入；`default_port()` 回 0（與 `Simulator` 同臂）；`build_protocol_frame()` 把它加進 `ModbusRtuOverTcp` 所在的 RTU 臂（不是 `InovanceEasyTcp` 的 MBAP 臂，也不得落入 `_ =>`）；`extract_payload_from_response()` 同樣加進 `ModbusRtuOverTcp` 臂（剝 RTU 表頭、辨識例外幀）；`list_driver_catalog()` 加一筆（`category "PLC"`、vendor `Inovance 匯川`、`default_port 0`、`CDAB`、範例 `D100, M100, X010, Y010, R100`、`supports_random_read: true`）。檔案中標明「19200-8-E-1／RTU 為文件值，未經實機驗證」。
- [ ] **Step 5: 驗證**：`cd MM && cargo test --workspace`（含 `standard_pins`、`schema_pins` 不得因此失敗；若 schema 列舉家族名稱而失敗，依失敗訊息同步更新 schema 與 pin，並在回報中說明）；`cd DS && cargo test -p machine-model -p mm-drivers -p plc-data-service -p datasvc-demo` 全過。

---

### Task 4: 序列埠 I/O（同步）

**Files:**
- Modify: `DS/crates/mm-drivers/src/lib.rs`（`DriverConfig`、`UniversalPlcDriver.stream`、`connect`、`exchange_raw`、`disconnect`）, `DS/crates/datasvc-demo/src/model.rs`（`connect_plc` Serial 分支）, `DS/crates/datasvc-demo/src/web.rs`（L401 描述文字）
- Test: `DS/crates/mm-drivers/tests/serial_rtu.rs`

**Interfaces:**
- Consumes: Task 3 的 `DriverKind::InovanceEasySerial`；`serial_and_protocols::{SerialPortConfig, open_tokio_serial_builder}`；`StationLink::Serial { config, station_no }`。
- Produces:
  - `DriverConfig` 新欄位 `pub serial: Option<SerialPortConfig>`，`#[serde(default)]`，`Default` 為 `None`（既有 `..DriverConfig::default()` 的呼叫端不受影響）。
  - 私有 `enum Link { Tcp(TcpStream), Serial(Box<dyn tokio_serial::SerialPort>) }`，欄位 `stream: Option<Link>`（欄位名保留，既有 `self.stream.is_some()` 判斷不動）。
  - `exchange_raw(&mut self, tx: &[u8]) -> Result<Vec<u8>, DriverError>` 行為擴充：Serial 時寫入後讀取，首位元組等到 `cfg.timeout_ms`，之後以 `inter_frame_silence_ms` 為單次讀取逾時持續累積，靜默即視為幀結束。

- [ ] **Step 1: 寫失敗測試**（`tests/serial_rtu.rs`，整檔 `#[cfg(unix)]`）。以 pty 配對模擬 PLC（`serialport::TTYPort::pair()`；若 `tokio_serial` 未轉出該型別，改以 `socat -d -d pty,raw,echo=0 pty,raw,echo=0` 起兩端並從 stderr 取路徑，兩者都不可行時測試標 `#[ignore]` 並在回報明說「串列收發未驗證」）。從端執行緒讀請求後回覆下列位元組（CRC 用 `modbus_crc16`）：
  - `serial_read_returns_register_value`：回 `01 03 02 00 7B`+CRC，`read_single("D100","int16")` 得 `123`。
  - `serial_exception_frame_is_error`：回 `01 83 02`+CRC → `Err(DriverError::ProtocolError(_))`。
  - `serial_bad_crc_is_error`：回的幀 CRC 錯一位 → `Err`。
  - `serial_silent_port_times_out`：從端不回，`timeout_ms = 100` → 於 1 秒內 `Err`。
  - `serial_fragmented_response_is_joined`：回應拆兩段，間隔小於 `inter_frame_silence_ms` → 值正確。
  - `serial_without_config_is_error`：`serial: None`、`loopback_emulation=false` → `connect` 回 `Err`（`InvalidAddress`），不 panic。
  - `serial_missing_port_is_error`：`port_name = "/dev/does-not-exist"` → `Err(IoError)`。
  - `loopback_emulation_needs_no_port`：`InovanceEasySerial` + `loopback_emulation=true`、`serial: None` → `connect` 與 `read_single` 成功（G：沒有實體埠仍可測）。
  - `datasvc-demo`：`connect_plc("p", PlcFamily::SiemensS7_1200, StationLink::Serial{..})` 回 `Ok`（Review Focus 6：維持警告後模擬）；`connect_plc("p", PlcFamily::InovanceEasySerial, StationLink::Loopback)` 回 `Ok`。
- [ ] **Step 2: 驗證失敗**：`cd DS && cargo test -p mm-drivers --test serial_rtu` → 預期編譯失敗（無 `serial` 欄位）。
- [ ] **Step 3: 實作 `DriverConfig.serial` 與 `Link`**：`connect()` 在 `kind == InovanceEasySerial && !loopback_emulation` 時走序列分支：`cfg.serial` 為 `None` → `DriverError::InvalidAddress("序列埠驅動需要 SerialPortConfig")`；否則 `open_tokio_serial_builder(&sc).timeout(Duration::from_millis(sc.inter_frame_silence_ms.max(1))).open()`，失敗映射 `DriverError::IoError`。其餘 kind 的 TCP 分支不變。連線交握改為對串列送一個 `D100` 長度 1 的 RTU 讀取幀，失敗回傳 `Err`（與 TCP 一致，但回應不合法也算失敗）。
- [ ] **Step 4: 實作串列 `exchange_raw`**：`rs485_rts_enable` 為真時，寫入前 `write_request_to_send(true)`、`flush()` 後 `false`；讀取迴圈如介面所述。回應驗證：長度 ≥ 5、CRC 以 `modbus_crc16` 檢查，失敗回 `DriverError::ProtocolError("CRC 錯誤…")`；例外幀交給既有 `extract_payload_from_response` 的 RTU 臂。`disconnect()` 清掉 `stream`。
- [ ] **Step 5: 接上設定**：`connect_plc` 的 `StationLink::Serial { config, station_no }`：若 `cfg.kind == InovanceEasySerial`，設 `cfg.serial = Some(config)`、`cfg.station = station_no`、`cfg.loopback_emulation = false`；其他 kind 維持原 `warn!` 與模擬行為。`web.rs` L401 文字改為依 kind 區分（串列型號：「序列埠（Modbus RTU）」；其他：維持原文）。
- [ ] **Step 6: 驗證**：`cd DS && cargo test -p machine-model -p mm-drivers -p plc-data-service -p datasvc-demo` 全過（含 `tight_timeout.rs`）；`cd MM && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` 全過；`cd MM && cargo tree -p machine-model -e normal,build | grep -E "tokio|serialport|mio-serial"` 無輸出。DataService 以 `cargo clippy`／`cargo fmt` 檢查**僅限本次改動檔**（該 repo 既有程式碼未必乾淨，既有警告原樣回報）。
- [ ] **Step 7: 回報**：明確列出「未用實體 PLC 驗證」；若 pty 測試跑過，說明是 pty 配對而非實機；RS-485 RTS 時序同為未驗證。

---

## 自我檢查

- 規格覆蓋：G1→T1、G2→T1/T2（`model-debugger` 與 `driver_for_family` 都吃 `Family::ALL`／serde 名稱）、G3→T3/T4、G4→T3 一致性測試、§7 依賴規則→T1、§6.3 目錄與預設值→T3、§6.4 loopback 保留→T4。
- 規格未寫、計畫補上的三處：Display 搬到 `plc-family`（orphan rule）；`DriverConfig.serial` 欄位；同步 `open()` 而非 async。
