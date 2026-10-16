# Unit／Tag 模型規格（v0.2，經 Grill-me 討論確認定案）

日期：2026-10-16
狀態：定案；已完成 Grill-me 設計決策對齊。
性質：獨立規格。取代 P1 規格與 P2 規格的部分章節（見 §1.2），其餘章節不動。
互動示範：`scratch/unit-type-viewer.html`（已發布為私人 Artifact），行為以本文為準，示範頁可能落後。

---

## 1. 目的與關係

### 1.1 目的

用兩種實例描述一台機台：

- **Tag**：葉節點，一個 PLC 位址上的點。
- **Unit**：節點，由一份 **Unit 類型檔**定義，內含若干接腳（pin）與子 Unit 槽位（part）。

警報、參數、模式、計數、訊息都不再是獨立實體，一律是 Tag，語意由接腳前綴決定。目標：

1. **徹底解耦權限**：使用者不手寫 `role` 與 `writable`，由命名規則自然推導硬體級 `writable`（防止誤寫輸入點）；`write_permission` 徹底自模型層移除，改由 HMI UI 端按操作類別與頁面統一管控。
2. HMI 的操作、警報、參數頁由 Unit 類型導出，不另寫 `operations`。
3. 結構規則由 serde／JSON Schema 保證，關係規則用程式碼，並各自編號。

### 1.2 取代範圍

| 原規格 | 處理 |
|---|---|
| P1 §3.3 `units[]`、§3.6 `devices[]` | 合併為本文 §3 的 `units[]` |
| P1 §3.7 `alarms[]`、§3.8 `parameters[]` | 取消；成為 Tag（`error_`、`par_` 前綴） |
| P1 §3.4 `tags[]` | 移除 `unit_id` 與 `write_permission`；`role`、`writable` 改為前綴推導（見 §6）；其餘欄位不變 |
| P1 §3.10 裝置類型檔 | 改為本文 §4 的 Unit 類型檔 |
| P1 §4.1 R3、R4、R7、R25、R26、R27 | 修訂（見 §8.4，v2 移除 R3／R4） |
| P1 §4.2 D1、D3～D7 | 由 §8 的 Schema 與 U 規則取代 |
| P1 §4.3 T1～T7 | 由 §8 取代 |
| P2 規格 §3～§4（單元模板、D9／D10） | 取代：模板就是帶 `parts` 的 Unit 類型 |
| P1 原則 G4（`write_permission` 省略即唯讀） | 取代：模型層廢除 `write_permission`，`writable` 由前綴推導（`man_`、`par_` 為可寫，`field.in`、`at_`、`error_` 等為不可寫，且輸入類位址嚴禁設為可寫） |

不動：P1 的 PLC 家族設定表、I/O 表編解碼、漂移偵測、變更清單、`PlcDataService`。
貼標機黃金案例（Inovance／FX3G）與 PLC 函式庫 FB 參數採全面重構（Station／Unit 樹狀架構與前綴命名，見 §11 #10）。

### 1.3 `schema_version` 升為 2

P1 原則 G5 規定只加欄位、不改既有欄位語意，R27 要求值為 1。本規格移除 `devices[]`、`alarms[]`、`parameters[]`、`write_permission` 並改變 `units[]` 語意，屬破壞性變更，故 `schema_version: 2`。

---

## 2. 概念

| 本文 | OPC UA | IEC 61131-3 | 說明 |
|---|---|---|---|
| Tag | Variable | VAR（有位址） | 葉，一個點 |
| Unit 類型 | ObjectType | FUNCTION_BLOCK 宣告 | 類型檔 |
| Unit 實例 | Object | FB 實例 | `units[]` 一項 |
| 接腳 | 屬性 | VAR_INPUT／VAR_OUTPUT | `field`、`ctrl` |
| part | 子物件 | VAR_IN_OUT（FB 型） | `parts` |

- 一台機台是一棵樹，根為保留 Unit 實例 `machine`。
- **歸屬由父層宣告**：父 Unit 以 `bind` 指向子 Unit 或 Tag。每個 Unit／Tag 至多一個父層，不得成環。
- 只有包含關係，模型內沒有共用或引用；降階時用 struct 傳遞只是匯出手法（§7）。
- FB 內部的 `local` 變數不在模型內。標準裝置 FB 的 ST 由工程師手寫，模型只檢查接腳與 ST 的 `VAR_INPUT`／`VAR_OUTPUT` 是否一致（檢查本身延到 P4，見 §11）。

---

## 3. `machine.yaml` v2

```yaml
schema_version: 2
machine:                           # 沿用 P1 §3.1、§3.2，不變（PLC 在 machine.plcs）
  id: demo
  plcs:
    - { id: plc1, family: inovance_easy }
tags:
  - { id: plc1.X_Cyl1_Ext_LS, address: X010, type: bool }
  - { id: plc1.M_Cyl1_ManExt, address: M100, type: bool }
units:
  - id: cyl1
    type: cylinder_double
    bind:
      lim_ext: plc1.X_Cyl1_Ext_LS   # field 腳：綁 Tag id
      lim_ret: null                 # 明確表示刻意不接
      valve_ext: plc1.Y_Cyl1_Ext
      man_ext: plc1.M_Cyl1_ManExt   # ctrl 腳：選擇性綁定
      error_ext: plc1.M_Cyl1_ErrExt
  - id: st1
    type: station_load
    bind:
      cyl: cyl1                     # part：綁 Unit id（不含 `.`）
  - id: machine
    type: machine_ctrl
    bind:
      stn: st1
```

- `variables[]`：保留，沿用 P1 §3.5，移除 `unit_id`；仍不得有 `address`（P1 R25 的這一句保留）。
- 辨識 `bind` 值：含 `.` 的字串是 Tag id（格式 `plc<N>.<Name>`，PLC 必須已宣告）；不含 `.` 的字串是 Unit id；`null` 是刻意不接。
- `tags[]`：沿用 P1 §3.4 欄位，移除 `unit_id` 與 `write_permission`（v2 若出現視為未知欄位 R22）。`role`、`writable` 可省略，由接腳前綴自動推導（§6）。
- `units[]`：`id`（唯一）、`type`（類型檔 id）、`bind`（map：接腳名或 part 名 → 值）。扁平清單，以 id 互相引用。
- `bind` 值只有兩種形式：字串 id，或 `null`。常數與運算式為保留，v1 不接受。
- 保留 id：`machine`（根）。P1 R26 的其他保留 id 沿用。

---

## 4. Unit 類型檔

位置：標準資產 `design/standards/unit-types/<id>.yaml`；機台本地類型放 `machines/<機台>/types/<id>.yaml`，文法相同，同 id 時本地覆蓋標準並產生提示。

```yaml
id: cylinder_double
version: 1
title: 雙作用氣缸
field:
  in:
    lim_ext: { type: bool }
    lim_ret: { type: bool, required: false }
  out:
    valve_ext: { type: bool }
    valve_ret: { type: bool }
ctrl:
  cmd_ext:       { type: bool }
  cmd_ret:       { type: bool }
  ena_ext:       { type: bool }
  man_ext:       { type: bool, label: 伸出 }
  man_ret:       { type: bool, label: 縮回 }
  at_ext:        { type: bool }
  at_ret:        { type: bool }
  par_ext_time:  { type: int16, unit: ms, min: 0, max: 30000, default: 3000 }
  par_err_time:  { type: int16, unit: ms, min: 0, max: 30000, default: 5000 }
  error_ext:     { message: 伸出逾時 }
  error_ret:     { message: 縮回逾時 }
parts: {}
```

### 4.1 區塊

- `field.in` / `field.out`：實體接腳，名稱自由。型別限標量詞彙（沿用 P1 §3.4）；`required: false` 表示可不接。不具 role、永遠不可由 HMI 寫入。名稱**不得**以 §5 任一前綴加 `_` 開頭，避免混淆。
- `ctrl`：控制介面，扁平 map，名稱必須是 `<前綴>_<功能>[_<細節>]`。前綴決定方向與語意（§5）。`ctrl.in` 與 `ctrl.out` 不分組，方向由前綴決定。
- `parts`：map，part 名 → Unit 類型 id。對應 IEC 的 `VAR_IN_OUT`。
- `type` 值的解析：標量詞彙 → Tag 槽位；否則視為 Unit 類型 id → Unit 槽位（僅 `parts` 內允許）。
- 接腳可選屬性：`label`、`kind: momentary|toggle`、`role`、`unit`、`min`、`max`、`default`、`message`、`level`、`reset`、`type`（有預設者可省略）。
- 同名後綴形成「功能」（上例 `ext`）：`man_ext`＝按鈕、`ena_ext`＝允許條件、`at_ext`＝狀態燈、`error_ext`＝警報、`par_ext_*`＝參數頁。HMI 由此導出操作，不另寫 `operations`。

### 4.2 類型檔 JSON Schema（規範性）

Schema 由命名規則表（§5）產生，不手寫。檔案：`design/standards/schema/unit-type.schema.json`（產生物，提交進 repo 並由測試確認與產生器一致）。要點：

- 頂層 `additionalProperties: false`；必填 `id`、`version`、`title`。
- `field.in.*`／`field.out.*` 的 `propertyNames` 用 `not: { pattern: "^(cmd|man|ena|par|at|sts|error|count|pv|msg)_" }`。
- `ctrl` 的 `patternProperties`：每個前綴一條，各自指定必填屬性與預設（例：`^par_` 要求 `type`、`min`、`max`、`default`；`^error_` 要求 `message`；`^pv_`、`^msg_` 要求 `type`）。不符任何前綴的名稱因 `additionalProperties: false` 而失敗，訊息指向前綴表。
- 屬性列舉：`kind`、`level`、`reset` 用 `enum`。

---

## 5. 命名規則（前綴表）

標準資產 `design/standards/naming/prefixes.yaml`，封閉、有版本。新增前綴屬規格變更。

| 前綴 | 方向 | 推測 role | 可寫 (writable) | 綁定 | 必要屬性 |
|---|---|---|---|---|---|
| `cmd` | in | command | 否 | 純接線，不需 Tag | — |
| `man` | in | command | 是 | 選擇性 | — |
| `ena` | in | command | 否 | 純接線 | — |
| `par` | in | setpoint | 是 | 選擇性 | `type`、`min`、`max`、`default` |
| `at` | out | status | 否 | 選擇性 | — |
| `sts` | out | status | 否 | 選擇性 | — |
| `error` | out | alarm | 否 | **必綁** | `message`；預設 level `abnormal`、reset `manual` |
| `count` | out | counter | 否 | 選擇性 | 預設型別 int32 |
| `pv` | out | process_value | 否 | 選擇性 | `type` |
| `msg` | out | message | 否 | 選擇性 | `type`（`string:N`） |

未寫 `type` 時的預設型別：`count` 為 `int32`，`cmd`、`man`、`ena`、`at`、`sts`、`error` 為 `bool`；`par`、`pv`、`msg` 必須寫 `type`。

模式：`man_mode`（int16，HMI 寫入，可寫）與 `at_mode`（FB 回報實際模式，唯讀）。FB 不得寫入 `man_*`／`par_*`，故 `ctrl` 中沒有 `inout`；`inout` 只出現在 `parts`。

欄位接腳前綴自由，僅作建議。

---

## 6. 推測規則

### 6.1 role

優先序：明確 `role` ＞ 名稱（前綴）規則 ＞ 結構推測（未被任何前綴涵蓋者，目前僅 `field` 腳：無 role）。`aia check` 輸出每個被綁 Tag 的推測 role 與理由。

### 6.2 writable

使用者**不填** `writable`。依 §5 推導：`man_*`、`par_*`、`man_mode` 為可寫（`true`），其餘不可寫（`false`）。Tag 明確寫了 `writable` 時只允許更嚴（`false`）；對不可寫接腳或 `field.in` 腳寫 `writable: true` 為錯誤（U13）。

### 6.3 權限由 HMI 管控，模型層不包含 write_permission

底層模型與 `PlcDataService` 僅需保證硬體位址寫入安全（`writable: bool`）。操作員、工程師、原廠（OP／Engineer／Vendor）的人員權限認證與 PIN 驗證，統一由 HMI（`runtime/hmi/config/permissions.yaml` 與 `auth` 模組）在 UI 端依操作類別與功能卡片管控，不再污染底層 Tag 定義。Tag 若出現 `write_permission` 欄位視為未知欄位（R22）。

### 6.4 label 與 message

`label`（HMI 中文顯示）可選，預設由「已知功能名詞典」（`ext→伸出`、`ret→縮回`、`start→啟動`…，放在命名規則資產）產生。`error` 的 `message` 目前仍為必填。

---

## 7. 降階（匯出層）

IR 保留樹狀；匯出器預設輸出降階形式：

- 帶 `field` 腳的 Unit 提到頂層；父層（station）以 `VAR_IN_OUT stX : <struct>` 接收子層 `ctrl` 介面。理由：目前 Inovance 精簡版 ST 不能把 FB 當參數傳，但可宣告巢狀 FB 實例；直接巢狀會使接腳逐層穿透暴增。
- struct 由 `ctrl` 腳機械導出；`field` 腳在實例宣告／呼叫處綁定。
- 父層只可寫子 `ctrl` 的 `in` 腳、讀 `out` 腳。
- 未降階（FB 型）形式延後，不在本規格範圍。

---

## 8. 驗證

### 8.1 分工

| 由 JSON Schema / serde 承擔（SC 規則） | 由程式碼承擔（U 規則） |
|---|---|
| 檔案結構、未知欄位（含禁止 write_permission） | 被綁的 Tag／Unit 是否存在 |
| 前綴是否合法、各前綴必要屬性 | 被綁 Tag 型別與位址方向是否相符 |
| 型別詞彙、列舉值、欄位格式 | 單親、無環 |
| field 腳不得用控制前綴 | `min ≤ default ≤ max`、名稱衝突 |

### 8.2 SC 規則（Schema）

- SC1：`machine.yaml` v2 結構（`schema_version: 2`、`tags[]`、`units[]`、`bind` 值為字串或 `null`）。
  `machine.yaml` 的 SC1／SC4 由 `machine-model` 的 serde 型別（`deny_unknown_fields`）承擔，不另維護 JSON Schema，避免與 Rust 型別漂移；類型檔（SC2／SC3）使用 JSON Schema（`design/standards/schema/unit-type.schema.json`，由前綴表產生）。
- SC2：類型檔結構（§4.2）。
- SC3：前綴合法與各前綴必要屬性。
- SC4：任何位置的未知欄位為錯誤（延續 P1 R22，含 `write_permission`、`devices`、`alarms`、`parameters`）。

### 8.3 U 規則（程式碼，`aia-core`）

| 編號 | 規則 |
|---|---|
| U1 | `units[].id` 唯一；類型內接腳／part 名稱在 `field`、`ctrl`、`parts` 全體唯一 |
| U2 | `units[].type` 存在於類型目錄 |
| U3 | `bind` 的鍵必須是該類型宣告的接腳或 part |
| U4 | `field` 腳綁定的 Tag 存在；`in` 腳須為輸入位址類、`out` 腳須為輸出位址類；型別相容 |
| U5 | part 綁定的 Unit 存在，且其類型符合 part 宣告 |
| U6 | 必綁項目已綁：`field` 腳（除 `required: false`，明確 `null` 視為刻意不接）、`error_` 腳 |
| U7 | 每個 Unit／Tag 至多一個父層 |
| U8 | 無環；所有 Unit 皆可由 `machine` 到達 |
| U9 | `machine` 存在且為根 |
| U10 | `ctrl` 腳綁定的 Tag 存在，型別與接腳 `type` 相容 |
| U11 | 同一 Tag 至多綁一個接腳 |
| U12 | `par_`：`min ≤ default ≤ max` |
| U13 | 明確 `writable: true` 落在不可寫接腳或 `field.in` 腳上為錯誤 |
| U14 | 明確 `role` 與前綴推測不同：提示（以明確值為準） |
| U15 | 接腳功能名拼寫相近：僅出現一次的功能名與出現 ≥2 次、編輯距離 1 的功能名 → 提示 |
| U16 | 未綁的選擇性 ctrl 腳：不報錯，列入「HMI 覆蓋率」報告 |
| U17 | 類型 id 在類型目錄內唯一 |
| U18 | `parts` 引用的類型存在於目錄，且類型之間的 `parts` 關係無環（含自我引用） |
| U19 | `error_` 接腳的型別必須是 bool |

型別字串須為標量詞彙；`string:N` 的長度上限等 Schema 樣式表達不了的部分，由 `DataType::parse` 檢查並以 `SC3` 回報。

**載入安全規則（`model/machine-model`，不需類型目錄）**：`R1`、`R2`、`R5`、`R6`、`R8`、`R9`、`R17`～`R21`、`R23`、`R24` 沿用 P1；`R3`／`R4` 在 v2 不存在（`write_permission` 為未知欄位 R22）；`R20` 沿用（輸入類位址嚴禁設為可寫）；`R25` 只剩「variables 不得有 address」；`R26` 檢查單元實例 id（格式、唯一、不得為 `unassigned`；`machine` 允許）；`R27` 要求 `schema_version: 2`；新增 `R28`：`units[].type` 與 `bind` 鍵必須是合法名稱，`bind` 值必須是 `null`、合法 Unit id，或 PLC 已宣告的合法 Tag id。U1～U19 在 `design/aia-core`。

診斷訊息用繁體中文並含規則編號，沿用 `machine-model` 的診斷格式。

### 8.4 P1 載入規則修訂

- R3、R4（writable↔write_permission）：v2 廢除。模型層移除 `write_permission`，`writable` 改由 §6 推導。
- R7（`unit_id`）、R26：改為 U7、U9。
- R25（引用存在、警報觸發為 bool）：改為 U4、U5、U10。
- R27：`schema_version` 必須為 2。
- R22（未知欄位）：由 SC4 涵蓋（含 `write_permission`、`devices`、`alarms`、`parameters`）。

### 8.5 實作位置與依賴

- Schema 驗證與 U 規則放 `design/aia-core`；**不放進 `model/machine-model`**，因為它是執行期共用 crate。
- Rust 函式庫：`jsonschema` 0.58.x，**必須** `default-features = false`（預設功能含 `resolve-http`，會拉進 `reqwest`，違反 AGENTS.md）。已用 `cargo tree` 確認關閉預設功能後不含 `reqwest`、`hyper`、`tokio`、`axum`。`dependency_rules` 測試需補檢查這四者不出現，防止日後開回預設功能。
- `machine-model` 只增加 v2 的 serde 結構（`units[].type/bind`），不加 Schema 驗證。

---

## 9. 執行期與類型目錄載入（採選項 B）

執行期（PlcDataService、HMI）採用「選項 B：二進位打包內嵌標準類型目錄」：

- 編譯時將標準 Unit 類型檔（`design/standards/unit-types/*.yaml`）以二進位靜態資產（如 `include_str!`）內嵌至執行期載入器中。
- 現場機台部署僅需提供單一 `machine.yaml`（v2），執行期啟動時直接以內嵌標準類型進行解析、接腳綁定與 `writable` 推導，現場不需額外維護散落的 types 目錄，亦不需產生與維護 `machine.resolved.yaml` 中介檔。
- 機台本地特化類型（若有）仍支援從本機機台目錄動態疊加覆蓋。
- `aia` 工具仍可提供 `aia inspect` 或 `aia export` 命令供工程師查看展開後的完整樹狀結構與綁定報表。

---

## 10. HMI 導出與覆蓋率

由類型與綁定導出（無需手寫）：

- 按鈕：`man_<f>`（`kind` 預設 momentary）。
- 啟用條件：同功能 `ena_<f>`。
- 狀態燈：`at_<f>`、`sts_<f>`。
- 警報：`error_<f>`（level／reset／message）。
- 參數頁：`par_*`（min／max／default／unit）。
- 數值與訊息：`pv_*`、`msg_*`、`count_*`。

「HMI 覆蓋率」報告：列出每個 Unit 實例有哪些 ctrl 腳未綁，供使用者判斷是否刻意省略（U16，不阻擋）。

---

## 11. 待決項目（更新為拍板結果）

| # | 項目 | 決策結果 |
|---|---|---|
| 1 | 執行期與類型目錄載入策略 | **已決：採選項 B**。標準類型目錄內嵌於 binary，現場直讀 `machine.yaml` v2，免除 resolved 中介檔 |
| 2 | 預設 label／警報訊息詞典 | 做，放命名規則資產 |
| 3 | ctrl 介面 Tag 位址來源 | 新機台由工具配置、既有機台由工程師提供，兩者並存 |
| 4 | 平面舊機台的 `derived` 虛擬輸出 | 延後 |
| 5 | 根 Unit `machine` 的最小框架（模式集合、`auto_running`） | 先給最小類型 `machine_ctrl`，完整框架留 P6 |
| 6 | FB 的 ST ↔ 類型檔一致性檢查 | 延到 P4 |
| 7 | Unit 陣列（`cyl[8]`） | 延後 |
| 8 | `bind` 的常數／運算式 | 保留，v1 不接受 |
| 9 | `Role` 無 `param` 值 | 沿用 `setpoint` |
| 10 | 既有 FB 參數命名遷移（`dExtTime` → `par_ext_time` 等） | **已決：採全面重構**。函式庫 FB 原始碼全面改用前綴命名，並將貼標機黃金案例完整重構為 Unit 樹 |
| 11 | 示範頁中已知的資料缺口（`par_ret_time` 缺 max／default 等） | 屬範例資料問題，非規格問題 |
| 12 | `write_permission` 格式與符號值（P01 vs operator） | **已決：模型層徹底廢除 `write_permission`**。權限判定完全移交 HMI UI 端按操作類別控管 |
| 13 | `machine.yaml` v2 的 JSON Schema（編輯器提示用） | 延後；需要時由 serde 型別產生，不手寫 |

---

## 12. 對現有程式的影響與實作切分

依 AGENTS.md，一次只改一個子目錄，各自一份計畫：

1. `model/machine-model`：v2 serde 結構（不含 `write_permission`）、載入規則 R7/R20/R25/R26/R27/R28 修訂（廢除 R3/R4）；不加 Schema 驗證。先改本規格 → 再改 P1 規格的「被取代」註記。
2. `design`：內嵌標準類型目錄、Schema 產生與驗證、U1～U19、前綴推導、降階；`device_rules.rs`（D3～D7）大部分重寫；`standards.rs` 的裝置類型載入改為 Unit 類型檔。
3. `runtime/datasvc`、`runtime/hmi`：直讀 `machine.yaml` v2，由 HMI UI 模組依操作類別實施身分權限驗證；`PlcDataService` 僅依 `writable: bool` 保護硬體寫入。
4. `machines/貼標機-inovance`、`machines/貼標機-fx3g`：重構為 Station／Unit 樹狀結構，PLC 函式庫 FB 參數全面改用前綴命名。
5. `integration`：補 `dependency_rules`（§8.5）與黃金案例重構測試。

不在範圍：ST 產生器（P4）、機台框架（P6）、未降階的 FB 型匯出。
