# Plan 4: ST 旁檔、驗證器與 `export-st`（Inovance H5U／Easy LiteST）

> **規格依據**：`2026-10-16-unit-tag-model-design.md` §2、§4.3、§7、§8.3（U20～U25）、§9.1、§9.5、§11 #6／#19～#22、§12 第 7 項。
> **前置工作**：Plan 1～3 已完成（類型目錄、推導、`schema` feature、`machinemodel` 命令列）。
> **取代**：`STConvert-design.md`（範本 CRUD、多廠牌方言、互鎖生成皆不採用；Unit 類型檔＋`.st` 旁檔承擔範本角色）。
> **語法依據**：`docs/H5U and Easy Series … Programming Guide-EN.pdf`（V4.12.0.0）§8.1 LiteST（p.261–275）、§7.3 變數（p.123–134）、FB 章節（p.218–222）。

## 目標

1. 標準與機台本地 Unit 類型可帶 `<id>.st` 旁檔，內嵌並一併釘選。
2. 輕量驗證器（U21～U25），只用 `std`，不是完整解析器。
3. `machinemodel export-st`：依規格 §7 降階，輸出 AutoShop 可用的資料（純本體 `.st`、表格、`symbols.csv`）。
4. **不做**：互鎖／護欄生成、`writable`／`safety_monitor` 相關輸出、其他廠牌方言、範本 CRUD。

## 關鍵事實（手冊）與設計後果

- LiteST 只有程式本體；FB 介面、區域變數、結構、全域變數都在 IDE 表格宣告，位址在變數表的「位址」欄（`D100`、`M0`、`X0`…），沒有 `AT %…` 語法。→ `.st` 旁檔是**本專案原始檔格式**，匯出時拆成「純本體」＋「表格」。
- 型別只有 `BOOL`、`INT`、`DINT`、`REAL`、`STRING`、`BYTE`、`IP`。→ `uint16`／`uint32`／`int64`／`uint64`／`float64` 無對應，U25 於匯出時報錯。
- `INOUT` 為複製進出語意；FB 巢狀 ≤ 8、單一 LiteST ≤ 1000 行、FB/FC 內部定義 ≤ 100 K。
- 支援 FB 呼叫 `inst(in := x, out => y);`，輸出實參不得為運算式。
- `X`／`Y` 為八進位（`X0`～`X1777`），與規格 §3 一致。

## 待使用者確認（阻塞項）

規格 §11 #21：AutoShop 的**匯入匯出**。請在 AutoShop 建最小專案（含一個結構、一個含 `IN`／`OUT`／`VAR` 的 FB、幾個綁定位址的全域變數、一個 FB 實例），匯出全域變數表、FB、結構，放進 `docs/`，並確認匯出檔能否原樣匯入回去。

- 取得前：Task 1～3 照常進行；Task 4 只做與格式無關的 `ExportPlan`；Task 5 的表格寫出器不定稿。
- 若 CSV 無法匯入：退路是輸出逐行對照表（Markdown／TSV）供手動貼上，`ExportPlan` 不受影響。
- 另需確認實際使用的 AutoShop 與韌體版本（手冊為 V4.12.0.0；`REGION`、FB 呼叫實參為運算式等功能有版本門檻）。

## 設計決策

- **驗證器放 `machine-model`**（旁檔隨類型目錄載入與內嵌）。
- **產生器放新 crate `st-export`**，只依賴 `machine-model` 與 `plc-common`，不引入新第三方依賴。命令列的 `export-st` 是薄殼。
- 產生器不做 I/O：輸出 `Vec<(路徑, 內容)>`，由命令列寫檔，方便黃金檔測試。
- **`ExportPlan`（與格式無關）**：降階結果的資料結構——結構清單、全域變數列（名稱、型別、位址、retain）、各 FB 介面列、FB 實例列、MAIN 本體的語句。寫出器（CSV／對照表）只讀 `ExportPlan`。
- FB 本體原封不動輸出（去除旁檔的宣告區），不做佔位符替換。
- `ctrl` 腳方向依前綴表（§5）：`in` 類（`cmd`／`man`／`ena`／`par`）在呼叫前由 Tag 寫入 struct，`out` 類（`at`／`sts`／`error`／`count`／`pv`／`msg`）在呼叫後寫回 Tag。純接線腳（`cmd`、`ena`）不綁 Tag，只存在於 struct。
- 全域變數的 `retain` 對應變數表的「掉電保持」欄；位址一律經 `plc-common` 的位址類別資料，不在產生器內自己拼。
- **方言集中在 `Dialect` trait**（借鑑 `STConvert-design.md`，只做 Inovance）：識別字轉換、型別對照、位址字串、FB 呼叫語法、註解／`REGION` 標記。產生器只呼叫 trait；新增廠牌 = 新增實作。現只有 `InovanceLiteSt`。
- **`symbols.csv`**：`Symbol,Type,Address,TagId`，工程師可由 IDE 變數名查回 Tag id。
- **檔頭來源註解**：每個輸出檔標明來源指紋（§9.2）、類型 `version` 清單與驗證狀態（通過實機前標「未經實機驗證」）。
- 診斷訊息用繁體中文並帶規則編號。

## 檔案結構

| 動作 | 路徑 | 說明 |
|---|---|---|
| 新增 | `standards/unit-types/<id>.st` | 三個既有標準類型各一份，**由人確認內容**，AI 不自創 |
| 新增 | `machine-model/src/st_check.rs` | 掃描器與 U21～U25 |
| 修改 | `machine-model/src/unit_type.rs`、`catalog.rs` | 旁檔載入（標準內嵌、本地 `types/<id>.st`） |
| 修改 | `machine-model/src/standard.rs` | `STANDARD` 增加 `.st` 的 `include_str!` |
| 修改 | `machine-model/tests/standard_pins.rs`、`standards/unit-types.pins` | 雜湊涵蓋 yaml＋st；三個類型加入 `.st` 後必須調高 `version` |
| 新增 | `st-export/`（`src/plan.rs`、`src/dialect.rs`、`src/writers.rs`、`src/lib.rs`） | 產生器 crate |
| 修改 | `Cargo.toml`（workspace members）、`machinemodel/` | 加 `export-st` 子命令 |
| 新增 | `st-export/tests/golden/` | 黃金檔 |


> **Task 1～3 實作狀態（2026-10-18）**：已完成並通過 `cargo test --workspace`（227 項）、clippy `-D warnings`、`cargo fmt --check`。
> 偏離計畫之處：
> - 三個標準類型的 `.st` **尚未建立**（AGENTS.md：標準類型內容須由人確認）。`STANDARD_ST` 目前為空，基礎設施與測試用夾具齊備；補上後須調高該類型 `version` 並更新 pins。因此「三個標準類型的 `.st` 必須通過」尚待人提供內容。
> - 旁檔雜湊在無旁檔時與舊算法相同（規格 §9.1 已同步），既有 pins 未變。
> - 新增 SC5（旁檔無對應類型檔／重複），已寫入規格 §8.2。
> - U23 實作為「已知非 `LiteST` 關鍵字／運算子／型別」的拒絕清單，不是完整白名單。

## Task 1：旁檔載入與釘選

- [x] `UnitCatalog` 載入類型時帶出 `Option<String>` 的 ST 內容；本地從 `types/<id>.st` 讀。
- [x] `standard_sources()` 擴充為含 `.st`；`Self::from_sources` 同步。
- [x] `pins` 雜湊改為 `content_hash(yaml + "\0" + st)`；`print_pins` 與 `every_standard_type_is_pinned_and_unchanged` 同步。
- [x] 測試：只改 `.st` 不調高 `version` → 釘選測試失敗；本地 `types/x.st` 無對應 `x.yaml` → 診斷（編號先補規格再用）。

## Task 2：掃描器與 U21、U22、U24

- [x] `st_check::scan`：去掉 `(* *)`（可巢狀）、`//` 註解與字串字面值；關鍵字不分大小寫。
- [x] 解析宣告區：`VAR_INPUT`／`VAR_OUTPUT`／`VAR_IN_OUT`／`VAR` 的 `name : type;`。
- [x] U24：成對關鍵字用堆疊判定，錯誤訊息指出行號。
- [x] U21：恰有一個 `FUNCTION_BLOCK`，名稱等於類型 id。
- [x] U22：`VAR_INPUT`／`VAR_OUTPUT` 與 `field.in`／`field.out` 比對名稱、方向、型別（標量詞彙→LiteST 型別）；`VAR_IN_OUT` 恰有 `ctrl : ST_<id>` 與各 part；本體引用的 `ctrl.<名>` 必須存在於 `ctrl` 腳。
- [x] 先寫失敗測試：FB 名稱不符、少接腳、方向錯、型別不符、`END_IF` 缺漏、註解中的假關鍵字、`ctrl.不存在`、`required: false` 的接腳仍須宣告。

## Task 3：U23、U25 與 `check` 整合

- [x] U23：語句白名單（見規格）；`ELSEIF` 報錯並提示 `ELSIF`；FB 呼叫輸出實參為運算式 → 錯；本體超過 1000 行 → 錯。不檢查函式名稱與運算元型別（交給 IDE）。
- [x] U25：標量詞彙→LiteST 型別對照表（程式內，與規格 §4.3 一致）；無對應者在匯出時列出 Tag／接腳並失敗。`check` 不報 U25。
- [x] `check`／`check-types` 對有旁檔的類型執行 U21～U24；三個標準類型的 `.st` 必須通過。

## Task 4：`ExportPlan` 與降階（與檔案格式無關）

- [ ] 依 §7 建立 `ExportPlan`：帶 `field` 腳的 Unit 提到頂層；station 以 `VAR_IN_OUT` 接子層 struct。
- [ ] 識別字轉換（`Dialect`）：Tag id 的 `.` 轉合法識別字，轉換結果衝突時報錯（不靜默改名）；符號必須能反查回唯一的 Tag id。
- [ ] ctrl 腳賦值順序（呼叫前寫入、呼叫後寫回）與 FB 呼叫語句。
- [ ] 資源檢查：FB 巢狀 ≤ 8、MAIN 本體 ≤ 1000 行，超過即失敗並指出數字。
- [ ] U20：被使用的類型缺 `.st` → 匯出失敗並列出。U25：型別無對應 → 失敗。
- [ ] `Dialect` 假實作測試：換方言不用改產生器。

## Task 5：寫出器與黃金檔（需 #21 的格式）

- [ ] 純本體：每個 FB 的 `<id>.body.st`（去除宣告區）與 `main.body.st`。
- [ ] 表格寫出器：結構、全域變數、FB 介面、FB 實例；格式依使用者提供的 AutoShop 匯出樣本定稿。樣本到手前只實作「逐行對照表」。
- [ ] `symbols.csv` 與檔頭來源註解。
- [ ] 黃金檔比對；樣本到手後，在本計畫補「輸出範例」。
- [ ] 確認 `tests/dependency_rules.rs` 仍通過；`st-export` 的依賴樹只含 `machine-model`、`plc-common`、`serde`、`serde_yaml_ng`。

## Task 6：`export-st` 命令列

- [ ] `export-st <machine.yaml> --out <資料夾>`；退出碼沿用 0／1／2。
- [ ] 輸出目錄已存在且非空 → 退回用法錯誤（2），不覆蓋。
- [ ] 測試：缺 `.st`、型別無對應（U25）、類型版本不符（U2）、成功輸出。

## 驗證（每個 Task 完成後）

    cargo test --workspace --all-features
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check

## 驗證紀錄

（待填：AutoShop 實機匯入、編譯結果與版本。）
