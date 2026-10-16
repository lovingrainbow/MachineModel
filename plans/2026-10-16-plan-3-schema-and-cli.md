# Plan 3: Schema Feature 與 machinemodel 命令列工具實作計畫

> **規格依據**：`2026-10-16-unit-tag-model-design.md` §8.2、§8.5、§9.2、§9.3、§9.5、§10、§12 第 2～3 項。  
> **前置工作**：Plan 1（基礎架構、型別與載入安全規則）與 Plan 2（Unit 類型、實例規則、模型推導）已全數完成並經 131 個測試驗證。  
> **目標**：  
> 1. 為 `machine-model` 實作可選的 `schema` feature（`jsonschema 0.58.x`，`default-features = false`），由 `Prefix::ALL` 遍歷前綴屬性自動產生 `unit-type.schema.json` 並提供 SC2／SC3 的 JSON Schema 驗證 API 與一致性測試。  
> 2. 為 `ResolvedModel` 及相關推導型別加上 `#[derive(Serialize)]`。  
> 3. 在 workspace 建立獨立 binary crate `machinemodel`，實作 `check`、`dump`、`check-types`、`write-report` 四大子命令，嚴格遵守退出碼規範（0: 成功, 1: 診斷錯誤, 2: 參數或 I/O 錯誤）與 §9.3 的 4 大安全護欄。

---

## 實作現況與修訂紀錄

- **核對日期**：2026-10-16（依 repo 現況）。
- **檔案已存在、`cargo test --workspace --all-features` 全綠**：Task 1（feature 與依賴測試）、Task 2（schema 產生器與釘選，含舊屬性，須依本次修訂重產）、Task 3（`validate_unit_type`）、Task 4（crate 骨架）、Task 5（`check`）。下列 Task 的「Create」指目標狀態，實作時先核對現有檔案，不要重建。
- **骨架（會假成功）**：Task 6 的 `dump`、`check-types` 與 Task 7 的 `write-report` 目前直接回 0；Task 6 Step 1（`Serialize`）尚未做。
- **本次修訂**（依審查結論，規格 v0.3 同步更新）：
  1. 移除接腳的 `kind`、`level`、`reset`、`role`（新增 Task 0；Tag 層級的 `role` 不在此列）。
  2. `par_` 的 `min`、`max`、`default` 改為必填。
  3. 字串陣列（`string:N[n]`）明確禁止。
  4. 機台本地類型自動讀取機台資料夾的 `types/`，移除 `--local-types` 旗標。
  5. U17 改為檢查原始檔的重複 id；與標準同 id 只產生覆蓋提示。
  6. Task 3 新增 Schema-only 函式，讓一致性測試能獨立判定 Schema。
  7. 最終驗證的 grep 改為模型側欄位定義檢查，並以 `!` 讓違規非零退出。

---

## 架構與決策摘要

1. **依賴嚴格隔離**：
   - `machine-model` 函式庫本體在未啟用 feature 時，維持**僅依賴 `serde` 與 `serde_yaml_ng`**。
   - 啟用 `schema` feature 時，僅可引入 `jsonschema 0.58.x`（`default-features = false`）與 `serde_json`。
   - `machinemodel` 二進位 crate 獨立於 `machine-model` 本體之外，由其單獨依賴 `clap`（v4 derive）、`toml`（0.8）。
   - `dependency_rules.rs` 升級為 feature-aware，以多組 `cargo tree` 命令把關：無論是否啟用 feature，`tokio`、`axum`、`reqwest`、`hyper`、`clap` 絕對不可進入 `machine-model` 本體依賴樹。
2. **唯一事實來源（Single Source of Truth）**：
   - `standards/schema/unit-type.schema.json` 由程式碼（透過 `Prefix::ALL` 遍歷 `Prefix::info()` 屬性）動態產生，並作為靜態資產提交進版本庫。
   - 設立自動化測試確保「版本庫中的 Schema 檔案 ≡ 產生器最新輸出」，若修改前綴表而未重新產生 Schema 檔案，CI 直接失敗。
3. **序列化策略（Serialize Strategy）**：
   - 採**作法 A**：直接在 `machine-model` 函式庫的 `ResolvedModel`、`ResolvedPlc`、`ResolvedUnit`、`ResolvedTag`、`TagBinding`、`ResolvedPin`、`Fingerprint`、`RoleSource` 加上 `#[derive(Serialize)]`。零額外依賴、零維護成本，並利於未來所有下游消費者（CLI、PlcDataService 匯出、Web API）共用。
4. **輸出格式（dump Format）**：
   - 預設為 `YAML`（符合機台設定檔的審查習慣與 diff 友善度），支援 `--format json`（或 `-f json`）輸出 pretty JSON。
5. **服務設定讀取（service.toml Scope）**：
   - 採**只讀子集**：CLI 僅解析 `[[tag_override]]` 區塊（`id` 與 `writable`），透過 serde 忽略其餘通訊設定（如 IP、Port、Baud rate），達成與通訊運行期的鬆耦合。
6. **類型目錄來源（自動讀取，無旗標）**：
   - 標準類型內嵌於函式庫（`standards/unit-types/`）。
   - 機台本地類型讀取「`machine.yaml` 所在資料夾」下的 `types/*.yaml`，同 id 時覆蓋標準並產生提示。
   - `machinemodel` 的 `check`、`dump`、`write-report` 與 PlcDataService 採同一規則；不提供 `--local-types`，避免每次都要手動指定路徑。

---

## 任務切分與實作步驟

### 第一階段：`schema` Feature（規格 §8.2、§12 第 2 項）

---

### Task 0: 移除舊接腳屬性 `kind`／`level`／`reset`／`role`（規格 §4.1 修訂，前置修正）

**Files:**
- Modify: `machine-model/src/unit_type.rs`：`CtrlPin` 移除 `kind`、`role`、`level`、`reset` 欄位；移除 `Kind`、`Level`、`Reset` 型別與其解析、測試。
- Modify: `machine-model/src/resolve.rs`：`ResolvedPin` 移除 `kind`、`level`、`reset`；刪除接腳層 `role` 覆寫分支（`pin_role` 只由前綴決定）。**Tag 層級的 `role`（規格 §3.2、§6.1）保留，不動。**
- Modify: `standards/unit-types/machine_ctrl.yaml`：移除 `error_estop` 的 `level`、`reset`；`version` 由 1 調為 2。
- Modify: `standards/unit-types.pins`：依 `tests/standard_pins.rs` 的 `print_pins` 重新產生（`machine_ctrl 2 <雜湊>`）。
- Modify: 受影響的 fixture 與測試的 `type_version`／期望版本：`machine-model/tests/fixtures/valid_units.yaml`、`valid_full.yaml`（如有）、`machine-model/tests/resolve.rs:232`（`("machine_ctrl", 1)` → 2）、`machinemodel/tests/cli_check.rs`。

**Steps:**
- [ ] **Step 1: 移除上列欄位與型別，`cargo build -p machine-model` 確認無殘留引用**
- [ ] **Step 2: 改 `machine_ctrl.yaml` 並調高 `version`，重新產生 pins（AGENTS 規定內容改變必須調高 version）**
- [ ] **Step 3: 同步 fixture 與測試的 `type_version`，`cargo test --workspace --all-features` 全綠**

**注意（副作用）**：`error_estop` 原本的 `emergency` 等級會隨之消失。若之後需要區分緊急停止，須另議（不在本次範圍）。

---

### Task 1: Feature 管線與依賴邊界測試

**Files:**
- Modify: `machine-model/Cargo.toml`
- Modify: `machine-model/tests/dependency_rules.rs`

**Interfaces:**
- `machine-model/Cargo.toml`:
  - `jsonschema = { version = "0.58.6", default-features = false, optional = true }`
  - `serde_json = { version = "1.0", optional = true }`
  - `[features]` -> `schema = ["dep:jsonschema", "dep:serde_json"]`
- `tests/dependency_rules.rs`（擴充為兩個測試函式，`cargo tree` 固定用 `--edges normal,build --prefix none --color never`，包名以空白分割後精確比對，避免 `clap_builder` 誤判為 `clap`）：
  - `fn no_features_banned()`：以 `cargo tree -p machine-model --no-default-features` 執行，禁止：`tokio`, `axum`, `reqwest`, `hyper`, `clap`, `clap_builder`, `clap_derive`, `jsonschema`, `serde_json`。
  - `fn schema_feature_banned()`：以 `cargo tree -p machine-model --features schema` 執行，禁止：`tokio`, `axum`, `reqwest`, `hyper`, `clap`, `clap_builder`, `clap_derive`；`jsonschema`、`serde_json` 必須出現（正向斷言 feature 確實啟用）。

- [ ] **Step 1: 修改 `machine-model/Cargo.toml` 宣告可選依賴與 feature**
- [ ] **Step 2: 擴充 `tests/dependency_rules.rs` 涵蓋無 feature 與含 feature 的雙重驗證**
- [ ] **Step 3: 執行 `cargo test -p machine-model --test dependency_rules` 確認通過**
- [ ] **Step 4: Checkpoint: `cargo clippy --workspace --all-targets -- -D warnings`**

---

### Task 2: Schema 產生器與檔案釘選

**Files:**
- Create: `machine-model/src/schema_gen.rs`
- Modify: `machine-model/src/lib.rs`（條件導出 `schema_gen`）
- Create: `standards/schema/unit-type.schema.json`
- Create: `machine-model/tests/schema_pins.rs`

**修訂注意**：既有 `schema_gen.rs` 含 `kind`、`level`、`reset`、`role` 與 `par_` 的選填 `min`／`max`／`default`，須依下列 SC2／SC3 重寫，並重產 `standards/schema/unit-type.schema.json`。

**Feature Gate（必須正確，否則 `cargo test --workspace` 不帶 feature 時編譯失敗）：**
- `lib.rs` 在現有模組清單中加入：`#[cfg(feature = "schema")] pub mod schema_gen;`
- `tests/schema_pins.rs` 頂層加：`#![cfg(feature = "schema")]`

**Interfaces:**
- `schema_gen::generate_unit_type_schema() -> String`（`#[cfg(feature = "schema")]`）:
  - 透過 `Prefix::ALL.iter().map(|p| p.info())` 遍歷 10 組前綴屬性（方向、角色、是否為可寫候選、型別約束、預設型別）。
  - 輸出符合 **JSON Schema Draft 2020-12** 的 JSON 字串，頂層宣告 `"$schema": "https://json-schema.org/draft/2020-12/schema"`。
  - SC2 結構：根物件包含必填欄位 `id`（字串 `^[a-z][a-z0-9_]*$`）、`version`（整數 `>= 1`）、`title`（字串），可選欄位 `field`、`ctrl`、`parts`；`additionalProperties: false`。
  - SC2 field 結構：`in` 與 `out` 映射；每個接腳物件的合法屬性僅限 `type`（字串）、`required`（布林）、`label`（字串）、`unit`（字串），且 **required: [type]**；**禁止**其餘屬性（用 `additionalProperties: false` 把關），包含已移除的 `kind`、`level`、`reset`、`role`，以及 `min`、`max`、`default`、`message`。`propertyNames` 加 `not: { pattern: "^(cmd|man|ena|par|at|sts|error|count|pv|msg)_" }` 防止 ctrl 前綴混入。
  - SC3 ctrl 結構：使用 `patternProperties`，每個前綴一條正規式，各自完整定義 `required` 與 `properties`：
    - `^(cmd|man|ena|at|sts|count)_`：可選 `type`（`anyOf`：9 種標量 `enum: [bool, int16, uint16, int32, uint32, int64, uint64, float32, float64]`、`pattern: "^string:[1-9][0-9]*$"`、陣列 `pattern: "^(bool|int16|uint16|int32|uint32|int64|uint64|float32|float64)\\[([1-9]|[1-9][0-9]|[1-4][0-9][0-9]|500)\\]$"`，字串陣列禁止）、`label`、`unit`；不允許 `min`、`max`、`default`、`message`（以 `additionalProperties: false` 把關）。
    - `^par_`：**required: [type, min, max, default]**（與規格 §4.2、§5 一致；缺任一即失敗）；可選 `unit`（string）、`label`。（`min ≤ default ≤ max` 是 U12，不在 Schema 層）
    - `^error_`：**required: [message]**；可選 `label`、`unit`；`type` 固定 bool（明寫時必須是 `bool`），不允許 `min`/`max`/`default`；只有 `^error_` 允許 `message`，其餘前綴禁止。
    - `^(pv|msg)_`：**required: [type]**；可選 `label`、`unit`；其中 `^msg_` 的 `type` 限 `pattern: "^string:[1-9][0-9]*$"`（Schema 直接拒非字串，錯誤碼 SC3）。
    - 不符任何前綴的名稱由 `unevaluatedProperties: false` 攔截，錯誤訊息指向前綴表。
  - `type` 詞彙在 Schema 層完整覆蓋 `DataType::parse`：`anyOf` 含 9 種標量 `enum`（`bool`、`int16`、`uint16`、`int32`、`uint32`、`int64`、`uint64`、`float32`、`float64`）、`pattern: "^string:[1-9][0-9]*$"`、陣列 `pattern: "^(bool|int16|uint16|int32|uint32|int64|uint64|float32|float64)\\[([1-9]|[1-9][0-9]|[1-4][0-9][0-9]|500)\\]$"`（1～500，字串陣列不在此列故被拒）；無殘差，一致性測試涵蓋陣列案例。
  - SC2 parts 結構：映射鍵為子單元名，值為單元類型字串。
- `tests/schema_pins.rs`:
  - 比對 `include_str!("../../standards/schema/unit-type.schema.json")` 與 `generate_unit_type_schema()`。
  - 附帶 `#[ignore] fn print_schema()` 供維護者直接更新該檔案。

- [ ] **Step 1: 實作 `schema_gen.rs`，透過 `Prefix::ALL` 遍歷前綴屬性，產生符合 JSON Schema Draft 2020-12 的 JSON 字串**
- [ ] **Step 2: 產生 `standards/schema/unit-type.schema.json` 並寫入磁碟（先有檔案，Step 3 的比對才有意義）**
- [ ] **Step 3: 撰寫 `tests/schema_pins.rs`，比對磁碟檔案與產生器輸出**
- [ ] **Step 4: 執行 `cargo test -p machine-model --test schema_pins --features schema` 確認通過**
- [ ] **Step 5: Checkpoint: `cargo fmt --all --check` 與 clippy 檢查**

---

### Task 3: Schema 驗證 API 與一致性測試

**Files:**
- Modify: `machine-model/src/lib.rs`（`strip_bom` 改為 `pub(crate) fn` 供 `unit_type::validate_unit_type` 複用）
- Modify: `machine-model/src/unit_type.rs`
- Create: `machine-model/tests/schema_validate.rs`

**Feature Gate：**
- `tests/schema_validate.rs` 頂層加：`#![cfg(feature = "schema")]`

**Interfaces:**
- `unit_type::validate_unit_type(text: &str) -> Vec<Diagnostic>`（`#[cfg(feature = "schema")]`）：
  - **步驟一**：先以 `strip_bom` 去除 BOM，再呼叫 `parse_unit_type`（或等價的以 `StrictMap` 反序列化）做重複鍵檢查；若此步驟回 `Err`，直接回傳那些 `Diagnostic`，不進行 Schema 驗證（`StrictMap` 的重複鍵報錯不能靠 YAML→JSON 路徑補抓）。
  - **步驟二**：`serde_yaml_ng` 解析為 `serde_json::Value` 並轉成 JSON。
  - **步驟三**：呼叫 `schema_gen::generate_unit_type_schema()` 在記憶體產生 Schema，以 `jsonschema::validator_for` 驗證（**不讀磁碟**；磁碟上的 `unit-type.schema.json` 僅供 `schema_pins.rs` 的快照比對）。
  - **錯誤訊息**：`ValidationError` 轉譯為繁體中文；`instance_path` 格式沿用 `parse_unit_type` 的慣例：`types[<id>].ctrl[<pin>]`、`types[<id>].field.in[<pin>]`；根據欄位路徑標註 `SC2` 或 `SC3`。
- `unit_type::validate_unit_type_schema(text: &str) -> Vec<Diagnostic>`（`#[cfg(feature = "schema")]`）：只執行「YAML→JSON→Schema」（步驟二、三），**不先跑** `parse_unit_type`。僅供一致性測試使用，讓 Schema 能獨立判定；`validate_unit_type` = 步驟一 + 本函式。
- `tests/schema_validate.rs`:
  - 遍歷 `standards/unit-types/*.yaml` 全部標準類型，驗證每個皆通過且無錯誤（目前為 `cylinder_double.yaml`、`station_load.yaml`、`machine_ctrl.yaml`，未來新增時自動涵蓋）。
  - **一致性測試**：拿 `unit_type.rs` 內建測試裡的各類非法案例（SC2 未知欄位、SC3 前綴違規、`error_` 缺少 `message`、`par_` 缺少 `type`/`min`/`max`/`default`、陣列越界如 `int16[501]`/`int16[0]`/字串陣列等），驗證 `parse_unit_type`（程式碼版）與 `validate_unit_type_schema`（Schema 版，不經 parse）兩者**皆判定為失敗且皆帶有 SC2 或 SC3**。Schema 已完整覆蓋陣列 1～500 與 `string:N`，無殘差豁免。

- [ ] **Step 1: 在 `unit_type.rs` 撰寫 `validate_unit_type` 函式與錯誤訊息轉譯邏輯**
- [ ] **Step 2: 撰寫 `tests/schema_validate.rs` 驗證標準類型與負向錯誤案例**
- [ ] **Step 3: 執行 `cargo test -p machine-model --features schema --test schema_validate` 確認通過**
- [ ] **Step 4: 執行 `cargo test --workspace --features schema` 確保現有測試全數相容**

---

### 第二階段：`machinemodel` CLI（規格 §9.5、§12 第 3 項）

---

### Task 4: CLI Crate 骨架與退出碼規範

**Files:**
- Modify: 根目錄 `Cargo.toml`（加入 `members = ["machine-model", "machinemodel"]`）
- Create: `machinemodel/Cargo.toml`
- Create: `machinemodel/src/main.rs`
- Create: `machinemodel/tests/cli_exit_codes.rs`

**Interfaces:**
- 根目錄 `Cargo.toml`（workspace dependencies）：
  - 新增 `clap = { version = "4", features = ["derive"] }`、`serde_json = "1.0"`、`toml = "0.8"` 至 `[workspace.dependencies]`，統一版本管理。
- `machinemodel/Cargo.toml`：
  - 依賴（全部從 workspace 繼承）：`clap`（v4 derive）、`machine-model = { path = "../machine-model", features = ["schema"] }`、`serde`、`serde_json`、`serde_yaml_ng`、`toml`。
- `ExitCode`:
  - `0`: 成功（通過，無 Error 級別診斷）。
  - `1`: 規則或模型錯誤（存在 Error 診斷、或安全護欄被違反）。
  - `2`: 用法錯誤或 I/O 錯誤（引數錯誤、找不到檔案、無法讀取等）。
- `main.rs`:
  - 使用 `clap` 定義頂層 CLI 與四個子命令骨架：`check`、`dump`、`check-types`、`write-report`。

- [ ] **Step 1: 設定 workspace 與建立 `machinemodel` crate**
- [ ] **Step 2: 撰寫 `machinemodel/tests/cli_exit_codes.rs` 驗證未知參數、找不到檔案時回傳退出碼 2**
- [ ] **Step 3: 實作 `main.rs` 的錯誤處理與 exit code 返回邏輯**
- [ ] **Step 4: 執行 `cargo test -p machinemodel` 確認通過**

---

### Task 5: `check` 子命令（模型與安全規則檢查）

**Files:**
- Create: `machinemodel/src/check.rs`
- Modify: `machinemodel/src/main.rs`
- Create: `machinemodel/tests/cli_check.rs`

**Interfaces:**
- `machinemodel check <machine.yaml>`:
  - 類型目錄：內嵌標準類型，加上 `machine.yaml` 所在資料夾下的 `types/*.yaml`（自動讀取，無旗標；同 id 覆蓋並產生提示）。`types/` 不存在視為沒有本地類型；存在但讀檔失敗→退出 2。
  - 選項 `--report`：列印推導報告（每個 Tag 的推導 `role`、`write_candidate` 及其推導理由）。
  - 選項 `--coverage`：列印 HMI 覆蓋率缺口報告（未綁定的選擇性 ctrl 腳）。
- 執行邏輯：
  - 讀取機台設定檔（讀不到→stderr 繁中訊息並退出 2；解析/驗證的 Error 診斷→退出 1）。
  - 建構 `UnitCatalog`：以標準內嵌類型為基礎，呼叫 `UnitCatalog::with_local` 疊加機台資料夾的 `types/*.yaml`（見上方「類型目錄」）。
  - 呼叫 `machine_model::load_checked`（傳入已建構的 catalog）。
  - 若有 Error 診斷，向 stderr 輸出繁中錯誤訊息並退出 1。
  - 若有 Warning/Info 診斷，向 stderr 輸出提示訊息。
  - 若指定 `--report` 或 `--coverage`，向 stdout 輸出格式化表格或報告。
  - 無 Error 時退出 0。

- [ ] **Step 1: 撰寫 `cli_check.rs` 測試合法案例（回 0）、錯誤案例（回 1）、帶報告輸出案例**
- [ ] **Step 2: 實作 `check.rs` 處理引數與整合 `load_checked`**
- [ ] **Step 3: 執行 `cargo test -p machinemodel --test cli_check` 確認通過**

---

### Task 6: `dump` 與 `check-types` 子命令

**Files:**
- Modify: `machine-model/src/resolve.rs`（`ResolvedModel` 等加 `Serialize`）
- Modify: `machine-model/src/schema.rs`（`Variable`、`Scan`、`Role`、`Polarity`、`Scalar` 加 `Serialize`）
- Modify: `machine-model/src/strict.rs`（`StrictMap<V>` 手寫 `Serialize` 委派、`ExtValue` 加 `Serialize`）
- Modify: `machine-model/src/naming.rs`（`Prefix` 序列化為字串：先加 `as_str()`＋`From` 再 `serde(into)` 或手寫 `Serialize`）
- Modify: `machine-model/src/family.rs`（`Family` 加 `Serialize`）
- Create: `machinemodel/src/dump.rs`
- Create: `machinemodel/src/check_types.rs`
- Modify: `machinemodel/src/main.rs`
- Create: `machinemodel/tests/cli_dump_and_types.rs`

**Interfaces:**
- **`#[derive(Serialize)]` 傳遞閉包**（只加 `Serialize`，不改 `Deserialize`）：
  - `machine-model/src/resolve.rs`：`ResolvedModel`、`ResolvedPlc`、`ResolvedUnit`、`ResolvedTag`、`TagBinding`、`ResolvedPin`、`Fingerprint`、`RoleSource`
  - `machine-model/src/schema.rs`：`Variable`、`Scan`、`Role`、`Polarity`、`Scalar`
  - `machine-model/src/strict.rs`：`StrictMap<V>`（`impl<V: Serialize> Serialize for StrictMap<V>`，委派給內部 `BTreeMap`）、`ExtValue`
  - `machine-model/src/naming.rs`：`Prefix`（序列化為字串，以 `PrefixInfo::name` 為值，加 `#[serde(into = "&'static str")]` 或手寫 `impl Serialize`）
  - `machine-model/src/family.rs`：`Family`
  - 以上型別必須全數加上 `Serialize` 後，`cargo build -p machine-model --features schema` 才能通過；Step 1 完成後立即執行確認。
- `machinemodel dump <machine.yaml>`：
  - 選項 `--format <yaml|json>`（或 `-f`，預設為 `yaml`）；序列化欄位名稱維持 snake_case（serde 預設）。
  - 類型目錄同 Task 5 自動讀取（無旗標）。
  - 執行 `load_checked`，通過後將 `ResolvedModel` 序列化並輸出至 stdout。
- `machinemodel check-types [<DIR>]`：
  - **無參數**：對內嵌標準類型執行完整驗證（等價於 `UnitCatalog::from_sources(standard::standard_sources())` 後對每個類型跑 `validate_unit_type`）。
  - **有 `<DIR>` 參數**：讀取 `<DIR>/*.yaml`（讀檔失敗→退出 2），以 `standard_catalog.with_local(locals)` 疊加（本地同 id 覆蓋標準），再對 **全部合併後** 的類型執行 U1（名稱唯一）、U12（`par_` 範圍）、U15（拼寫提示）、U18（`parts` 存在且無環）、U19（`error_` bool）等驗證（U2 需 `units[]` 實例，不在 `check-types` 範圍；U17 須在 `<DIR>` 原始檔上檢查重複 id，因合併後 id 必然唯一；與標準同 id 只產生覆蓋提示），以及對每個類型跑 `validate_unit_type`（SC2/SC3 雙重驗證）。不支援「只驗 `<DIR>` 不帶標準庫」，避免 U5/U18 跨庫引用無法檢查。
  - 輸出每個類型的驗證結果；有錯誤退出 1，全部有效退出 0。

- [ ] **Step 1: 按 Interfaces 傳遞閉包在 `resolve.rs`、`schema.rs`、`strict.rs`、`naming.rs`、`family.rs`、`unit_type.rs` 加上 `Serialize` 衍生，隨即執行 `cargo build -p machine-model --features schema` 確認**
- [ ] **Step 2: 撰寫 `cli_dump_and_types.rs` 測試 `dump`（YAML 與 JSON 格式）及 `check-types`**
- [ ] **Step 3: 實作 `dump.rs` 與 `check_types.rs`**
- [ ] **Step 4: 執行 `cargo test -p machinemodel --test cli_dump_and_types` 確認通過**

---

### Task 7: `write-report` 子命令與 §9.3 安全護欄

**Files:**
- Create: `machinemodel/src/write_report.rs`
- Modify: `machinemodel/src/main.rs`
- Create: `machinemodel/tests/cli_write_report.rs`

**Interfaces:**
- `machinemodel write-report <machine.yaml> --service <service.toml>`:
  - 解析 `service.toml` 的 `[[tag_override]]` 子集（`id: String`, `writable: bool`），struct **不得**加 `deny_unknown_fields`（忽略 IP/Port 等通訊設定）；重複 `id` 視為護欄違規（退出 1）；`service.toml` 讀不到或 TOML 語法錯→退出 2。
  - 類型目錄同 Task 5 自動讀取（無旗標），再執行 `load_checked` 取得 `ResolvedModel`。
  - **驗證 §9.3 的 4 大安全護欄**：
    1. **唯讀類別不可覆寫為可寫**：若被覆寫的 Tag 之 `ResolvedTag::class_read_only == true`（如 X 點，由家族位址表判定），且覆寫設為 `writable = true`，判定違規。
    2. **安全監控點不可覆寫為可寫**：若被覆寫的 Tag 之 `ResolvedTag::safety_monitor == true`，且覆寫設為 `writable = true`，判定違規。
    3. **覆寫不可引用不存在的 Tag**：若 override 的 `id` 在 `ResolvedModel::tags` 中找不到，判定違規。
    4. **計算最終可寫清單與來源標記**：
       - 最終可寫預設 = 模型的 `write_candidate`。
       - 合法覆寫套用後，可寫性 = 覆寫的 `writable`。
       - 來源標明為 `model` 或 `override`。
       - 列出覆寫導致狀態改變的差異表（diff）。
  - 若違反任何護欄，印出清晰繁中違規錯誤並退出 1。
  - 若無違規，向 stdout 輸出完整審查報表並退出 0。

- [ ] **Step 1: 撰寫 `cli_write_report.rs` 測試護欄 1～3 的拒絕機制以及合法覆寫的報告輸出**
- [ ] **Step 2: 實作 `write_report.rs` 讀取 TOML 與執行護欄驗證邏輯**
- [ ] **Step 3: 執行 `cargo test -p machinemodel --test cli_write_report` 確認通過**
- [ ] **Step 4: 執行全 workspace 整合測試與檢查**

---

## 最終全域驗證協議

完成全部 7 個 Task 後執行：

```bash
# 1. 全工作區測試（含 schema feature）
cargo test --workspace --all-features

# 2. Clippy 靜態檢查（零警告）
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. 程式碼排版
cargo fmt --all --check

# 4. 函式庫依賴白名單檢驗
cargo test -p machine-model --test dependency_rules

# 5. 二進位執行檔冒煙測試
cargo run -p machinemodel -- check machine-model/tests/fixtures/valid_units.yaml
cargo run -p machinemodel -- dump machine-model/tests/fixtures/valid_units.yaml -f json | grep -q '"machine_id": "demo"'
cargo run -p machinemodel -- check-types

# 6. 禁止詞彙與已廢除規則檢查（只查欄位定義與規則編號字串，文件註解中提及廢棄欄位不算）
! grep -rnE '^\s+(pub\s+)?(writable|write_permission)\s*:' machine-model/src && ! grep -rnE '"(R3|R4|R20|R23)"' machine-model/src machinemodel/src && echo "OK: 函式庫核心無廢棄欄位"
# 註：只檢查 machine-model/src 的欄位定義（行首）；測試字串與 machinemodel 的 service.toml 的 writable 不在此列
```
