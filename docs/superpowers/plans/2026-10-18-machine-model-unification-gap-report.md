# machine-model 融合：階段 1 差距報告

日期：2026-10-18。工具：`tools/model-diff`（自成工作區，不在 MachineModel 的 members 內；原因：兩個同名 crate 會讓 `cargo tree -p machine-model` 含糊）。

重跑（`tools/model-diff` 已於階段 6 刪除，以下指令僅供回溯，需自行還原工具）：

    cargo run --manifest-path tools/model-diff/Cargo.toml -- tools/model-diff/fixtures/*
    python3 tools/model-diff/make-mutants.py   # 重新產生 fixtures/mut_*

比對的是兩邊的真實輸出（規則編號與是否為錯誤、每個 Tag 的 role／write_candidate／唯讀位址旗標／綁定數）。為了讓兩邊用同一份 Unit 類型定義，`base_same_types` 與所有 `mut_*` 都在 `types/` 放 MachineModel 的三個標準類型；因此 DataService 會固定多報 U16／U17 提示，工具已把這兩個提示列為已知雜訊並忽略。

結果：27 個 fixture（3 個既有檔、1 個無違規基準、23 個違規變體）；**33 項差異，另有 10 個違規變體兩邊完全一致**。`base_same_types`（無違規）兩邊一致，Tag 13 個，role 等欄位全相同。

## A. 同一缺陷、不同規則編號（共 7 組）

| 缺陷 | MachineModel | DataService |
|---|---|---|
| 位址無法解析／超出範圍 | R18 | R8 |
| Tag 型別語法錯誤（`string:0`） | R2 | SC3 |
| Tag id 重複 | R1 | R5 |
| `min > max` | R5 | R18 |
| `polarity` 用在非 bool | R24 | R19 |
| YAML 型別或列舉錯誤（`scan: warp`、`string:8[3]`、未知 family） | PARSE | SC1 |
| 未知欄位 | R22 | SC4 |

**危險點：R5、R18 兩邊同編號、意義互換**（MachineModel R5＝`min>max`、R18＝位址；DataService R5＝重複 id、R18＝`min>max`）。診斷編號會出現在 UI、文件與測試裡，融合時必須由規格決定唯一一套，且不能直接合併兩邊的字串。

## B. 規則寬嚴不同（行為差異）

| 樣本 | MachineModel | DataService | 判讀 |
|---|---|---|---|
| `default: 70000` 用在 int16 | R6 擋下 | **通過並推導成功** | DataService 較寬，預設值越界會進到執行期 |
| `plcs: []` | R1、R28 | R1、R28、**另加 R6** | DataService 多一條規則（MachineModel 是否有等價的「至少一個 PLC」要查規格） |
| 未知 Unit 類型 | 只報 U2 | U2＋**U5** | DataService 多報 U5；MachineModel 對同一缺陷只報 U2（兩邊對「類型不存在」的分類不同，U5 的定義待查規格） |
| `variables[]` 帶 `default`／`retain` | **接受** | **拒絕**（SC4 未知欄位） | 欄位集合不同：MachineModel 的 variables 比較寬 |

## C. 一致的部分（10 個違規變體兩邊給出相同編號集合）

`schema_version: 1`（R27）、`type_version` 不符（U2）、未知 bind 鍵（U3）、拼錯鄰近鍵（U3）、未知 Tag 參照（U4）、必綁腳缺漏（U6）、輸出腳重複綁定（U11）、父子成環（U5 U7 U8）、`man_` 腳綁唯讀輸入位址（U11 U13）、variables 帶位址（R25）。

## D. 標準 Unit 類型已分歧（資料層）

- 不加本地類型時：MachineModel 的 `valid_units`（`machine_ctrl` 用 version 2）在 DataService 報 U2、U6；DataService 自己的範例 `datasvc-demo/config/machine.yaml` 在 MachineModel 報 U2、U3。
- 類型內容差異：例如 `cylinder_double` 在 DataService 內嵌版有 `valve_ret: { required: false }`，MachineModel 版沒有；`machine_ctrl` 版本為 1 對 2。
- 結論：標準類型以 MachineModel 的 `standards/` 為準（規格：只有人確認過的類型才能進標準目錄）。DataService 內嵌的三份 YAML 字串不能直接丟，要以 MachineModel 版取代，並把 DataService 範例升版。

## E. 尚未涵蓋（本階段工具沒比對，不能從結果推論）

- `lowering`（降階）、`datasvc`（tag_override、DS1～DS3）、Builder API、HMI 聚合、拼寫提示文字、JSON Schema 產生：這些不在「兩邊都有」的範圍，屬計畫的「移植」項，不是差異。
- U14、U15、U18、U19、R9、R17、R21、R26 等共有規則：本階段沒有針對它們的違規樣本，等價性**仍未驗證**。
- `ResolvedTag` 與 `EffectiveTag` 的欄位逐項對應：目前只比對 role、write_candidate、唯讀位址旗標、綁定數，在 `base_same_types` 全部一致；其餘欄位（`readback`／`retain` 為 `Option<bool>` 對 `bool` 等）未比對。
- 規格版本是否同源、`DataType` 是否只差命名：未比對。

## F. 對階段 2 決策的輸入

1. 診斷編號必須統一成一套並改規格；R5、R18 的互換要明確處理（建議新編號，不沿用任一邊）。
2. 寬嚴差異（B 表）每一項都要決定採嚴的一邊或改規格；預設值越界建議採 MachineModel（較嚴）。
3. variables 的欄位集合以規格為準，需查規格 §3.1 後決定。
4. 標準類型以 MachineModel 為準，DataService 範例與測試升版。

## G. 階段 2 決策清單（建議，待使用者確認）

新發現：兩邊的 R 規則都宣稱「沿用 P1」，但 P1 規格（`AIAutomation/.claude/worktrees/agent-a0472972f7fa232b0/docs/superpowers/specs/2026-10-15-p1-machine-model-sync-core-design.md` §規則表）可以當裁判。對照結果：

| 規則 | P1 原文 | MachineModel | DataService |
|---|---|---|---|
| R1 | id 格式、全機唯一 | 符合 | 把「重複 id」改編為 R5 |
| R2 | type 屬於詞彙 | 符合 | 改報 SC3 |
| R5 | min 小於 max | 符合 | 被改成「重複 id」 |
| R18 | 位址語法與範圍符合家族表 | 符合 | 被改成 min>max |
| R19 | 位址類別允許該 type | 符合（見 `rules/addresses.rs`） | 被改成 polarity |
| R24 | polarity 只能用於 bool 輸入點 | 符合 | 被改成 R19 |
| R8 | 同 PLC 位址不得重疊 | 符合 | 被改成「位址語法」 |

結論：**DataService 的 R 編號相對 P1 漂移了，MachineModel 沒有。** 因此決策 1 不必發明新編號。

| # | 決策 | 建議 | 理由 |
|---|---|---|---|
| 1 | 診斷編號 | 以 MachineModel（＝P1 原編號）為準；DataService 一律改成同編號，SC1／SC4 併入 PARSE／R22，SC3 併入 R2 | P1 是裁判；融合後只需改 DataService 一邊 |
| 2a | 預設值越界（int16 的 70000） | 採 MachineModel（R6 擋下） | P1：default 須在範圍內且型別相容；DataService 漏掉 |
| 2b | `plcs: []` 額外的 R6 | **不納入（使用者已確認：不一定需要至少一個 PLC）** | P1／v2 規格也沒有這條 |
| 2c | 未知 Unit 類型多報 U5 | 不納入 | v2 規格：U2 管「類型存在」，U5 管「part 綁定的 Unit 存在且類型符合」；DataService 重複報錯 |
| 3 | variables 欄位集合 | 以 MachineModel 為準（＝P1 §3.5 去掉 `unit_id`）：`id`、`symbol`、`type`、`description`、`retain`、`default`、`ext`，加上只為報 R25 而保留的 `address`。DataService 缺 `retain`、`default`、`ext`，並多出規格沒有的 `label` | P1 §3.5 欄位清單即裁判；缺 `ext` 會丟掉 `ext.inovance.seq`（匯出排序用） |
| 4 | 標準 Unit 類型 | 以 MachineModel `standards/` 為準；DataService 內嵌 YAML 換成它；範例 `machine.yaml` 升版 | 規格：只有人確認過的類型才能進標準目錄 |

2b 已由使用者確認。決策 3 已由 P1 §3.5 裁定，不需再問。其餘建議都照做；階段 2 的決策到此完成。
