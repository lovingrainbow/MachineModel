# machine-model 融合計畫（盤點與路線）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. 本計畫的第 1 階段只產出盤點與差異測試，不改產品碼；第 2 階段起每一階段開始前，先依盤點結果補上該階段的細部 task（writing-plans）。

**Goal:** 把 `MachineModel/machine-model` 與 `DataService/crates/machine-model` 融合成**一個** crate，DataService、PLCDriver 工具、設計期 CLI 都用同一份。

**終點依賴：** `DataService → MachineModel/machine-model → plc-common`；PLCDriver 的 `mm-drivers` 只依賴 plc-common，維持不變。

**Architecture:** 以 `MachineModel/machine-model`（模組化、受規格 `2026-10-16-unit-tag-model-design.md` 管轄、標準類型有雜湊釘選）為唯一本體。DataService 版獨有的功能逐項移植：屬於純模型的進本體；需要 `toml`／`serde_json`／`clap` 等相依的拆到本體之外的下游 crate，維持 AGENTS.md「本體只依賴 serde、serde_yaml_ng、plc-common」。移植完成並通過差異測試後，刪除 DataService 版。

**Tech Stack:** Rust（本體 edition 2024、rust-version 1.85）、serde／serde_yaml_ng、plc-common。

**Spec:** `2026-10-16-unit-tag-model-design.md`（根目錄）。欄位與規則要改，先改規格。

## 現況盤點（2026-10-18，已驗證項與未驗證項分開標示）

### 規模與形態（已驗證）

| 項目 | MachineModel 版 | DataService 版 |
|---|---|---|
| 形態 | 模組化，約 4900 行（`address`、`analyze/`、`catalog`、`naming`、`resolve`、`rules/`、`schema`、`st_check`、`standard`、`strict`、`types`、`unit_type`…） | 單檔 `lib.rs` 4079 行，內含 `diagnostics`、`family`、`datatype`、`naming`、`unit_type`、`model`、`validator`、`resolver`、`lowering`、`datasvc`、`cli`、測試 |
| 相依 | plc-common、serde、serde_yaml_ng；`schema` feature 才有 jsonschema、serde_json | plc-common、serde、**serde_json**、serde_yaml_ng；jsonschema 在 `schema` feature |
| 版本／edition | 0.1.0，edition 2024 | 0.3.0，workspace edition |
| 設計期工具 | `machinemodel` CLI（check、dump、check-types、write-report）、`tools/model-debugger` | `cli` 模組（check、dump、check-types、write-report、lower-st、emit-schema、debugger） |
| 消費者 | `machinemodel` CLI、model-debugger | `plc-data-service`、`datasvc-demo`、`DataService/src-tauri` |

### 兩邊的公開 API 不同名（已驗證）

| 概念 | MachineModel | DataService |
|---|---|---|
| 解析入口 | `parse`／`load`／`load_checked(text, &UnitCatalog)` | `MachineFile::from_yaml_str`、`MachineModelBuilder`、`UnitInstanceBuilder` |
| 類型目錄 | `catalog::UnitCatalog`（`Built`、`with_st`、`st_source`） | `unit_type::TypeCatalog`（`with_embedded_standards`、`load_local_types_from_dir`） |
| 推導結果 | `resolve::ResolvedModel`（`tags`、`units`、`pins`、`fingerprint`） | `resolver::ResolvedMachineModel`（`EffectiveTag`、`ResolvedUnitNode`、`DerivedOperationItem`） |
| 診斷 | `diag::Diagnostic` | `diagnostics::Diagnostic` |
| 標量 | `schema::Scalar`、`schema::Scan` | `model::ScalarValue`、`model::ScanRate` |
| 家族 | `family::Family` | `family::PlcFamily`（皆為 plc-common 的別名） |

DataService 的 `plc-data-service`／`datasvc-demo` 實際用到的 API：`PlcFamily`、`ScalarValue`、`ScanRate`、`TypeCatalog`、`resolve_machine_model`／`ResolvedMachineModel`、`MachineModelBuilder`／`UnitInstanceBuilder`、`MachineFile`、`PlcDef`、`TagDef`、`Polarity`、`Diagnostic`、`datasvc::{TagOverrideEntry, evaluate_plc_data_service_writability}`。

### 規則編號覆蓋（已驗證，取自程式碼字串；規則實作是否等價**未驗證**）

- 兩邊共有：R1 R2 R5 R6 R8 R9 R17 R18 R19 R21 R25 R26 R27 R28、SC2 SC3、U1～U15 U17 U18 U19。
- **只有 DataService**：SC1、SC4、U16、DS1～DS3（`[[tag_override]]` 護欄）。
- **只有 MachineModel**：R22（未知欄位）、R24、SC5、PARSE、U21～U24（ST 旁檔）。
- 註：DataService 的 R22／R24 在文件字串中出現，但程式碼沒有同名診斷碼；它的未知欄位攔截走 `deny_unknown_fields`，歸類成 SC4。這是**診斷編號不一致**，融合時要由規格決定。

### 功能差距（已驗證存在與否；細節行為未驗證）

**只在 DataService，需移植：**

1. §7 降階 `lowering::lower_to_iec_st`（`<type>_ctrl_t` 結構與頂層 FB 實例）。MachineModel 的計畫 4（ST 匯出）要做的正是同一件事，兩邊要合併成一份。
2. §9.3 `datasvc`：`service.toml` 的 `[[tag_override]]` 解析與最終 `writable` 計算（DS1～DS3）。需要 `toml`，**不得進本體**。
3. `MachineModelBuilder`／`UnitInstanceBuilder`：以程式建立模型（datasvc-demo 互動模式在用）。
4. `EffectiveTag` 的 `role_reason`、`write_candidate_reason`，以及 HMI 的 `DerivedOperationItem`（`man_`／`ena_`／`at_`／`sts_` 聚合）。MachineModel 的 `ResolvedTag` 沒有「理由」字串，也沒有 HMI 聚合。
5. 編輯距離 1 的拼寫提示（`edit_distance`，U 規則的提示）與 HMI 覆蓋率報告（U 規則 §10）。MachineModel 是否已有等價實作，**未驗證**。
6. `validate_unit_type_with_jsonschema`、`emit-schema`、`lower-st` CLI 子命令。
7. CLI 的 `debugger` 子命令（MachineModel 以獨立的 `tools/model-debugger` 取代，需對照）。

**只在 MachineModel，DataService 沒有（融合後下游自動獲得）：**

1. 標準類型的雜湊釘選（`standards/unit-types.pins`、`hash.rs`、`tests/standard_pins.rs`）。
2. `.st` 旁檔載入與 U21～U24 檢查（`st_check.rs`，計畫 4 進行中）。
3. 圖分析（`analyze/graph.rs`）與接腳分析（`analyze/pins.rs`）。
4. 載入安全規則集中於 `rules/`，並有 `strict.rs`（嚴格讀取）。
5. 推導結果帶 `Fingerprint`（`source_hash`、`type_versions`）。

### 資料不相容（已驗證）

用 MachineModel 的 `machinemodel check` 檢查 DataService 自己的範例 `datasvc-demo/config/machine.yaml`，得到 **U2** 與 **U3** 錯誤：
- U2：範例的 `machine_ctrl` 是 `type_version: 1`，MachineModel 標準目錄的 `machine_ctrl` 是 version 2。
- U3：範例 `st1` 的 `bind` 用了 `error_jam`，MachineModel 的 `station_load` 類型沒有這個接腳或 part。

結論：兩邊**內建的標準 Unit 類型內容已經分歧**（DataService 在 `lib.rs` 內嵌三份 YAML 字串，MachineModel 在 `standards/unit-types/` 且有釘選）。融合時標準類型以 MachineModel 的為準（規格：只有人確認過的類型才能進標準目錄），DataService 的範例與測試資料要跟著升版。

### 尚未驗證（階段 1 要補的）

- 同一份輸入在兩邊的診斷編號與數量是否一致（只有上面一組樣本）。
- 兩邊 `ResolvedTag` 與 `EffectiveTag` 的欄位逐項對應與語意差（例如 `readback`／`retain` 在 MachineModel 是 `Option<bool>`，在 DataService 是 `bool`）。
- DataService 的規則實作是否漏規則，或比 MachineModel 嚴／寬。
- 兩邊規格版本是否同源：DataService 檔頭自稱 v0.3，MachineModel 規格也記為 v0.3（`1a67fbf`），但沒比對條文。
- `DataType`（DataService 的 `datatype`）與 MachineModel 的 `types::DataType` 是否只差命名。

## 目標架構

```
plc-common                      家族表、ScalarType、ScalarValue（已完成）
  └─ machine-model              唯一本體：解析、規則、目錄、推導、ST 檢查（只依賴 serde、serde_yaml_ng）
       ├─ machine-model-export  （新）降階與 ST 產生；可依賴更多東西時才拆
       ├─ machine-model-service （新）service.toml、tag_override、最終 writable（需要 toml）
       └─ 工具與消費者          machinemodel CLI、model-debugger、plc-data-service、datasvc-demo
```

「是否另拆 export／service crate」是階段 2 的決策點：降階若只需 serde 就留在本體內（模組 `lower`）；`service.toml` 需要 `toml`，必須在本體之外。

## Global Constraints

- **終點（使用者目標）**：DataService 的 `plc-data-service`、`datasvc-demo`、`src-tauri` 直接依賴 `../MachineModel/machine-model`，`DataService/crates/machine-model` 刪除，整個 `projects/` 之下只剩一份 machine-model；依賴方向維持 plc-common ← MachineModel ← DataService，不得反向。
- **依賴方式（使用者決定：git 為主）**：專案之間的依賴一律用 GitHub 的 git tag（`lovingrainbow/*`），不再要求並排目錄；階段 5 起 DataService 對 MachineModel 的依賴用 `git + tag`，需先在 MachineModel 打 tag。開發期若需要本機聯調，用 Cargo 的 `[patch]` 暫時指向本機路徑，不 commit。
- **凍結規則（防止進度再分岔）**：從本計畫開始，`DataService/crates/machine-model` 不再新增規則或功能；任何模型相關的新增與修正只進 `MachineModel/machine-model`。階段 5 完成前若 DataService 需要新規則，先做在 MachineModel，再由 DataService 取用。
- **融合不得增加使用者設定的複雜度**（使用者要求）：`machine.yaml` 與 Unit 類型檔的欄位集合以 MachineModel 現行為準，**零新增欄位、零新增 feature flag、零新增模式或旗標**；DataService 獨有的欄位（例如 variables 的 `label`）移除而不是搬過來。新增的只能是程式內部的 API、推導結果與診斷輸出，不能要求使用者在 YAML 或 CLI 多做任何選擇。由 `tests/schema_pins.rs` 與 `tools/model-diff` 把關：融合前後 `machine.yaml` 與類型檔的 JSON Schema 必須逐字相同。
- 本體只依賴 `serde`、`serde_yaml_ng`、`plc-common`；不得依賴 `tokio`、`axum`、`reqwest`、`hyper`、`clap`、`toml`；`jsonschema` 只在 `schema` feature 之後（`tests/dependency_rules.rs` 把關）。
- 讀取端拒絕未知欄位，擴充走 `ext`。Tag 沒有 `unit_id`、`writable`、`write_permission`。
- 診斷訊息用繁體中文並帶規則編號（`R…`、`U…`、`SC…`、`DS…`）。
- 不使用 let-chains（`rust-version = "1.85"`）。
- 標準 Unit 類型只能由人確認；改內容必須調高 `version`，並更新 `standards/unit-types.pins`。
- 不合併時不可讓 DataService 或 PLCDriver 的測試變紅；每階段結束三個專案的 `cargo test --workspace` 都要過。
- 計畫 4（ST 匯出）正由另一個 agent 進行；本計畫不得改動 `st_check.rs`、`catalog.rs` 的 ST 相關部分、`standard.rs`、`hash.rs`，降階的合併在計畫 4 完成後才開始。

## Review Focus

- 同一份 `machine.yaml` 在融合前後的診斷編號集合要相同，不可靜默放寬或收緊規則（尤其 U 規則與 R19 極性）。
- `write_candidate` 的 fail-closed：沒有綁定一律 `false`；融合後兩邊結果必須逐 Tag 相同。
- 最終 `writable`（DS1～DS3）不得因為搬家而變成預設可寫。
- 標準類型 version 與釘選：DataService 範例升版後仍要通過 U2。
- 本體相依樹不得出現 `toml`／`serde_json`（未開 `schema` feature 時）。

---

## 階段 1：差異測試（只加測試與報告，不改產品碼）

**Files:**
- Create: `MachineModel/docs/superpowers/plans/2026-10-18-machine-model-unification-gap-report.md`（盤點結果，逐項標 已驗證／未驗證→已驗證）
- Create: `MachineModel/tools/model-diff/`（新工作區成員，dev 工具，不發佈）：同時依賴兩個 crate（DataService 版以 `package` 改名引入），讀一組 fixtures，輸出兩邊的診斷碼集合與每 Tag 的 role／write_candidate 比對
- Create: `MachineModel/tools/model-diff/fixtures/`：MachineModel 的 `structural_full.yaml`、`valid_units.yaml`、DataService 的 `machine.yaml`，再加每個共有規則至少一個違規樣本

**Interfaces:**
- Produces: 一份「規則 × 兩邊是否觸發」表與一份「Tag × 欄位」差異表，作為階段 2 的輸入。

- [x] **Step 1:** 建立 `tools/model-diff`，用 Cargo 的 `package = "machine-model"` 改名同時引入兩個 crate；確認 `cargo build -p model-diff` 過（需處理兩 crate 同名與 plc-common 共用）。
- [x] **Step 2:** 對所有 fixtures 跑兩邊的 load／驗證，列出診斷碼差異；測試名稱 `rule_codes_match_for_shared_rules`，目前預期**失敗**（至少 U2／U3 那組），把失敗當作差異清單的來源，不要為了變綠而改斷言。
- [x] **Step 3:** 對 `valid_units.yaml` 比對每個 Tag 的 `role`、`write_candidate`、`address_read_only`／`class_read_only`、`bindings`；差異寫入報告。
- [x] **Step 4:** 完成 gap-report，把本計畫「尚未驗證」五項逐一改成結論。
- [x] **Step 5:** `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check` 在 MachineModel 通過（差異測試以 `#[ignore]` 或獨立 `--example` 呈現，避免讓主驗證變紅）。

## 階段 2：規格對齊與決策（需要使用者）

由 gap-report 產出決策清單，使用者確認後才進階段 3。預期決策點：

1. 診斷編號：DataService 的 SC1／SC4／U16 與 MachineModel 的 R22／SC5 如何統一（改規格）。
2. 降階與 ST 匯出：以 DataService `lowering` 為底、還是以計畫 4 的實作為底；放本體模組或另拆 crate。
3. `service.toml`／最終 writable：放哪個 crate（建議 `machine-model-service`，因為要 `toml`）。
4. `Builder` API 預設**不搬**：它是 Rust 內部 API，不是使用者設定，但搬了就是多一組要維護的建立方式。先查 datasvc-demo 互動模式是否真的需要；若可改用 `schema::MachineModel` 直接建構，就不搬。
5. `EffectiveTag` 的理由字串與 `DerivedOperationItem` 是否納入 `ResolvedModel`。
6. `DataType` 統一方式（MachineModel 的 `types::DataType` 為準，DataService 的 `datatype` 對應）。
7. 標準類型：以 MachineModel 的 `standards/` 為準，DataService 範例升版的處理方式。

## 階段 3 之前的實測：消費者真正需要什麼（2026-10-18 已驗證）

用 grep 查 `plc-data-service`、`datasvc-demo`、`src-tauri` 對 DataService 版 machine-model 的實際使用：

| DataService 版功能 | 消費者是否使用 | 處置 |
|---|---|---|
| `effective_tags`（`id`、`address`、`data_type`、`scan`、`deadband`、`min`/`max`、`default`、`safety_monitor`、`write_candidate`、`bindings[].category`） | 是（`plc-data-service`、`datasvc-demo`） | MachineModel 的 `ResolvedTag` 已有等價欄位；`category == "error"` 對應 `bindings[].prefix == Some(Prefix::Error)` |
| `write_candidate_reason`（顯示在 `datasvc-demo/web.rs`） | 是 | MachineModel 沒有；以 `ResolvedTag` 上的**方法**補（由 `bindings` 即時組字串），不加欄位 |
| `MachineModelBuilder`、`UnitInstanceBuilder` | 是，但只在測試與 `datasvc-demo/model.rs` 的內建示範機台 | **不搬**：示範機台與測試改成 YAML 文字常數，`parse` 後使用 |
| `MachineFile` 的就地修改（`editor.rs`） | 是 | MachineModel 的 `schema::MachineModel`、`Tag`、`Plc` 是公開結構，可直接修改後重新 `load_checked` |
| `datasvc`（`[[tag_override]]`、DS1～DS3、最終 writable） | 是（只有 `plc-data-service`） | 規格 §6.3 明說最終 writable 不在模型專案：整個模組**搬進 `plc-data-service`**，不進 MachineModel。解析器是手寫的，沒有 `toml` 相依 |
| `lowering`、`cli`（`lower-st`、`emit-schema`、`debugger`） | 否 | 不搬：降階由 P4 的 `export-st` 取代，CLI 由 `machinemodel` 取代 |
| `DerivedOperationItem`／`lists`／`hmi_coverage`／`unit_tree` | 否 | 不搬（沒有消費者；覆蓋率 MachineModel 已有 `coverage_gaps()`） |
| DataService 版的規則實作與診斷編號 | 否（消費者只印診斷） | 不搬；融合後用 MachineModel 的 |

結論：**MachineModel 本體幾乎不需要新增功能**，只新增一個 `ResolvedTag` 方法。階段 3 因此縮成一個小任務；工作量集中在階段 5（DataService 換依賴）。這也符合「使用者設定不變複雜」的限制：YAML、類型檔、CLI 全部零新增。

## 階段 3：補齊 MachineModel（唯一的本體新增）

### Task 3.1: `ResolvedTag::write_candidate_reason`

**Files:**
- Modify: `machine-model/src/resolve.rs`（P4 不碰此檔）
- Test: `machine-model/tests/resolve.rs`

**Interfaces:**
- Produces: `impl ResolvedTag { pub fn write_candidate_reason(&self) -> String }`，供 `datasvc-demo` 顯示。

- [x] **Step 1: 失敗測試 `write_reason_names_the_binding_pin`**：用 `valid_units` fixture，`plc1.M_Cyl1_ManExt` 綁在 `cyl1.man_ext`，斷言回傳字串含 `cyl1.man_ext` 與前綴 `man`，且 `write_candidate == true`；`plc1.X_Cyl1_Ext_LS`（field 腳）斷言字串說明未綁定可寫候選接腳、`write_candidate == false`；沒有綁定的 `plc1.M_Spare` 斷言字串說明「沒有綁定，fail-closed」。
- [x] **Step 2:** 執行確認失敗。
- [x] **Step 3:** 依 `bindings` 組字串，不新增欄位、不動既有輸出。文字繁體中文，措辭沿用 DataService 版（`綁定至可寫候選接腳 '<unit>.<pin>' (<prefix>_*)`）。
- [x] **Step 4:** `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check`。
- [x] **Step 5: Review Focus 測試** `write_candidate_is_fail_closed_without_bindings`：沒有任何綁定的 Tag，`write_candidate == false` 且理由字串非空。

## 階段 4：把 `datasvc` 搬進 `plc-data-service`

### Task 4.1: 搬移 `datasvc` 模組

**Files:**
- Create: `DataService/crates/plc-data-service/src/writable.rs`（來源：`DataService/crates/machine-model/src/lib.rs` 的 `pub mod datasvc`）
- Modify: `DataService/crates/plc-data-service/src/lib.rs`（`use crate::writable::{...}`）
- 搬移時一併搬 `datasvc` 的既有測試

**Interfaces:**
- Consumes: 階段 5 之前仍用 DataService 版 `ResolvedMachineModel`；階段 5 再改成 `machine_model::resolve::ResolvedModel`。
- Produces: `TagOverrideEntry`、`evaluate_plc_data_service_writability`、`parse_service_toml_overrides`（簽名不變）。

- [x] **Step 1:** 搬移既有 DS1～DS3 測試到新位置，在舊位置保留並確認兩邊都過（階段 5 才刪舊的）。
- [x] **Step 2:** `cargo test -p plc-data-service` 全過；`datasvc-demo` 的 `service.toml` 範例結果與搬移前逐字相同。
- [x] **Step 3: Review Focus 測試** `override_cannot_make_safety_monitor_tag_writable`：護欄不因搬家而放寬。

## 階段 5：DataService 改用 MachineModel（最大工作量）

### Task 5.1: 標準類型與範例資料升版
- 以 MachineModel `standards/` 為準；`datasvc-demo/config/machine.yaml` 升到 `machine_ctrl` version 2，修正 `st1` 的 `bind` 使其符合 `station_load`（階段 1 實測：目前報 U2、U3）。
- [x] 驗證：`cargo run -p machinemodel -- check DataService/crates/datasvc-demo/config/machine.yaml` 通過。

### Task 5.2: `plc-data-service` 換型別
- `DataService/crates/plc-data-service/Cargo.toml` 依賴改指向 `../../../MachineModel/machine-model`；`effective_tags` → `tags`、`data_type` → `ty`、`ScanRate` → `Scan`、`bindings[].category == "error"` → `prefix == Some(Prefix::Error)`、`readback`／`retain` 的 `Option<bool>` 在**消費端** `unwrap_or(false)`（規格預設為 false，查 §3.2 確認後再寫）。
- 測試內的 Builder 改為 YAML 文字常數。

### Task 5.3: `datasvc-demo` 換型別
- `model.rs` 的內建示範機台改 YAML 常數；`editor.rs` 直接修改 `schema::MachineModel` 並重新 `load_checked`；`web.rs` 用 `write_candidate_reason()`。
- 保留既有的 27 個測試，斷言語意不變（只換型別），不得為了變綠而放寬斷言。

### Task 5.4: `src-tauri` 換依賴
- 同步改 Cargo 路徑與型別；`cargo check -p plc-data-service-tauri`（該 crate 的 lib 測試在 WSL 內會 SIGSEGV，是環境問題，不納入判定）。

**階段 5 狀態（2026-10-18）：5.1～5.4 已完成並 commit（路徑依賴）；尚待：切換為 git tag 依賴（需 MachineModel 打 tag 並推送）。**

**階段 5 完成條件：** DataService `cargo test --workspace`（除 tauri 的 SIGSEGV 項外）全過、clippy 不增加警告、`datasvc-demo` 手動跑得起來，且 `DataService/crates/machine-model` 已無任何人引用。

## 階段 6：刪除與收尾
- [x] 刪除 `DataService/crates/machine-model`；從 DataService workspace members 移除。
- [x] 更新 `DataService/README.md`、`USAGE.md`、`DATA_SERVICE_SPEC.md` 連結（原本指向 DataService 版 `lib.rs` 的兩處改為 `MachineModel/machine-model/src/resolve.rs` 等實際位置）、`MachineModel/AGENTS.md`（說明 DataService 依賴本專案）、規格 §8 風險段。
- 全域搜尋 `DataService/crates/machine-model` 無殘留；三個專案 `cargo test --workspace`、clippy、fmt 通過。
- [x] 保留 `tools/model-diff` 與否：融合後 DataService 版已不存在，工具失去用途，**刪除**。

**階段 6 狀態（2026-10-18）：已完成（`tools/model-diff` 已刪；差距報告保留作紀錄）。DataService 對 MachineModel 的依賴已改為 git tag v0.2.0（已推送）；`dataservice*.md`、`Upgrade.md` 是歷史對話紀錄，未更新。**

## 風險

- DataService 工作樹有使用者未 commit 的修改（本計畫開始時尚未處理）；階段 5 動 DataService 前先確認它的 commit 狀態。
- 計畫 4 與階段 3／4 會碰同一批檔案，需等計畫 4 完成再動降階。
- DataService 沒有針對規則等價性的現成測試，階段 1 的差異工具本身要小心不要引入另一份「看起來對」的比對邏輯：比對的是兩邊真實輸出，不是手寫期望值。
- PLCDriver、plc-common 已有 git；DataService 的 commit 狀態未知。
