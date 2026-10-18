# Unit／Tag 模型規格（v0.3，待使用者審閱）

日期：2026-10-16
狀態：v0.3 定稿（使用者已確認；已依審查修訂：移除接腳 `kind`、`level`、`reset`、`role`；字串陣列明確禁止；機台本地類型自動讀取 `types/`）。相對 v0.2 的變更：模型層不再有 `writable`（改輸出 `write_candidate`，最終可寫由 PlcDataService 決定）；`units[]` 加 `type_version`；新增 §9 執行期輸出與下游接口；新增命令列工具範圍；規格適用於獨立的 MachineModel 專案。
性質：獨立規格，自成一體。P1／P2 規格（AIAutomation 專案）暫停，其中被取代的章節只作歷史參考。
互動示範：`scratch/unit-type-viewer.html`（AIAutomation 專案內，已發布為私人 Artifact），行為以本文為準，示範頁可能落後。

---

## 1. 目的與關係

### 1.1 目的

用兩種實例描述一台機台：

- **Tag**：葉節點，一個 PLC 位址上的點。
- **Unit**：節點，由一份 **Unit 類型檔**定義，內含若干接腳（pin）與子 Unit 槽位（part）。

警報、參數、模式、計數、訊息都不再是獨立實體，一律是 Tag，語意由接腳前綴決定。目標：

1. **讓工程師專心做機台模型化。** 使用者不手寫 `role`；模型由命名規則推導每個 Tag 的 `role` 與 `write_candidate`（是否為操作或參數的候選）。**模型層沒有 `writable`，也沒有 `write_permission`**：最終是否可寫由 PlcDataService 決定（§9.3），人員權限由 HMI 決定（§6.3）。
2. HMI 的操作、警報、參數頁由 Unit 類型導出，不另寫 `operations`。
3. 結構規則由 serde／JSON Schema 保證，關係規則用程式碼，並各自編號。

### 1.2 相對 P1／P2 規格的變更

| 原規格（AIAutomation 專案） | 處理 |
|---|---|
| P1 §3.3 `units[]`、§3.6 `devices[]` | 合併為本文 §3 的 `units[]` |
| P1 §3.7 `alarms[]`、§3.8 `parameters[]` | 取消；成為 Tag（`error_`、`par_` 前綴） |
| P1 §3.4 `tags[]` | 移除 `unit_id`、`writable`、`write_permission`；`role` 改為前綴推導（§6）；其餘欄位不變（§3.2） |
| P1 §3.10 裝置類型檔 | 改為本文 §4 的 Unit 類型檔 |
| P1 §4.1 R3、R4、R7、R20、R23、R25、R26、R27 | R3、R4、R20、R23 在 v2 不存在；R7、R25、R26、R27 修訂（見 §8.4） |
| P1 §4.2 D1、D3～D7；§4.3 T1～T7 | 由 §8 的 Schema 與 U 規則取代 |
| P2 規格 §3～§4（單元模板、D9／D10） | 取代：模板就是帶 `parts` 的 Unit 類型 |
| P1 原則 G4（`write_permission` 省略即唯讀） | 取代：模型層沒有可寫欄位；`write_candidate` 由前綴推導（§6.2） |

**V1 暫停**：本專案目前只支援 v2。P1 規格的其餘部分（I/O 表編解碼、漂移偵測、變更清單）不在本專案範圍，其現況與遷移見 §11。

### 1.3 `schema_version` 為 2

v2 移除 `devices[]`、`alarms[]`、`parameters[]`、Tag 的 `writable` 與 `write_permission`，並改變 `units[]` 語意，屬破壞性變更，故 `schema_version: 2`。本專案不讀 v1。

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

### 3.1 範例

```yaml
schema_version: 2
machine:
  id: demo
  plcs:
    - { id: plc1, family: inovance_easy }
tags:
  - { id: plc1.X_Cyl1_Ext_LS, address: X010, type: bool }
  - { id: plc1.M_Cyl1_ManExt, address: M100, type: bool }
units:
  - id: cyl1
    type: cylinder_double
    type_version: 1
    bind:
      lim_ext: plc1.X_Cyl1_Ext_LS   # field 腳：綁 Tag id
      lim_ret: null                 # 明確表示刻意不接
      valve_ext: plc1.Y_Cyl1_Ext
      man_ext: plc1.M_Cyl1_ManExt   # ctrl 腳：選擇性綁定
      error_ext: plc1.M_Cyl1_ErrExt
  - id: st1
    type: station_load
    type_version: 1
    bind:
      cyl: cyl1                     # part：綁 Unit id（不含 `.`）
  - id: machine
    type: machine_ctrl
    type_version: 1
    bind:
      stn: st1
```

- `machine`：`id`、`title`、`plcs[]`（`id`、`title`、`family`、`online`、`diff_scope`、`sources`），欄位與現有 `machine-model` 實作相同。`family` 為 PLC 驅動型號，共 55 種，見 §3.3。
- `variables[]`：保留，無位址的 PLC 變數；沿用現有欄位，移除 `unit_id`；不得有 `address`。
- `units[]`：`id`（唯一）、`type`（類型檔 id）、**`type_version`（必填整數，必須等於目錄中該類型的 `version`，不符即錯誤，見 U2）**、`bind`（map：接腳名或 part 名 → 值）。扁平清單，以 id 互相引用。
- `bind` 值只有兩種形式：字串 id，或 `null`。含 `.` 的字串是 Tag id（格式 `plc<N>.<Name>`，PLC 必須已宣告）；不含 `.` 的字串是 Unit id；`null` 是刻意不接。常數與運算式為保留，v1 不接受。
- 保留 id：`machine`（根）、`unassigned`（不得宣告）。

### 3.2 Tag 欄位

| 欄位 | 型別 | 說明 |
|---|---|---|
| `id` | 字串，必填 | `plc<N>.<Name>`，全機唯一（與 `variables` 共用命名空間） |
| `address` | 字串，必填 | 軟元件位址，依 PLC 家族解析 |
| `type` | 字串，必填 | 標量詞彙：`bool`、`int16`、`uint16`、`int32`、`uint32`、`int64`、`uint64`、`float32`、`float64`、`string:N`。除 `string:N` 外，標量可加 `[n]` 陣列（`n` 為 1～500）；**字串陣列（`string:N[n]`）禁止** |
| `symbol`、`label`、`description` | 字串 | 名稱、顯示名、說明 |
| `scan` | `fast`／`normal`／`slow` | 預設 `normal` |
| `scale`、`offset`、`deadband` | 數字 | `scale` 不得為 0 |
| `unit` | 字串 | 工程單位（如 `degC`） |
| `readback`、`retain`、`spare`、`safety_monitor` | 布林 | |
| `min`、`max`、`default` | 數字／標量 | 範圍與預設值 |
| `role` | 列舉 | 明確覆寫用，通常省略（§6.1） |
| `polarity` | `NO`／`NC` | 只能用於 bool 的唯讀輸入位址類別 |
| `ext` | map | 廠牌擴充 `ext.<廠牌>.<欄位>`，唯一的擴充通道 |

**沒有** `unit_id`、`writable`、`write_permission`；寫了即為未知欄位（`R22`）。

### 3.3 PLC 家族（驅動型號）與位址語法

`plcs[].family` 的值是 **PLC 驅動型號**，名稱與 DataService `mm-drivers` 的驅動型錄相同（例如 `inovance_easy_tcp`、`siemens_s7_1200`、`melsec_mc_binary`）。本專案收錄 **55 種 PLC 驅動**；非 PLC 的驅動（機器人、CNC、儀表、讀碼器、扭力槍、內建模擬器）不得作為 `family`。

- 舊名保留為別名：`inovance_easy` 等同 `inovance_easy_tcp`；`mitsubishi_fx3g` 與 `mitsubishi_fx3s` 分別等同 `melsec_fx_serial_tcp` 與 `melsec_fx_links_tcp`。後兩者的對應待實機確認。
- 位址語法分兩類（資料表 `machine-model/src/family.rs` 的 `Grammar`）：
  - **前綴表**（`classes`）：位址為「前綴＋編號」，可接 `.位元`；前綴必須在表中，編號與位元依表檢查。
  - **非空**（`opaque`）：位址有結構性語法（如 `DB1.DBW0`、`N7:0`、標籤名稱），只檢查非空，不檢查範圍、型別與重疊。共 15 種：三菱 CIP、西門子 S7-1200／1500／300／400 與 FetchWrite、歐姆龍 CIP、Allen-Bradley 五種、倍福 ADS、富士 SPH 與 CS。
- 來源與可信度：前綴取自驅動型錄的位址範例；進位與字／位元寬度依廠商慣例；**範圍一律為暫用上限 65535，未經實機或手冊驗證**，並在 `family.rs` 註解標明。例外：匯川 Easy 的 `X`／`Y`（八進位、唯讀）與三菱 FX 的 `X`／`Y` 沿用既有驗證。
- 連線協定即家族對應的驅動，`service.toml` 不另外指定協定；`online: true` 的家族皆可連線。
- 連線參數（IP、COM 埠、逾時）仍在 `service.toml`，不進模型（§9.3）。

---

## 4. Unit 類型檔

位置：標準資產 `standards/unit-types/<id>.yaml`（本專案）；機台本地類型放機台資料夾的 `types/<id>.yaml`，文法相同，同 id 時本地覆蓋標準並產生提示。

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

- `field.in` / `field.out`：實體接腳，名稱自由。型別限標量詞彙；`required: false` 表示可不接。不具 role、不是操作候選。名稱**不得**以 §5 任一前綴加 `_` 開頭，避免混淆。
- `ctrl`：控制介面，扁平 map，名稱必須是 `<前綴>_<功能>[_<細節>]`。前綴決定方向與語意（§5）。`ctrl.in` 與 `ctrl.out` 不分組，方向由前綴決定。
- `parts`：map，part 名 → Unit 類型 id。對應 IEC 的 `VAR_IN_OUT`。
- `type` 值的解析：標量詞彙 → Tag 槽位；否則視為 Unit 類型 id → Unit 槽位（僅 `parts` 內允許）。
- 接腳可選屬性：`type`（有預設者可省略）、`label`、`unit`、`min`、`max`、`default`、`message`。各前綴實際允許的屬性由 §4.2 的 Schema 把關：`min`、`max`、`default` 只用於 `par_`；`message` 只用於 `error_`（且必填）；`unit`、`label` 可用於任何 ctrl 腳；`field` 腳只允許 `type`、`required`、`label`、`unit`。`kind`、`level`、`reset`、`role` 是舊版留下的屬性，本版已移除，出現即為未知欄位。
- 同名後綴形成「功能」（上例 `ext`）：`man_ext`＝按鈕、`ena_ext`＝允許條件、`at_ext`＝狀態燈、`error_ext`＝警報、`par_ext_*`＝參數頁。HMI 由此導出操作，不另寫 `operations`。

### 4.2 類型檔 JSON Schema（規範性）

Schema 由命名規則表（§5）產生，不手寫。檔案：`standards/schema/unit-type.schema.json`（產生物，提交進 repo 並由測試確認與產生器一致）。要點：

- 頂層 `additionalProperties: false`；必填 `id`、`version`、`title`。
- `field.in.*`／`field.out.*` 的 `propertyNames` 用 `not: { pattern: "^(cmd|man|ena|par|at|sts|error|count|pv|msg)_" }`。
- `ctrl` 的 `patternProperties`：每個前綴一條，各自指定必填屬性與預設（例：`^par_` 要求 `type`、`min`、`max`、`default`；`^error_` 要求 `message`；`^pv_`、`^msg_` 要求 `type`）。不符任何前綴的名稱因 `additionalProperties: false` 而失敗，訊息指向前綴表。

---

## 5. 命名規則（前綴表）

標準資產 `standards/naming/prefixes.yaml`（或由程式內的前綴表產生，擇一為唯一事實來源），封閉、有版本。新增前綴屬規格變更。

| 前綴 | 方向 | 推測 role | 操作候選 (write_candidate) | 綁定 | 必要屬性 |
|---|---|---|---|---|---|
| `cmd` | in | command | 否 | 純接線，不需 Tag | — |
| `man` | in | command | **是** | 選擇性 | — |
| `ena` | in | command | 否 | 純接線 | — |
| `par` | in | setpoint | **是** | 選擇性 | `type`、`min`、`max`、`default` |
| `at` | out | status | 否 | 選擇性 | — |
| `sts` | out | status | 否 | 選擇性 | — |
| `error` | out | alarm | 否 | **必綁** | `message` |
| `count` | out | counter | 否 | 選擇性 | 預設型別 int32 |
| `pv` | out | process_value | 否 | 選擇性 | `type` |
| `msg` | out | message | 否 | 選擇性 | `type`（`string:N`） |

未寫 `type` 時的預設型別：`count` 為 `int32`，`cmd`、`man`、`ena`、`at`、`sts`、`error` 為 `bool`；`par`、`pv`、`msg` 必須寫 `type`。

模式：`man_mode`（int16，HMI 寫入，操作候選）與 `at_mode`（FB 回報實際模式，非候選）。FB 不得寫入 `man_*`／`par_*`，故 `ctrl` 中沒有 `inout`；`inout` 只出現在 `parts`。

欄位接腳前綴自由，僅作建議。

---

## 6. 推測規則

### 6.1 role

優先序：Tag 上明確寫的 `role`（§3.2）＞ 名稱（前綴）規則 ＞ 結構推測（未被任何前綴涵蓋者，目前僅 `field` 腳：無 role）。「明確 role」只指 Tag 層級；接腳上的 `role` 已在 v0.3 移除（§4.1），不參與優先序。檢查工具輸出每個被綁 Tag 的推測 role 與理由。

### 6.2 write_candidate

`write_candidate` 是模型輸出的**事實**，不是使用者欄位，也不是最終可寫：

- Tag 被 `man_*`、`par_*`（含 `man_mode`）接腳綁定 → `true`；
- 被其他接腳綁定，或沒有被任何 Unit 綁定 → `false`。

使用者不填；Tag 上沒有對應欄位。模型層另以 U13 擋掉明顯錯誤的綁定（把操作接腳綁到唯讀輸入位址）。

### 6.3 可寫與權限的責任分工

| 層 | 負責什麼 | 不負責什麼 |
|---|---|---|
| MachineModel | 輸出 `write_candidate`、`role`、`safety_monitor`、位址類別是否唯讀 | 最終可寫、人員權限 |
| PlcDataService | **最終 `writable`**：預設採 `write_candidate`，可針對單一 Tag 覆寫（§9.3），並以護欄保護硬體 | 人員權限、Unit 結構 |
| HMI | 人員權限（OP／Engineer／Vendor 與 PIN）：依操作類別（`man`／`par` 等前綴類別）與頁面統一管控 | Tag 的硬體可寫性 |

模型輸出的是前綴類別（例如這是 `man` 類或 `par` 類），HMI 自己決定該類需要哪個權限碼，不回頭污染 Tag 定義。

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
| 檔案結構、未知欄位（含禁止 `writable`、`write_permission`、`unit_id`） | 被綁的 Tag／Unit 是否存在 |
| 前綴是否合法、各前綴必要屬性 | 被綁 Tag 型別與位址方向是否相符 |
| 型別詞彙、列舉值、欄位格式 | 單親、無環 |
| field 腳不得用控制前綴 | `min ≤ default ≤ max`、名稱衝突 |

### 8.2 SC 規則（Schema）

- SC1：`machine.yaml` v2 結構（`schema_version: 2`、`tags[]`、`units[]`、`bind` 值為字串或 `null`）。由 serde 型別（`deny_unknown_fields`）承擔，不另維護 JSON Schema，避免與 Rust 型別漂移；類型檔（SC2／SC3）使用 JSON Schema（`standards/schema/unit-type.schema.json`，由前綴表產生）。
- SC2：類型檔結構（§4.2）。
- SC3：前綴合法與各前綴必要屬性。
- SC4：任何位置的未知欄位為錯誤（延續 P1 R22，含 `writable`、`write_permission`、`unit_id`、`devices`、`alarms`、`parameters`）。

### 8.3 U 規則（程式碼）

| 編號 | 規則 |
|---|---|
| U1 | `units[].id` 唯一；類型內接腳／part 名稱在 `field`、`ctrl`、`parts` 全體唯一 |
| U2 | `units[].type` 存在於類型目錄，且 `type_version` 等於該類型的 `version` |
| U3 | `bind` 的鍵必須是該類型宣告的接腳或 part |
| U4 | `field` 腳綁定的 Tag 存在；`in` 腳須為輸入位址類（唯讀輸入，如 X）、`out` 腳須為實體輸出位址類（如 Y）；型別完全相符 |
| U5 | part 綁定的 Unit 存在，且其類型符合 part 宣告 |
| U6 | 必綁項目已綁：`field` 腳（除 `required: false`，明確 `null` 視為刻意不接）、`error_` 腳 |
| U7 | 每個 Unit 至多一個父層（Tag 多重綁定由 U11 規範） |
| U8 | 無環；所有 Unit 皆可由 `machine` 到達 |
| U9 | `machine` 存在且為根 |
| U10 | `ctrl` 腳綁定的 Tag 存在，型別與接腳 `type` 相容 |
| U11 | 多重綁定限制：`field.out` 腳綁定的 Tag 必須全機唯一（杜絕致動器輸出衝突）；其餘接腳（`field.in`、`par_*`、`man_*`、`at_*` 等）允許多個單元接腳綁定同一個 Tag（如共用感測輸入、共用參數、共用同動操作），但同一 Tag 綁定的所有接腳前綴與語意必須相容（禁止混用 field/ctrl、禁止混用可寫候選與唯讀、型別必須一致） |
| U12 | `par_`：`min ≤ default ≤ max` |
| U13 | `man_*`、`par_*` 接腳綁定的 Tag 不得位於唯讀位址類別（如 `X`），也不得 `safety_monitor: true`（取代 P1 的 R20、R23） |
| U14 | Tag 上明確寫的 `role` 與前綴推測不同：提示（以明確值為準） |
| U15 | 接腳功能名拼寫相近：僅出現一次的功能名與出現 ≥2 次、編輯距離 1 的功能名 → 提示 |
| U16 | 未綁的選擇性 ctrl 腳：不報錯，列入「HMI 覆蓋率」報告 |
| U17 | 類型 id 在同一來源內唯一：同一資料夾內重複為錯誤；本地類型與標準同 id 為覆蓋，產生提示（§9.1），不是錯誤 |
| U18 | `parts` 引用的類型存在於目錄，且類型之間的 `parts` 關係無環（含自我引用） |
| U19 | `error_` 接腳的型別必須是 bool |

型別字串須為標量詞彙；`string:N` 的長度上限等 Schema 樣式表達不了的部分，由 `DataType::parse` 檢查並以 `SC3` 回報。

**載入安全規則（不需類型目錄）**：`R1`、`R2`、`R5`、`R6`、`R8`、`R9`、`R17`～`R19`、`R21`、`R24` 沿用 P1；`R3`、`R4`、`R20`、`R23` 在 v2 不存在；`R25` 只剩「variables 不得有 address」；`R26` 檢查單元實例 id（格式、唯一、不得為 `unassigned`；`machine` 允許）；`R27` 要求 `schema_version: 2`；新增 `R28`：`units[].type` 與 `bind` 鍵必須是合法名稱，`type_version` 必須是正整數，`bind` 值必須是 `null`、合法 Unit id，或 PLC 已宣告的合法 Tag id。

診斷訊息用繁體中文並含規則編號。

### 8.4 P1 載入規則修訂

- R3、R4（writable↔write_permission）、R20（唯讀類別不得有 write_permission）、R23（safety_monitor 不得可寫）：v2 廢除；其意圖由 U13 與 PlcDataService 的護欄（§9.3）承接。
- R7（`unit_id`）、R26：改為 U7、U9。
- R25（引用存在、警報觸發為 bool）：改為 U4、U5、U10。
- R27：`schema_version` 必須為 2。
- R22（未知欄位）：由 SC4 涵蓋。

### 8.5 實作位置與依賴

- 本專案是一個 Rust 函式庫（`machine-model`）加一個命令列 crate（§9.5）。函式庫本體包含：v2 型別、載入安全規則、前綴表、類型目錄、推導、U 規則、家族與位址表。執行期（PlcDataService）也依賴函式庫本體，所以本體的依賴只有 `serde` 與 `serde_yaml_ng`。
- JSON Schema 驗證（類型檔的 SC2／SC3）用 `jsonschema` 0.58.x，放在 `schema` feature（預設關閉），**必須** `default-features = false`（預設功能含 `resolve-http`，會拉進 `reqwest`）。已用 `cargo tree` 確認關閉預設功能後不含 `reqwest`、`hyper`、`tokio`、`axum`。專案測試需檢查函式庫本體的依賴樹不含 `tokio`、`axum`、`reqwest`、`hyper`。
- 命令列 crate 可依賴 `clap` 與 `schema` feature；函式庫本體不得依賴它們。

---

## 9. 執行期與下游接口

### 9.1 類型目錄載入（採選項 B，加版本固定）

- 標準 Unit 類型檔（本專案 `standards/unit-types/*.yaml`）在編譯時以 `include_str!` 內嵌至函式庫，現場部署只需單一 `machine.yaml`（v2），不需額外的 types 目錄，也不產生 `machine.resolved.yaml`。
- 機台本地特化類型：機台資料夾（`machine.yaml` 所在的資料夾）下的 `types/*.yaml`，**自動讀取**，不需任何旗標；同 id 時覆蓋標準並產生提示。命令列工具與 PlcDataService 採同一規則。
- **版本固定**：`units[].type_version`（§3.1）必須等於載入時目錄中該類型的 `version`，不符就是載入錯誤（U2），**不得靜默採用**。理由：設計期用新版類型檔通過檢查、現場 binary 內嵌舊版時，兩邊對同一 Tag 推導出不同的 `write_candidate`，屬安全相關差異。
- 類型檔內容改變而不調高 `version` 屬違規；專案測試以內容雜湊釘住每個標準類型的 `version`。

### 9.2 解析結果（函式庫輸出）

函式庫對一份 `machine.yaml` 與類型目錄，輸出下列資料結構（記憶體內；命令列工具可傾印為 YAML／JSON 供審查，見 §9.5）：

- **有效 Tag 表**：每個 Tag 的 `id`、`address`、`type`、`scan`、`scale`、`offset`、`deadband`、`unit`、`readback`、`retain`、`safety_monitor`、`role`（推導後）、**`write_candidate`**、來自 `par_` 接腳的 `min`／`max`／`default`／`unit`、所屬 Unit 路徑與接腳名清單（`bindings`）、位址類別是否唯讀。
- **Unit 樹**：每個 Unit 的 `id`、類型、`type_version`、父層、`bind`。
- **清單**：操作（`man_`）、警報（`error_`：message）、參數（`par_`）、數值（`pv_`、`count_`）、訊息（`msg_`），各項帶接腳前綴類別與 `label`。
- **指紋**：來源檔雜湊與使用的類型檔版本。

### 9.3 PlcDataService

- **來源**：`model_source` 改為呼叫函式庫取得「有效 Tag 表」，轉成 `Point`（欄位多半一一對應；`min`／`max`／`default`／`unit` 來自 `par_` 接腳）。連線設定（協定、COM 埠、逾時）仍在 `service.toml`，不進模型。`cmd_`、`ena_` 是純接線，不出現在點表。
- **最終 `writable`**：由 PlcDataService 持有。預設等於模型給的 `write_candidate`；允許在自己的設定裡針對單一 Tag 覆寫：

  ```toml
  [[tag_override]]
  id = "plc1.Homing_Start_CMD"
  writable = true        # true 或 false
  ```

- **護欄**（違反即啟動失敗或拒絕該覆寫，並記錄）：
  1. 覆寫**不能**讓唯讀位址類別（`X` 等，由家族表判定）的 Tag 變可寫；
  2. `safety_monitor: true` 的 Tag 不能被覆寫為可寫；
  3. 覆寫引用不存在的 Tag → 啟動錯誤；
  4. 啟動時輸出最終可寫清單，並標明來源（`model` 或 `override`）；API 在 Tag 資訊中帶 `writable` 與來源。
- 設計期檢查（命令列工具）列出「最終可寫但不是模型候選」的 Tag，供人審查；取代原本的 `review.md` 簽核。
- 沒被任何 Unit 綁定的 Tag 預設不可寫（fail-closed）；需要寫入就綁到某個 `man_` 接腳，或使用覆寫。

### 9.4 HMI

- HMI **本階段不直接接模型**。HMI 目前使用虛擬點位，且貼標機黃金案例尚未重構成 v2，沒有真實模型可對。
- HMI 繼續只依賴 PlcDataService 的 API（`/api/v1/tags` 等）。建議之後由 PlcDataService 在 Tag 資訊中帶出 `role`、`label`、`unit`、`min`／`max`、前綴類別，HMI 即可在不依賴 MachineModel 的情況下顯示與分類。
- 黃金案例重構完成後，再評估：第一級——命令列工具檢查 HMI YAML 引用的 Tag 皆存在，且每個 `write_tag` 的最終 `writable` 為真；第二級——由 Unit 的操作／狀態／警報／參數產生起手式畫面；第三級——HMI 執行期直接渲染通用面板。
- 權限（`P01`～`P08` 與角色）完全屬 HMI 的 `permissions.yaml`；模型只提供前綴類別，不提供權限碼。

### 9.5 命令列工具

獨立 crate（`machinemodel`），函式庫本體不依賴它。子命令（本階段）：

- `check <machine.yaml>`：載入安全規則與 U 規則，輸出診斷；可選輸出推導報告（每個 Tag 的 `role`、`write_candidate` 與理由）、「HMI 覆蓋率」報告。
- `dump <machine.yaml>`：傾印解析結果（§9.2）為 YAML／JSON，供審查與 diff。
- `check-types [<資料夾>]`：檢查類型檔（含 JSON Schema，需 `schema` feature）與標準目錄。
- `write-report <machine.yaml> --service <service.toml>`：列出最終可寫清單與覆寫差異（§9.3）。
- 退出碼：0 通過、1 有錯誤、2 用法或檔案讀寫錯誤。
- HMI 引用檢查（§9.4 第一級）在需要時再加。

---

## 10. HMI 導出與覆蓋率

由類型與綁定導出（無需手寫）：

- 按鈕：`man_<f>`。
- 啟用條件：同功能 `ena_<f>`。
- 狀態燈：`at_<f>`、`sts_<f>`。
- 警報：`error_<f>`（message）。
- 參數頁：`par_*`（min／max／default／unit）。
- 數值與訊息：`pv_*`、`msg_*`、`count_*`。

「HMI 覆蓋率」報告：列出每個 Unit 實例有哪些 ctrl 腳未綁，供使用者判斷是否刻意省略（U16，不阻擋）。

---

## 11. 待決項目

| # | 項目 | 決策／建議 |
|---|---|---|
| 1 | 執行期與類型目錄載入策略 | **已決：選項 B ＋ `type_version` 固定**（§9.1） |
| 2 | 預設 label／警報訊息詞典 | 做，放命名規則資產 |
| 3 | ctrl 介面 Tag 位址來源 | 新機台由工具配置、既有機台由工程師提供，兩者並存 |
| 4 | 平面舊機台的 `derived` 虛擬輸出 | 延後 |
| 5 | 根 Unit `machine` 的最小框架（模式集合、`auto_running`） | 先給最小類型 `machine_ctrl`，完整框架留 P6 |
| 6 | FB 的 ST ↔ 類型檔一致性檢查 | 延到 P4 |
| 7 | Unit 陣列（`cyl[8]`） | 延後 |
| 8 | `bind` 的常數／運算式 | 保留，v2 不接受 |
| 9 | `Role` 無 `param` 值 | 沿用 `setpoint` |
| 10 | 既有 FB 參數命名遷移（`dExtTime` → `par_ext_time` 等） | **已決：全面重構**；貼標機黃金案例重構為 Unit 樹（在 AIAutomation 專案 `machines/`） |
| 11 | 示範頁中已知的資料缺口 | 屬範例資料問題，非規格問題 |
| 12 | 可寫與權限 | **已決**：模型層只輸出 `write_candidate`；最終 `writable` 在 PlcDataService（§9.3）；權限在 HMI（§6.3） |
| 13 | `machine.yaml` v2 的 JSON Schema（編輯器提示用） | 延後；需要時由 serde 型別產生，不手寫 |
| 14 | 標準類型檔與前綴表的存放 | 本專案 `standards/`；內嵌進函式庫（§9.1） |
| 15 | HMI 是否現在接模型 | **已決：本階段不接**（§9.4） |
| 16 | P1 同步核心（I/O 表匯入匯出、漂移比對、變更清單：`aia-core`、`aia-cli`）如何遷移到 v2 | 待決；V1 暫停期間它與 `datasvc-core` 仍使用 AIAutomation 內的 v1 `machine-model`，需等本專案可用後逐一遷移，另開計畫 |
| 17 | PlcDataService 覆寫設定放 `service.toml` 還是獨立檔 | 建議放 `service.toml` 的 `[[tag_override]]`，隨機台資料夾版本管理 |
| 18 | 命令列工具名稱與發行方式 | 暫名 `machinemodel`；先隨原始碼建置，之後再決定預建二進位 |

---

## 12. 實作切分

各自一份計畫，依序進行（本專案內的計畫放本專案 `plans/`）：

1. **MachineModel 函式庫本體**：v2 型別（Tag 無 `writable`／`write_permission`／`unit_id`；`units[]` 含 `type_version`）、載入安全規則（R1、R2、R5、R6、R8、R9、R17～R19、R21、R24～R28）、家族與位址表、前綴表、類型檔型別與目錄（含內嵌標準類型與 `type_version` 檢查）、U 規則、推導（`role`、`write_candidate`）、解析結果資料結構。
2. **`schema` feature**：JSON Schema 產生與類型檔驗證（SC2／SC3）。
3. **命令列 crate**：`check`、`dump`、`check-types`，其後 `write-report`。
4. **PlcDataService 整合**（AIAutomation 專案 `runtime/datasvc`）：`model_source` 改用函式庫；`[[tag_override]]` 與護欄；API 帶 `writable` 與來源。
5. **貼標機黃金案例重構**（AIAutomation 專案 `machines/`）。
6. **AIAutomation 遷移**：`aia-core`／`aia-cli` 改用 v2、移除內部 `model/machine-model`（待決 #16）。

不在範圍：ST 產生器（P4）、機台框架（P6）、未降階的 FB 型匯出、HMI 整合（§9.4 第二、三級）。
