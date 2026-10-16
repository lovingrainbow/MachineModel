# MachineModel 函式庫（二）：前綴表、類型目錄、U 規則與推導 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Plan 1 的基礎上完成函式庫本體：命名前綴表、Unit 類型檔（解析與 SC2／SC3）、類型目錄（含內嵌標準類型與版本釘選）、U 規則、`role`／`write_candidate` 推導，以及解析結果資料結構與單一入口 `load_checked`。

**Architecture:** 新增模組 `naming`（前綴表）、`unit_type`（類型檔型別與解析）、`catalog`＋`type_rules`（目錄與類型層規則 U1／U12／U15／U17／U18／U19）、`standard`＋`hash`（內嵌標準類型與內容雜湊）、`analyze`（實例層規則 U2～U11、U13、U14）、`resolve`（推導與輸出結構）。依賴仍只有 `serde`、`serde_yaml_ng`。類型檔的 SC2／SC3 在本計畫以程式碼檢查；JSON Schema 版（`schema` feature）是下一份計畫，須與這裡的規則一致。

**Tech Stack:** 同 Plan 1（Rust 2024、`rust-version = "1.85"`、`serde`、`serde_yaml_ng`）。

**Spec:** `2026-10-16-unit-tag-model-design.md`（v0.3）：§4 類型檔、§5 前綴表、§6 推測、§8.2～§8.3 規則、§9.1～§9.2 目錄與解析結果、§12 第 1 項後半。

**Prerequisite:** Plan 1（`plans/2026-10-16-plan-1-foundation.md`）全部完成並通過驗證。本計畫假設其產物原樣存在（`schema.rs`、`rules/`、`tests/common/mod.rs` 的 `HEAD`／`doc`／`tags`／`codes`／`tag_codes`）。

> **勾選說明**：已完成的步驟已勾選，依據為程式與測試（`cargo test -p machine-model` 全綠）。8 個「確認測試失敗」（紅燈）步驟未勾選：紅燈的執行結果沒有留在 git 歷史中，無法回溯驗證。

## Global Constraints

- 本專案根目錄：`/mnt/c/Users/wydct/orca/projects/MachineModel`；指令都在此執行。
- 函式庫本體依賴只有 `serde`、`serde_yaml_ng`（`tests/dependency_rules.rs` 把關，不得放寬）。
- 不使用 let-chains（`rust-version = "1.85"`）。`Option::is_none_or`、`is_some_and` 可用。
- 診斷訊息用繁體中文並帶規則編號；規則編號只能是：`R*`（Plan 1）、`SC2`、`SC3`、`U1`～`U19`、`PARSE`。
- Tag 沒有 `writable`／`write_permission`／`unit_id`；本計畫**不得**把它們加回來。最終 `writable` 屬 PlcDataService，不在本專案。
- 嚴重度：`U14`、`U15` 是 `Warning`；本地類型覆蓋標準類型的提示是 `Info`（編號 `U17`）；其餘規則都是 `Error`。只有 `Error` 會使載入失敗。
- 前綴表的程式內常數（`naming.rs`）是唯一事實來源，不另建 `prefixes.yaml`（規格 §5 允許擇一）。
- 每個任務結束時必須通過：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all --check`。clippy 為 pedantic：遇到誤報時用**區域性** `#[allow(clippy::…)]` 並在上一行註解原因；不要放寬 workspace 設定（Task 1 唯一的例外：`too_many_lines`）。
- **不要 `git add`／`git commit`**：提交由控制端在審查後、使用者同意時進行。

## Review Focus

規格沒逐項點名、但最可能咬到使用者的情境，各自在對應任務有測試：

1. 本地類型覆蓋標準類型但 `version` 不同 → 舊的 `type_version` 實例必須得 `U2`，不得靜默採用（Task 3）。
2. 類型檔含 `writable`／`write_permission` 之類的 v1 屬性、或 CRLF／BOM → `SC2` 未知欄位；CRLF／BOM 照常解析且不影響內容雜湊（Task 2、4）。
3. 標準類型檔內容被改但 `version` 沒調 → 測試失敗（版本釘選，Task 4）。
4. 單元綁自己、互相綁成環、被兩個父層綁、`machine` 被當成子層 → `U8`／`U7`／`U9`，且演算法不得因環而無窮迴圈（Task 5）。
5. 兩個 `field.out` 接腳搶同一個 Tag、同一 Tag 混綁寫入候選與唯讀狀態、`man_`／`par_` 綁到唯讀輸入位址或 `safety_monitor` Tag → `U11`／`U13`；允許多個單元共用 `field.in`、`par_`、`man_`（Task 6）。
6. 沒被任何接腳綁定的 Tag：`write_candidate` 必須是 `false`（fail-closed），且不得 panic（Task 7）。

## File Structure

```
Cargo.toml                         Task 1：workspace lints 加 too_many_lines
AGENTS.md                          Task 4：加標準類型規則
standards/unit-types/*.yaml        Task 4：cylinder_double、station_load、machine_ctrl
standards/unit-types.pins          Task 4：每個標準類型的 id／version／內容雜湊
machine-model/src/
  diag.rs                          Task 1：加 warning／info 建構子
  naming.rs                        Task 1：Prefix、PrefixInfo、split、預設 label 詞典
  unit_type.rs                     Task 2：UnitType、CtrlPin、FieldPin、PinRef、parse_unit_type（SC2／SC3）
  catalog.rs                       Task 3：UnitCatalog（from_sources、with_local）
  type_rules.rs                    Task 3：U1、U12、U15、U17（在 catalog.rs）、U18、U19
  hash.rs                          Task 4：content_hash（FNV-1a 64，正規化換行）
  standard.rs                      Task 4：內嵌標準類型、UnitCatalog::standard()
  analyze/mod.rs                   Task 5：check／check_graph／check_pins
  analyze/graph.rs                 Task 5：U2、U3、U5、U7、U8、U9
  analyze/pins.rs                  Task 6：U4、U6、U10、U11、U13、U14
  resolve.rs                       Task 7：ResolvedModel 與推導
  lib.rs                           各任務加 `pub mod …;`；Task 8 加 load_checked
machine-model/tests/
  common/mod.rs                    Task 5、6：加目錄與模型工具
  catalog.rs                       Task 3
  standard_pins.rs                 Task 4
  analyze_graph.rs                 Task 5
  analyze_pins.rs                  Task 6
  resolve.rs                       Task 7
  e2e.rs                           Task 8
  fixtures/valid_units.yaml        Task 7
```

---

### Task 1: 前綴表與診斷等級

**Files:**
- Modify: `Cargo.toml`（workspace lints）、`machine-model/src/diag.rs`、`machine-model/src/lib.rs`
- Create: `machine-model/src/naming.rs`

**Interfaces:**
- Produces:
  - `Diagnostic::warning(code, location, message) -> Diagnostic`、`Diagnostic::info(code, location, message) -> Diagnostic`（簽名同 `error`；`severity` 分別為 `Warning`／`Info`）
  - `naming::Prefix { Cmd, Man, Ena, Par, At, Sts, Error, Count, Pv, Msg }`（`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`）；`Prefix::ALL: [Prefix; 10]`；`Prefix::info(self) -> &'static PrefixInfo`
  - `naming::PrefixInfo { prefix, name: &'static str, direction: Direction, role: Role, write_candidate: bool, bind: BindPolicy, default_type: Option<&'static str> }`
  - `naming::{Direction { In, Out }, BindPolicy { Wiring, Optional, Required }}`
  - `naming::split(name: &str) -> Option<(Prefix, &str)>`：`"par_ext_time"` → `Some((Par, "ext_time"))`；前綴後的部分為空或沒有 `_` 回 `None`
  - `naming::has_prefix_underscore(name: &str) -> bool`：名稱是否以任一 `<前綴>_` 開頭（`"cmd_"` 也算）
  - `naming::function_of(rest: &str) -> &str`：`"ext_time"` → `"ext"`
  - `naming::default_label(function: &str) -> Option<&'static str>`

- [x] **Step 1: 放寬 `too_many_lines` 並加診斷建構子**

`Cargo.toml` 的 `[workspace.lints.clippy]` 區塊加一行：

```toml
too_many_lines = "allow"
```

`machine-model/src/diag.rs` 的 `impl Diagnostic` 內，`error` 之後加：

```rust
    pub fn warning(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            location: Some(location.into()),
        }
    }

    pub fn info(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Info,
            message: message.into(),
            location: Some(location.into()),
        }
    }
```

並在 `diag.rs` 的 `mod tests` 內加：

```rust
    #[test]
    fn warning_and_info_are_not_errors() {
        let w = Diagnostic::warning("U14", "tags[plc1.A]", "role 與前綴不同");
        let i = Diagnostic::info("U17", "types[x]", "本地覆蓋");
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(i.severity, Severity::Info);
        assert!(!w.is_error() && !i.is_error());
    }
```

- [x] **Step 2: 寫失敗的測試（`naming.rs` 只先寫測試，並在 `lib.rs` 加 `pub mod naming;`）**

`machine-model/src/lib.rs`：在 `pub mod family;` 之後加 `pub mod naming;`。

`machine-model/src/naming.rs` 先只放：

```rust
//! 命名規則（前綴表，規格 §5）。這張程式內的表是前綴的唯一事實來源。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Role;

    #[test]
    fn table_is_complete_and_indexed_by_enum_order() {
        assert_eq!(Prefix::ALL.len(), 10);
        for (i, p) in Prefix::ALL.iter().enumerate() {
            assert_eq!(*p as usize, i);
            assert_eq!(p.info().prefix, *p);
        }
    }

    #[test]
    fn spec_table_values() {
        let info = |p: Prefix| p.info();
        assert_eq!(info(Prefix::Man).role, Role::Command);
        assert_eq!(info(Prefix::Par).role, Role::Setpoint);
        assert_eq!(info(Prefix::Error).role, Role::Alarm);
        assert_eq!(info(Prefix::Count).role, Role::Counter);
        assert_eq!(info(Prefix::Pv).role, Role::ProcessValue);
        assert_eq!(info(Prefix::Msg).role, Role::Message);
        assert_eq!(info(Prefix::At).direction, Direction::Out);
        assert_eq!(info(Prefix::Cmd).direction, Direction::In);
        // 只有 man 與 par 是寫入候選
        let candidates: Vec<_> = Prefix::ALL
            .iter()
            .filter(|p| p.info().write_candidate)
            .map(|p| p.info().name)
            .collect();
        assert_eq!(candidates, ["man", "par"]);
        // 只有 error 必綁；cmd、ena 是純接線
        assert_eq!(info(Prefix::Error).bind, BindPolicy::Required);
        assert_eq!(info(Prefix::Cmd).bind, BindPolicy::Wiring);
        assert_eq!(info(Prefix::Ena).bind, BindPolicy::Wiring);
        assert_eq!(info(Prefix::Man).bind, BindPolicy::Optional);
        // 預設型別
        assert_eq!(info(Prefix::Count).default_type, Some("int32"));
        assert_eq!(info(Prefix::Man).default_type, Some("bool"));
        assert_eq!(info(Prefix::Error).default_type, Some("bool"));
        for p in [Prefix::Par, Prefix::Pv, Prefix::Msg] {
            assert_eq!(p.info().default_type, None, "{p:?}");
        }
    }

    #[test]
    fn split_recognises_prefix_underscore_rest() {
        assert_eq!(split("par_ext_time"), Some((Prefix::Par, "ext_time")));
        assert_eq!(split("man_mode"), Some((Prefix::Man, "mode")));
        assert_eq!(split("error_ext"), Some((Prefix::Error, "ext")));
        for bad in ["par", "par_", "parx_a", "lim_ext", "Par_a", "", "_par_a", "at"] {
            assert_eq!(split(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn field_names_must_not_look_like_control_pins() {
        for name in ["cmd_x", "man_", "error_a", "pv_1"] {
            assert!(has_prefix_underscore(name), "{name}");
        }
        for name in ["lim_ext", "valve_ext", "pvx", "cmd", "atx_a"] {
            assert!(!has_prefix_underscore(name), "{name}");
        }
    }

    #[test]
    fn function_and_label() {
        assert_eq!(function_of("ext_time"), "ext");
        assert_eq!(function_of("ext"), "ext");
        assert_eq!(default_label("ext"), Some("伸出"));
        assert_eq!(default_label("ret"), Some("縮回"));
        assert_eq!(default_label("zzz"), None);
    }
}
```

- [ ] **Step 3: 確認測試失敗**

Run: `cargo test -p machine-model naming`
Expected: FAIL（編譯錯誤：`Prefix` 等未定義）。

- [x] **Step 4: 寫實作（放在 `naming.rs` 的 `#[cfg(test)]` 之前）**

```rust
use crate::schema::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prefix {
    Cmd,
    Man,
    Ena,
    Par,
    At,
    Sts,
    Error,
    Count,
    Pv,
    Msg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// 綁定要求（規格 §5「綁定」欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindPolicy {
    /// 純接線，不需要 Tag（`cmd`、`ena`）。
    Wiring,
    /// 選擇性綁定；未綁列入 HMI 覆蓋率（U16）。
    Optional,
    /// 必綁（`error`，U6）。
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixInfo {
    pub prefix: Prefix,
    pub name: &'static str,
    pub direction: Direction,
    /// 推測的 role（規格 §6.1）。
    pub role: Role,
    /// 操作候選（規格 §6.2）。
    pub write_candidate: bool,
    pub bind: BindPolicy,
    /// 未寫 `type` 時的預設型別；`None` 表示必須明寫。
    pub default_type: Option<&'static str>,
}

const fn row(
    prefix: Prefix,
    name: &'static str,
    direction: Direction,
    role: Role,
    write_candidate: bool,
    bind: BindPolicy,
    default_type: Option<&'static str>,
) -> PrefixInfo {
    PrefixInfo {
        prefix,
        name,
        direction,
        role,
        write_candidate,
        bind,
        default_type,
    }
}

/// 順序必須與 [`Prefix`] 的宣告順序一致（`Prefix::info` 以列舉值當索引，由測試保證）。
static TABLE: [PrefixInfo; 10] = [
    row(Prefix::Cmd, "cmd", Direction::In, Role::Command, false, BindPolicy::Wiring, Some("bool")),
    row(Prefix::Man, "man", Direction::In, Role::Command, true, BindPolicy::Optional, Some("bool")),
    row(Prefix::Ena, "ena", Direction::In, Role::Command, false, BindPolicy::Wiring, Some("bool")),
    row(Prefix::Par, "par", Direction::In, Role::Setpoint, true, BindPolicy::Optional, None),
    row(Prefix::At, "at", Direction::Out, Role::Status, false, BindPolicy::Optional, Some("bool")),
    row(Prefix::Sts, "sts", Direction::Out, Role::Status, false, BindPolicy::Optional, Some("bool")),
    row(Prefix::Error, "error", Direction::Out, Role::Alarm, false, BindPolicy::Required, Some("bool")),
    row(Prefix::Count, "count", Direction::Out, Role::Counter, false, BindPolicy::Optional, Some("int32")),
    row(Prefix::Pv, "pv", Direction::Out, Role::ProcessValue, false, BindPolicy::Optional, None),
    row(Prefix::Msg, "msg", Direction::Out, Role::Message, false, BindPolicy::Optional, None),
];

impl Prefix {
    pub const ALL: [Prefix; 10] = [
        Prefix::Cmd,
        Prefix::Man,
        Prefix::Ena,
        Prefix::Par,
        Prefix::At,
        Prefix::Sts,
        Prefix::Error,
        Prefix::Count,
        Prefix::Pv,
        Prefix::Msg,
    ];

    pub fn info(self) -> &'static PrefixInfo {
        &TABLE[self as usize]
    }
}

/// `par_ext_time` → `(Par, "ext_time")`。前綴後必須有 `_` 與至少一個字元。
pub fn split(name: &str) -> Option<(Prefix, &str)> {
    Prefix::ALL.into_iter().find_map(|p| {
        let rest = name.strip_prefix(p.info().name)?.strip_prefix('_')?;
        (!rest.is_empty()).then_some((p, rest))
    })
}

/// 名稱是否以任一 `<前綴>_` 開頭（`field` 腳名稱不得如此，規格 §4.1）。
pub fn has_prefix_underscore(name: &str) -> bool {
    Prefix::ALL.iter().any(|p| {
        name.strip_prefix(p.info().name)
            .is_some_and(|r| r.starts_with('_'))
    })
}

/// `ext_time` → `ext`：`<前綴>_` 之後的第一段是「功能」（規格 §4.1）。
pub fn function_of(rest: &str) -> &str {
    rest.split('_').next().unwrap_or(rest)
}

/// 已知功能名詞典（規格 §6.4）。沒有收錄的功能名沒有預設 label。
pub fn default_label(function: &str) -> Option<&'static str> {
    Some(match function {
        "ext" => "伸出",
        "ret" => "縮回",
        "start" => "啟動",
        "stop" => "停止",
        "reset" => "復歸",
        "home" => "原點",
        "auto" => "自動",
        "manual" => "手動",
        "mode" => "模式",
        "up" => "上升",
        "down" => "下降",
        "open" => "開啟",
        "close" => "關閉",
        "run" | "running" => "運轉",
        "estop" => "緊急停止",
        _ => return None,
    })
}
```

- [x] **Step 5: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS（含 Plan 1 全部測試）。

- [x] **Step 6: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨（先 `cargo fmt --all`，`TABLE` 的長行會被 rustfmt 重排）。

---

### Task 2: Unit 類型檔型別與解析（SC2／SC3）

**Files:**
- Create: `machine-model/src/unit_type.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod unit_type;`）

**Interfaces:**
- Consumes: `naming::{Prefix, split, has_prefix_underscore}`、`schema::{Role, Scalar, is_valid_name}`、`strict::StrictMap`、`types::{DataType, ScalarType}`、`diag::Diagnostic`。
- Produces:
  - `unit_type::UnitType { id: String, version: u32, title: String, field: FieldBlock, ctrl: StrictMap<CtrlPin>, parts: StrictMap<String> }`
  - `unit_type::FieldBlock { input: StrictMap<FieldPin> (yaml: in), output: StrictMap<FieldPin> (yaml: out) }`（`Default`）
  - `unit_type::FieldPin { ty: String (yaml: type), required: Option<bool>, label: Option<String>, unit: Option<String> }`
  - `unit_type::CtrlPin { ty: Option<String>, label, kind: Option<Kind>, role: Option<Role>, unit: Option<String>, min: Option<f64>, max: Option<f64>, default: Option<Scalar>, message: Option<String>, level: Option<Level>, reset: Option<Reset> }`；方法 `CtrlPin::type_text(&self, prefix: Prefix) -> Option<&str>`（明寫的 `type`，否則前綴預設型別）
  - `unit_type::{Kind { Momentary, Toggle }, Level { Warning, Abnormal, Emergency }, Reset { Auto, Manual }}`（`Debug, Clone, Copy, PartialEq, Eq, Deserialize`，snake_case）
  - `unit_type::PinRef<'a> { FieldIn(&'a FieldPin), FieldOut(&'a FieldPin), Ctrl(Prefix, &'a CtrlPin), Part(&'a str) }`；`UnitType::lookup(&self, name: &str) -> Option<PinRef<'_>>`（依序找 `field.in`、`field.out`、`ctrl`、`parts`）
  - `unit_type::parse_unit_type(label: &str, text: &str) -> Result<UnitType, Vec<Diagnostic>>`：`label` 是檔名之類的來源名稱，只用在診斷位置

- [x] **Step 1: 寫失敗的測試**

`machine-model/src/unit_type.rs` 先只放（並在 `lib.rs` 加 `pub mod unit_type;`，置於 `pub mod types;` 之後）：

```rust
//! Unit 類型檔（規格 §4）：型別、解析與 SC2（結構）／SC3（前綴與必要屬性）規則。
//!
//! 這裡的檢查是程式碼版；`schema` feature 之後會用 JSON Schema 再驗一次，兩者必須一致。

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const CYLINDER: &str = "\
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
";

    fn codes(text: &str) -> Vec<&'static str> {
        match parse_unit_type("t.yaml", text) {
            Ok(_) => vec![],
            Err(d) => d.iter().map(|x| x.code).collect(),
        }
    }

    fn with(ctrl: &str) -> String {
        format!("id: t\nversion: 1\ntitle: T\nctrl:\n{ctrl}")
    }

    #[test]
    fn spec_example_parses() {
        let t = parse_unit_type("cylinder_double.yaml", CYLINDER).unwrap();
        assert_eq!(t.id, "cylinder_double");
        assert_eq!(t.version, 1);
        assert_eq!(t.field.input.0.len(), 2);
        assert_eq!(t.field.output.0.len(), 2);
        assert_eq!(t.ctrl.0.len(), 11);
        assert_eq!(t.field.input.get("lim_ret").unwrap().required, Some(false));
    }

    #[test]
    fn crlf_and_bom_are_accepted() {
        let text = format!("\u{feff}{}", CYLINDER.replace('\n', "\r\n"));
        assert!(parse_unit_type("t.yaml", &text).is_ok());
    }

    #[test]
    fn lookup_finds_each_kind_of_pin() {
        let t = parse_unit_type("t.yaml", CYLINDER).unwrap();
        assert!(matches!(t.lookup("lim_ext"), Some(PinRef::FieldIn(_))));
        assert!(matches!(t.lookup("valve_ext"), Some(PinRef::FieldOut(_))));
        assert!(matches!(
            t.lookup("man_ext"),
            Some(PinRef::Ctrl(Prefix::Man, _))
        ));
        assert!(t.lookup("nope").is_none());
        let with_part = CYLINDER.replace("parts: {}", "parts: {sub: other_type}");
        let t = parse_unit_type("t.yaml", &with_part).unwrap();
        assert!(matches!(t.lookup("sub"), Some(PinRef::Part("other_type"))));
    }

    #[test]
    fn type_text_uses_explicit_type_then_prefix_default() {
        let t = parse_unit_type(
            "t.yaml",
            &with("  man_a: {}\n  count_n: {}\n  par_x: { type: int16, min: 0, max: 5, default: 1 }\n"),
        )
        .unwrap();
        let pin = |n: &str| match t.lookup(n) {
            Some(PinRef::Ctrl(p, c)) => c.type_text(p).map(str::to_owned),
            _ => None,
        };
        assert_eq!(pin("man_a").as_deref(), Some("bool"));
        assert_eq!(pin("count_n").as_deref(), Some("int32"));
        assert_eq!(pin("par_x").as_deref(), Some("int16"));
    }

    #[test]
    fn sc2_structure_errors() {
        // 缺必填欄位、未知欄位（含 v1 屬性）、重複鍵、壞 YAML
        for text in [
            "id: t\nversion: 1\n".to_owned(),
            "id: t\nversion: 1\ntitle: T\nbogus: 1\n".to_owned(),
            with("  man_a: { writable: true }\n"),
            with("  man_a: { write_permission: P01 }\n"),
            with("  man_a: {}\n  man_a: {}\n"),
            "id: t\nversion: 1\ntitle: [\n".to_owned(),
            "id: 'bad id'\nversion: 1\ntitle: T\n".to_owned(),
            "id: t\nversion: 0\ntitle: T\n".to_owned(),
            "id: t\nversion: 1\ntitle: ''\n".to_owned(),
        ] {
            assert_eq!(codes(&text), ["SC2"], "{text}");
        }
    }

    #[test]
    fn sc3_prefix_rules() {
        // 名稱不符前綴表
        assert_eq!(codes(&with("  lim_ext: { type: bool }\n")), ["SC3"]);
        assert_eq!(codes(&with("  par: {}\n")), ["SC3"]);
        // 需要 type 的前綴
        assert_eq!(codes(&with("  pv_t: {}\n")), ["SC3"]);
        assert_eq!(codes(&with("  msg_m: {}\n")), ["SC3"]);
        // par 需要 min、max、default
        assert_eq!(codes(&with("  par_x: { type: int16 }\n")), ["SC3", "SC3", "SC3"]);
        assert_eq!(
            codes(&with("  par_x: { type: int16, min: 0, max: 5 }\n")),
            ["SC3"]
        );
        // error 需要 message
        assert_eq!(codes(&with("  error_x: {}\n")), ["SC3"]);
        assert_eq!(codes(&with("  error_x: { message: '' }\n")), ["SC3"]);
        // msg 必須是 string:N
        assert_eq!(codes(&with("  msg_m: { type: int16 }\n")), ["SC3"]);
        assert_eq!(codes(&with("  msg_m: { type: 'string:16' }\n")), Vec::<&str>::new());
        // 型別詞彙
        assert_eq!(codes(&with("  man_a: { type: integer }\n")), ["SC3"]);
        assert_eq!(codes(&with("  pv_t: { type: 'string:0' }\n")), ["SC3"]);
    }

    #[test]
    fn sc3_field_and_part_rules() {
        // field 腳名稱不得用控制前綴
        let bad_name = "id: t\nversion: 1\ntitle: T\nfield:\n  in:\n    man_x: { type: bool }\n";
        assert_eq!(codes(bad_name), ["SC3"]);
        // field 型別必須是標量詞彙
        let bad_type = "id: t\nversion: 1\ntitle: T\nfield:\n  out:\n    v: { type: other_unit }\n";
        assert_eq!(codes(bad_type), ["SC3"]);
        // part 的型別是 Unit 類型 id，不能是標量
        let scalar_part = "id: t\nversion: 1\ntitle: T\nparts:\n  p: bool\n";
        assert_eq!(codes(scalar_part), ["SC3"]);
        let bad_part_name = "id: t\nversion: 1\ntitle: T\nparts:\n  'p-1': other\n";
        assert_eq!(codes(bad_part_name), ["SC2"]);
    }

    #[test]
    fn diagnostics_are_traditional_chinese_with_location() {
        let d = parse_unit_type("cyl.yaml", &with("  lim_ext: {}\n")).unwrap_err();
        let text = d[0].to_string();
        assert!(text.starts_with("[SC3] types[t].ctrl[lim_ext]："), "{text}");
        assert!(text.contains("前綴"), "{text}");
        let d = parse_unit_type("cyl.yaml", "id: t\nbogus: 1\n").unwrap_err();
        assert!(d[0].location.as_deref().unwrap().contains("cyl.yaml"));
    }
}
```

- [ ] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model unit_type`
Expected: FAIL（編譯錯誤：`parse_unit_type` 等未定義）。

- [x] **Step 3: 寫實作（放在 `unit_type.rs` 的 `#[cfg(test)]` 之前）**

```rust
use serde::Deserialize;

use crate::diag::Diagnostic;
use crate::naming::{self, Prefix};
use crate::schema::{Role, Scalar, is_valid_name};
use crate::strict::StrictMap;
use crate::types::{DataType, ScalarType};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    pub id: String,
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub field: FieldBlock,
    #[serde(default)]
    pub ctrl: StrictMap<CtrlPin>,
    /// part 名 → Unit 類型 id。
    #[serde(default)]
    pub parts: StrictMap<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBlock {
    #[serde(rename = "in", default)]
    pub input: StrictMap<FieldPin>,
    #[serde(rename = "out", default)]
    pub output: StrictMap<FieldPin>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldPin {
    #[serde(rename = "type")]
    pub ty: String,
    /// `false` 表示可不接；省略視為必接。
    pub required: Option<bool>,
    pub label: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Momentary,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Warning,
    Abnormal,
    Emergency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reset {
    Auto,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CtrlPin {
    #[serde(rename = "type")]
    pub ty: Option<String>,
    pub label: Option<String>,
    pub kind: Option<Kind>,
    pub role: Option<Role>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub message: Option<String>,
    pub level: Option<Level>,
    pub reset: Option<Reset>,
}

impl CtrlPin {
    /// 明寫的 `type`，否則該前綴的預設型別；兩者都沒有回 `None`（SC3 會擋下）。
    pub fn type_text(&self, prefix: Prefix) -> Option<&str> {
        self.ty.as_deref().or(prefix.info().default_type)
    }
}

/// 類型內一個名稱所指的東西。
#[derive(Debug, Clone, Copy)]
pub enum PinRef<'a> {
    FieldIn(&'a FieldPin),
    FieldOut(&'a FieldPin),
    Ctrl(Prefix, &'a CtrlPin),
    /// part 宣告的 Unit 類型 id。
    Part(&'a str),
}

impl UnitType {
    /// 依序在 `field.in`、`field.out`、`ctrl`、`parts` 找名稱。名稱重複（U1）時取第一個。
    pub fn lookup(&self, name: &str) -> Option<PinRef<'_>> {
        if let Some(p) = self.field.input.get(name) {
            return Some(PinRef::FieldIn(p));
        }
        if let Some(p) = self.field.output.get(name) {
            return Some(PinRef::FieldOut(p));
        }
        if let Some(p) = self.ctrl.get(name) {
            let (prefix, _) = naming::split(name)?;
            return Some(PinRef::Ctrl(prefix, p));
        }
        self.parts.get(name).map(|t| PinRef::Part(t.as_str()))
    }
}

/// 解析類型檔文字，並套用 SC2／SC3。`label` 只用在診斷位置（通常是檔名）。
pub fn parse_unit_type(label: &str, text: &str) -> Result<UnitType, Vec<Diagnostic>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let ty: UnitType = serde_yaml_ng::from_str(text).map_err(|e| {
        let detail = e.to_string();
        let message = if detail.contains("unknown field") {
            format!("未知欄位（類型檔只接受規格定義的欄位）：{detail}")
        } else {
            format!("類型檔格式或型別錯誤：{detail}")
        };
        let location = e.location().map_or_else(
            || label.to_owned(),
            |l| format!("{label} 第 {} 行第 {} 欄", l.line(), l.column()),
        );
        vec![Diagnostic::error("SC2", location, message)]
    })?;
    let diags = check_structure(&ty);
    if diags.is_empty() { Ok(ty) } else { Err(diags) }
}

fn prefix_list() -> String {
    let names = Prefix::ALL.map(|p| p.info().name);
    names.join("、")
}

fn check_structure(t: &UnitType) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let root = format!("types[{}]", t.id);

    if !is_valid_name(&t.id) {
        out.push(Diagnostic::error(
            "SC2",
            &root,
            "類型 id 只能使用英數與底線",
        ));
    }
    if t.version == 0 {
        out.push(Diagnostic::error("SC2", &root, "version 必須是正整數"));
    }
    if t.title.trim().is_empty() {
        out.push(Diagnostic::error("SC2", &root, "title 不得為空"));
    }

    for (dir, map) in [("in", &t.field.input), ("out", &t.field.output)] {
        for (name, pin) in &map.0 {
            let loc = format!("{root}.field.{dir}[{name}]");
            if !is_valid_name(name) {
                out.push(Diagnostic::error("SC2", &loc, "接腳名稱只能使用英數與底線"));
            } else if naming::has_prefix_underscore(name) {
                out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("field 腳名稱不得以控制前綴開頭（{}）", prefix_list()),
                ));
            }
            if let Err(e) = DataType::parse(&pin.ty) {
                out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("type `{}`：{e}（field 腳只接受標量詞彙）", pin.ty),
                ));
            }
        }
    }

    for (name, pin) in &t.ctrl.0 {
        let loc = format!("{root}.ctrl[{name}]");
        if !is_valid_name(name) {
            out.push(Diagnostic::error("SC2", &loc, "接腳名稱只能使用英數與底線"));
            continue;
        }
        let Some((prefix, _)) = naming::split(name) else {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!(
                    "ctrl 腳名稱 `{name}` 必須是 <前綴>_<功能>，前綴只能是：{}",
                    prefix_list()
                ),
            ));
            continue;
        };
        match pin.type_text(prefix) {
            None => out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!("`{}_` 前綴的接腳必須明寫 type", prefix.info().name),
            )),
            Some(text) => match DataType::parse(text) {
                Err(e) => out.push(Diagnostic::error(
                    "SC3",
                    &loc,
                    format!("type `{text}`：{e}"),
                )),
                Ok(dt) => {
                    let is_text = matches!(dt.scalar, ScalarType::Text(_)) && !dt.array;
                    if prefix == Prefix::Msg && !is_text {
                        out.push(Diagnostic::error(
                            "SC3",
                            &loc,
                            "`msg_` 接腳的 type 必須是 string:N",
                        ));
                    }
                }
            },
        }
        if prefix == Prefix::Par {
            for (field, present) in [
                ("min", pin.min.is_some()),
                ("max", pin.max.is_some()),
                ("default", pin.default.is_some()),
            ] {
                if !present {
                    out.push(Diagnostic::error(
                        "SC3",
                        &loc,
                        format!("`par_` 接腳必須有 {field}"),
                    ));
                }
            }
        }
        if prefix == Prefix::Error && pin.message.as_deref().is_none_or(|m| m.trim().is_empty()) {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                "`error_` 接腳必須有非空的 message",
            ));
        }
    }

    for (name, ty) in &t.parts.0 {
        let loc = format!("{root}.parts[{name}]");
        if !is_valid_name(name) || !is_valid_name(ty) {
            out.push(Diagnostic::error(
                "SC2",
                &loc,
                "part 名稱與類型 id 只能使用英數與底線",
            ));
        } else if DataType::parse(ty).is_ok() {
            out.push(Diagnostic::error(
                "SC3",
                &loc,
                format!("part 的型別必須是 Unit 類型 id，`{ty}` 是標量詞彙"),
            ));
        }
    }
    out
}
```

注意：`sc3_prefix_rules` 中 `par_x: { type: int16 }` 預期三個 `SC3`（缺 min、max、default），順序固定。

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model unit_type`
Expected: PASS（8 個測試）。`sc2_structure_errors` 若某筆回的不是單一 `SC2`（例如 `title: ''` 同時觸發別的規則），回報實際結果；`with("  man_a: {}\n  man_a: {}\n")` 的重複鍵錯誤來自 `StrictMap`，訊息是「重複的鍵」，不含 `unknown field`，但仍應是 `SC2`。

- [x] **Step 5: Checkpoint**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 全過。測試裡 `format!("{CYLINDER}")` 若被 clippy 指為 `useless_format`，改成 `CYLINDER.to_owned()` 即可。

---

### Task 3: 類型目錄與類型層規則（U1、U12、U15、U17、U18、U19）

**Files:**
- Create: `machine-model/src/catalog.rs`, `machine-model/src/type_rules.rs`, `machine-model/tests/catalog.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod catalog;` 與 `mod type_rules;`）

**Interfaces:**
- Consumes: `unit_type::{UnitType, parse_unit_type, PinRef, CtrlPin}`、`naming`、`types::DataType`、`schema::Scalar`、`diag::Diagnostic`。
- Produces:
  - `catalog::UnitCatalog`；`get(&self, id: &str) -> Option<&UnitType>`、`iter(&self) -> impl Iterator<Item = &UnitType>`、`len()`、`is_empty()`
  - `catalog::Built = Result<(UnitCatalog, Vec<Diagnostic>), Vec<Diagnostic>>`
  - `UnitCatalog::from_sources(sources: &[(&str, &str)]) -> Built`：每項是 `(來源名稱, 類型檔文字)`；成功時 `Vec<Diagnostic>` 是警告與提示（無錯誤）；失敗時是全部診斷
  - `UnitCatalog::with_local(self, locals: &[(&str, &str)]) -> Built`：疊加本地類型，同 id 覆蓋並產生 `Info` `U17`
  - `type_rules::check(types: &BTreeMap<String, UnitType>) -> Vec<Diagnostic>`（`pub(crate)`）

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/catalog.rs`：

```rust
use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;

fn ty(id: &str, version: u32, body: &str) -> String {
    format!("id: {id}\nversion: {version}\ntitle: T\n{body}")
}

fn build(types: &[(&str, String)]) -> Result<Vec<(&'static str, Severity)>, Vec<&'static str>> {
    let sources: Vec<(&str, &str)> = types.iter().map(|(n, t)| (*n, t.as_str())).collect();
    match UnitCatalog::from_sources(&sources) {
        Ok((_, d)) => Ok(d.iter().map(|x| (x.code, x.severity)).collect()),
        Err(d) => Err(d.iter().map(|x| x.code).collect()),
    }
}

fn err_codes(types: &[(&str, String)]) -> Vec<&'static str> {
    build(types).unwrap_err()
}

#[test]
fn valid_catalog_builds_and_can_be_queried() {
    let a = ty("a", 1, "ctrl:\n  man_x: {}\nparts:\n  child: b\n");
    let b = ty("b", 3, "");
    let sources = [("a.yaml", a.as_str()), ("b.yaml", b.as_str())];
    let (cat, diags) = UnitCatalog::from_sources(&sources).unwrap();
    assert!(diags.is_empty());
    assert_eq!(cat.len(), 2);
    assert_eq!(cat.get("b").unwrap().version, 3);
    assert!(cat.get("zzz").is_none());
    assert_eq!(cat.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
}

#[test]
fn syntax_errors_are_reported_per_file() {
    let bad = "id: x\n".to_owned();
    assert_eq!(err_codes(&[("x.yaml", bad)]), ["SC2"]);
}

#[test]
fn u17_duplicate_ids_in_one_layer() {
    let a = ty("same", 1, "");
    assert_eq!(
        err_codes(&[("a.yaml", a.clone()), ("b.yaml", a)]),
        ["U17"]
    );
}

#[test]
fn u1_names_are_unique_across_field_ctrl_and_parts() {
    let t = ty(
        "t",
        1,
        "field:\n  in:\n    dup: { type: bool }\n  out:\n    dup: { type: bool }\nparts:\n  dup: other\n",
    );
    let other = ty("other", 1, "");
    let codes = err_codes(&[("t.yaml", t), ("other.yaml", other)]);
    assert_eq!(codes, ["U1"]);
}

#[test]
fn u12_par_range_and_default() {
    let par = |min: &str, max: &str, def: &str, ty_: &str| {
        ty(
            "t",
            1,
            &format!("ctrl:\n  par_x: {{ type: {ty_}, min: {min}, max: {max}, default: {def} }}\n"),
        )
    };
    // 合法
    assert!(build(&[("t.yaml", par("0", "10", "5", "int16"))]).is_ok());
    assert!(build(&[("t.yaml", par("0", "10", "10", "int16"))]).is_ok());
    assert!(build(&[("t.yaml", par("0.5", "1.0", "0.75", "float32"))]).is_ok());
    // default 超出範圍、min > max、default 不是數字、超出型別範圍
    for bad in [
        par("0", "10", "11", "int16"),
        par("0", "10", "-1", "int16"),
        par("10", "0", "5", "int16"),
        par("0", "10", "true", "int16"),
        par("0", "70000", "5", "int16"),
        par("-1", "10", "5", "uint16"),
    ] {
        assert_eq!(err_codes(&[("t.yaml", bad.clone())]), ["U12"], "{bad}");
    }
}

#[test]
fn u15_flags_a_likely_misspelled_function_name_as_warning() {
    let t = ty(
        "t",
        1,
        "ctrl:\n  man_ext: {}\n  at_ext: {}\n  man_exr: {}\n  man_stop: {}\n",
    );
    let diags = build(&[("t.yaml", t)]).unwrap();
    assert_eq!(diags, [("U15", Severity::Warning)]);
    // 兩個都只出現一次：沒有「多數」可比，不提示
    let t = ty("t", 1, "ctrl:\n  man_ext: {}\n  man_exr: {}\n");
    assert!(build(&[("t.yaml", t)]).unwrap().is_empty());
}

#[test]
fn u18_parts_must_exist_and_must_not_form_a_cycle() {
    let missing = ty("t", 1, "parts:\n  p: ghost\n");
    assert_eq!(err_codes(&[("t.yaml", missing)]), ["U18"]);
    let selfref = ty("t", 1, "parts:\n  p: t\n");
    assert_eq!(err_codes(&[("t.yaml", selfref)]), ["U18"]);
    let a = ty("a", 1, "parts:\n  p: b\n");
    let b = ty("b", 1, "parts:\n  p: a\n");
    // 環上每個類型各報一次
    assert_eq!(
        err_codes(&[("a.yaml", a), ("b.yaml", b)]),
        ["U18", "U18"]
    );
}

#[test]
fn u19_error_pins_must_be_bool() {
    let t = ty("t", 1, "ctrl:\n  error_x: { type: int16, message: m }\n");
    assert_eq!(err_codes(&[("t.yaml", t)]), ["U19"]);
    let ok = ty("t", 1, "ctrl:\n  error_x: { message: m }\n");
    assert!(build(&[("t.yaml", ok)]).is_ok());
}

#[test]
fn local_types_override_standard_ones_with_an_info_notice() {
    let std_a = ty("a", 1, "ctrl:\n  man_x: {}\n");
    let sources = [("a.yaml", std_a.as_str())];
    let (std, _) = UnitCatalog::from_sources(&sources).unwrap();
    let local_a = ty("a", 2, "ctrl:\n  man_x: {}\n  man_y: {}\n");
    let locals = [("local/a.yaml", local_a.as_str())];
    let (merged, diags) = std.with_local(&locals).unwrap();
    assert_eq!(merged.get("a").unwrap().version, 2);
    assert_eq!(diags.len(), 1);
    assert_eq!((diags[0].code, diags[0].severity), ("U17", Severity::Info));
}

#[test]
fn local_types_are_checked_against_the_combined_catalog() {
    // 本地類型的 part 引用標準類型：存在就通過，不存在就是 U18
    let std_b = ty("b", 1, "");
    let (std, _) = UnitCatalog::from_sources(&[("b.yaml", std_b.as_str())]).unwrap();
    let ok = ty("c", 1, "parts:\n  p: b\n");
    assert!(std.clone().with_local(&[("c.yaml", ok.as_str())]).is_ok());
    let bad = ty("c", 1, "parts:\n  p: ghost\n");
    let err = std.with_local(&[("c.yaml", bad.as_str())]).unwrap_err();
    assert_eq!(err.iter().map(|d| d.code).collect::<Vec<_>>(), ["U18"]);
}
```

- [ ] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test catalog`
Expected: FAIL（編譯錯誤：`machine_model::catalog` 不存在）。

- [x] **Step 3: 寫 `catalog.rs`**

```rust
//! Unit 類型目錄（規格 §9.1）：載入多份類型檔、疊加本地類型、套用類型層規則。

use std::collections::BTreeMap;

use crate::diag::Diagnostic;
use crate::type_rules;
use crate::unit_type::{UnitType, parse_unit_type};

/// 成功時附帶警告與提示（沒有錯誤）；失敗時是全部診斷。
pub type Built = Result<(UnitCatalog, Vec<Diagnostic>), Vec<Diagnostic>>;

#[derive(Debug, Clone, PartialEq)]
pub struct UnitCatalog {
    types: BTreeMap<String, UnitType>,
}

/// 解析一層來源；同層 id 重複是 `U17`。
fn parse_layer(sources: &[(&str, &str)], out: &mut Vec<Diagnostic>) -> Vec<UnitType> {
    let mut seen: BTreeMap<String, &str> = BTreeMap::new();
    let mut layer = Vec::new();
    for (label, text) in sources {
        match parse_unit_type(label, text) {
            Err(d) => out.extend(d),
            Ok(t) => {
                if let Some(first) = seen.insert(t.id.clone(), label) {
                    out.push(Diagnostic::error(
                        "U17",
                        format!("types[{}]", t.id),
                        format!("類型 id `{}` 在 `{first}` 與 `{label}` 重複", t.id),
                    ));
                } else {
                    layer.push(t);
                }
            }
        }
    }
    layer
}

fn assemble(base: Vec<UnitType>, local: Vec<UnitType>, mut diags: Vec<Diagnostic>) -> Built {
    let mut types = BTreeMap::new();
    for t in base {
        types.insert(t.id.clone(), t);
    }
    for t in local {
        let id = t.id.clone();
        if types.insert(id.clone(), t).is_some() {
            diags.push(Diagnostic::info(
                "U17",
                format!("types[{id}]"),
                "本地類型覆蓋同 id 的標準類型（實例的 type_version 必須對應本地版本）",
            ));
        }
    }
    diags.extend(type_rules::check(&types));
    if diags.iter().any(Diagnostic::is_error) {
        Err(diags)
    } else {
        Ok((UnitCatalog { types }, diags))
    }
}

impl UnitCatalog {
    pub fn get(&self, id: &str) -> Option<&UnitType> {
        self.types.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &UnitType> {
        self.types.values()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// 由 `(來源名稱, 類型檔文字)` 建目錄。
    pub fn from_sources(sources: &[(&str, &str)]) -> Built {
        let mut diags = Vec::new();
        let layer = parse_layer(sources, &mut diags);
        assemble(layer, Vec::new(), diags)
    }

    /// 疊加機台本地類型；同 id 覆蓋並產生 `Info`。合併後的目錄整體重新檢查。
    pub fn with_local(self, locals: &[(&str, &str)]) -> Built {
        let mut diags = Vec::new();
        let local = parse_layer(locals, &mut diags);
        assemble(self.types.into_values().collect(), local, diags)
    }
}
```

- [x] **Step 4: 寫 `type_rules.rs`**

```rust
//! 類型層規則：U1、U12、U15、U18、U19（U17 在 `catalog`）。輸入是整個目錄，
//! 因為 U18 需要跨類型。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::diag::Diagnostic;
use crate::naming::{self, Prefix};
use crate::schema::Scalar;
use crate::types::DataType;
use crate::unit_type::UnitType;

pub(crate) fn check(types: &BTreeMap<String, UnitType>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for t in types.values() {
        check_unique_names(t, &mut out);
        check_par_ranges(t, &mut out);
        check_error_pins(t, &mut out);
        check_function_spelling(t, &mut out);
    }
    check_parts(types, &mut out);
    out
}

fn check_unique_names(t: &UnitType, out: &mut Vec<Diagnostic>) {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    let names = t
        .field
        .input
        .0
        .keys()
        .chain(t.field.output.0.keys())
        .chain(t.ctrl.0.keys())
        .chain(t.parts.0.keys());
    for name in names {
        *count.entry(name.as_str()).or_default() += 1;
    }
    for (name, n) in count {
        if n > 1 {
            out.push(Diagnostic::error(
                "U1",
                format!("types[{}].{name}", t.id),
                format!("名稱 `{name}` 在 field、ctrl、parts 內出現 {n} 次，必須全體唯一"),
            ));
        }
    }
}

#[allow(clippy::cast_precision_loss)] // 型別界限轉 f64 只用於比較
fn check_par_ranges(t: &UnitType, out: &mut Vec<Diagnostic>) {
    for (name, pin) in &t.ctrl.0 {
        let Some((Prefix::Par, _)) = naming::split(name) else {
            continue;
        };
        let (Some(min), Some(max), Some(default)) = (pin.min, pin.max, pin.default.as_ref())
        else {
            continue; // 缺屬性是 SC3
        };
        let loc = format!("types[{}].ctrl[{name}]", t.id);
        let value = match default {
            Scalar::Int(i) => *i as f64,
            Scalar::Float(f) => *f,
            _ => {
                out.push(Diagnostic::error("U12", &loc, "par_ 的 default 必須是數字"));
                continue;
            }
        };
        // 用否定寫法，NaN 也會被擋下
        if !(min <= value && value <= max) {
            out.push(Diagnostic::error(
                "U12",
                &loc,
                format!("必須滿足 min ≤ default ≤ max（min={min}、default={value}、max={max}）"),
            ));
        }
        let bounds = pin
            .type_text(Prefix::Par)
            .and_then(|text| DataType::parse(text).ok())
            .and_then(|dt| dt.int_bounds());
        if let Some((lo, hi)) = bounds {
            for (what, v) in [("min", min), ("max", max), ("default", value)] {
                if v < lo as f64 || v > hi as f64 {
                    out.push(Diagnostic::error(
                        "U12",
                        &loc,
                        format!("{what}={v} 超出型別 `{}` 的範圍", pin.ty.as_deref().unwrap_or("")),
                    ));
                }
            }
        }
    }
}

fn check_error_pins(t: &UnitType, out: &mut Vec<Diagnostic>) {
    for (name, pin) in &t.ctrl.0 {
        let Some((Prefix::Error, _)) = naming::split(name) else {
            continue;
        };
        let is_bool = pin
            .type_text(Prefix::Error)
            .and_then(|text| DataType::parse(text).ok())
            .is_some_and(|dt| dt.is_bool() && !dt.array);
        if !is_bool {
            out.push(Diagnostic::error(
                "U19",
                format!("types[{}].ctrl[{name}]", t.id),
                "error_ 接腳的型別必須是 bool",
            ));
        }
    }
}

/// 只差一個字元（取代、插入或刪除），且兩者不同。
fn within_one_edit(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a == b {
        return false;
    }
    let (short, long) = if a.len() <= b.len() { (&a, &b) } else { (&b, &a) };
    if long.len() - short.len() > 1 {
        return false;
    }
    let i = short.iter().zip(long.iter()).take_while(|(x, y)| x == y).count();
    if short.len() == long.len() {
        short[i + 1..] == long[i + 1..]
    } else {
        short[i..] == long[i + 1..]
    }
}

fn check_function_spelling(t: &UnitType, out: &mut Vec<Diagnostic>) {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for name in t.ctrl.0.keys() {
        if let Some((_, rest)) = naming::split(name) {
            *count.entry(naming::function_of(rest)).or_default() += 1;
        }
    }
    for (&f, &n) in &count {
        if n != 1 {
            continue;
        }
        let neighbour = count
            .iter()
            .find(|&(&g, &m)| m >= 2 && within_one_edit(f, g));
        if let Some((&g, &m)) = neighbour {
            out.push(Diagnostic::warning(
                "U15",
                format!("types[{}]", t.id),
                format!("功能名 `{f}` 只出現一次，與出現 {m} 次的 `{g}` 只差一個字元，是否拼錯？"),
            ));
        }
    }
}

fn check_parts(types: &BTreeMap<String, UnitType>, out: &mut Vec<Diagnostic>) {
    for t in types.values() {
        for (part, ty) in &t.parts.0 {
            if !types.contains_key(ty) {
                out.push(Diagnostic::error(
                    "U18",
                    format!("types[{}].parts[{part}]", t.id),
                    format!("part 的類型 `{ty}` 不在目錄內"),
                ));
            }
        }
    }
    // 環：從每個類型出發，看能不能經由 parts 回到自己。
    for t in types.values() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut queue: VecDeque<&str> = t.parts.0.values().map(String::as_str).collect();
        let mut cyclic = false;
        while let Some(id) = queue.pop_front() {
            if id == t.id {
                cyclic = true;
                break;
            }
            if !seen.insert(id) {
                continue;
            }
            if let Some(next) = types.get(id) {
                queue.extend(next.parts.0.values().map(String::as_str));
            }
        }
        if cyclic {
            out.push(Diagnostic::error(
                "U18",
                format!("types[{}]", t.id),
                "parts 關係成環（含自我引用）",
            ));
        }
    }
}
```

在 `lib.rs` 加 `pub mod catalog;`（`pub mod address;` 之後）與 `mod type_rules;`（`pub mod types;` 之後）。

- [x] **Step 5: 執行測試**

Run: `cargo test -p machine-model`
Expected: PASS。若 `u12_par_range_and_default` 的 `par("0", "70000", "5", "int16")` 沒有恰好一個 `U12`（min≤default≤max 成立，只有 max 超出型別），代表邊界檢查有誤，修實作而不是測試。`u15` 的第一個案例：`ext`×2、`exr`×1（與 `ext` 差一字）、`stop`×1（與 `ext` 距離 > 1）→ 恰一個警告。

- [x] **Step 6: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨。

---

### Task 4: 標準類型、內嵌與版本釘選

**Files:**
- Create: `standards/unit-types/cylinder_double.yaml`, `standards/unit-types/station_load.yaml`, `standards/unit-types/machine_ctrl.yaml`, `standards/unit-types.pins`, `machine-model/src/hash.rs`, `machine-model/src/standard.rs`, `machine-model/tests/standard_pins.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod hash;`、`pub mod standard;`）、`AGENTS.md`

**Interfaces:**
- Produces:
  - `hash::content_hash(text: &str) -> u64`：FNV-1a 64；去掉開頭 BOM、忽略所有 `\r`，所以 CRLF／LF 結果相同
  - `hash::hex(value: u64) -> String`（16 位小寫十六進位）
  - `standard::standard_sources() -> &'static [(&'static str, &'static str)]`：`(來源名稱如 "unit-types/cylinder_double.yaml", 內容)`
  - `UnitCatalog::standard() -> UnitCatalog`（`standard.rs` 內的 `impl UnitCatalog`）；內嵌類型有誤時 panic（測試保證不會發生）

- [x] **Step 1: 建立三個標準類型檔**

`standards/unit-types/cylinder_double.yaml`（規格 §4 範例）：

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

`standards/unit-types/station_load.yaml`：

```yaml
id: station_load
version: 1
title: 上料站
ctrl:
  man_start:      { label: 啟動 }
  man_stop:       { label: 停止 }
  at_running:     {}
  par_cycle_time: { type: int16, unit: ms, min: 0, max: 30000, default: 1000 }
  count_done:     {}
parts:
  cyl: cylinder_double
```

`standards/unit-types/machine_ctrl.yaml`（最小框架，規格 §11 #5；完整框架留 P6）：

```yaml
id: machine_ctrl
version: 1
title: 機台根（最小框架）
ctrl:
  man_start:   { label: 啟動 }
  man_stop:    { label: 停止 }
  man_reset:   { label: 復歸 }
  man_mode:    { type: int16 }
  at_mode:     { type: int16 }
  at_running:  {}
  error_estop: { message: 緊急停止, level: emergency, reset: manual }
parts:
  stn: station_load
```

- [x] **Step 2: 寫失敗的測試與空的釘選檔**

`standards/unit-types.pins`（先只放註解）：

```
# 標準 Unit 類型的版本釘選：<id> <version> <內容雜湊>
# 改了類型檔內容就必須調高 version，並重新產生本檔：
#   cargo test -p machine-model --test standard_pins print_pins -- --ignored --nocapture
```

`machine-model/tests/standard_pins.rs`：

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;
use machine_model::hash::{content_hash, hex};
use machine_model::standard::standard_sources;
use machine_model::unit_type::parse_unit_type;

const PINS: &str = include_str!("../../standards/unit-types.pins");

/// 目前內嵌的每個標準類型：id → (version, 雜湊)。
fn actual() -> BTreeMap<String, (u32, String)> {
    standard_sources()
        .iter()
        .map(|(label, text)| {
            let t = parse_unit_type(label, text).expect("標準類型必須能解析");
            (t.id, (t.version, hex(content_hash(text))))
        })
        .collect()
}

fn pinned() -> BTreeMap<String, (u32, String)> {
    PINS.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut it = l.split_whitespace();
            let id = it.next().expect("id").to_owned();
            let version = it.next().expect("version").parse().expect("version 是整數");
            let hash = it.next().expect("hash").to_owned();
            (id, (version, hash))
        })
        .collect()
}

#[test]
#[ignore = "手動執行：產生 standards/unit-types.pins 的內容"]
fn print_pins() {
    for (id, (version, hash)) in actual() {
        println!("{id} {version} {hash}");
    }
}

#[test]
fn every_standard_type_is_pinned_and_unchanged() {
    let pinned = pinned();
    for (id, (version, hash)) in actual() {
        let Some((pv, ph)) = pinned.get(&id) else {
            panic!("標準類型 `{id}` 沒有釘版本；請執行 print_pins 並更新 standards/unit-types.pins");
        };
        assert_eq!(
            *pv, version,
            "`{id}` 的 version 從 {pv} 變成 {version}；請更新 standards/unit-types.pins"
        );
        assert_eq!(
            ph, &hash,
            "`{id}` 的內容改變了但 version 仍是 {version}；改類型檔必須調高 version"
        );
    }
}

#[test]
fn pins_file_has_no_stale_entries() {
    let actual = actual();
    for id in pinned().keys() {
        assert!(actual.contains_key(id), "pins 檔列了不存在的類型 `{id}`");
    }
}

#[test]
fn content_hash_ignores_line_endings_and_bom() {
    let lf = "id: x\nversion: 1\n";
    let crlf = "id: x\r\nversion: 1\r\n";
    let bom = "\u{feff}id: x\nversion: 1\n";
    assert_eq!(content_hash(lf), content_hash(crlf));
    assert_eq!(content_hash(lf), content_hash(bom));
    assert_ne!(content_hash(lf), content_hash("id: y\nversion: 1\n"));
    assert_eq!(hex(255), "00000000000000ff");
}

#[test]
fn every_file_in_the_standards_directory_is_embedded() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../standards/unit-types");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("找不到 standards/unit-types")
        .map(|e| e.expect("目錄項").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".yaml"))
        .map(|n| format!("unit-types/{n}"))
        .collect();
    on_disk.sort();
    let mut embedded: Vec<String> = standard_sources().iter().map(|(l, _)| (*l).to_owned()).collect();
    embedded.sort();
    assert_eq!(on_disk, embedded, "新增標準類型檔後要加進 standard.rs 的 STANDARD");
}

#[test]
fn standard_catalog_builds_with_no_errors_or_warnings() {
    let (catalog, diags) = UnitCatalog::from_sources(standard_sources())
        .unwrap_or_else(|d| panic!("標準目錄有錯誤：{d:#?}"));
    assert!(
        diags.iter().all(|d| d.severity == Severity::Info),
        "{diags:#?}"
    );
    assert_eq!(catalog.len(), standard_sources().len());
    assert_eq!(UnitCatalog::standard(), catalog);
}

#[test]
fn spec_example_types_are_present() {
    let c = UnitCatalog::standard();
    for id in ["cylinder_double", "station_load", "machine_ctrl"] {
        assert!(c.get(id).is_some(), "{id}");
    }
    assert_eq!(c.get("machine_ctrl").unwrap().parts.get("stn").unwrap(), "station_load");
}
```

- [ ] **Step 3: 確認測試失敗**

Run: `cargo test -p machine-model --test standard_pins`
Expected: FAIL（編譯錯誤：`machine_model::hash` 不存在）。

- [x] **Step 4: 寫實作**

`machine-model/src/hash.rs`：

```rust
//! 內容雜湊（FNV-1a 64）。用於標準類型的版本釘選與解析結果的指紋；不是密碼學雜湊。

/// 去掉開頭 BOM、忽略所有 `\r`（Windows 的 CRLF 與 LF 得到相同結果）。
pub fn content_hash(text: &str) -> u64 {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes().filter(|b| *b != b'\r') {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// 16 位小寫十六進位。
pub fn hex(value: u64) -> String {
    format!("{value:016x}")
}
```

`machine-model/src/standard.rs`：

```rust
//! 內嵌的標準 Unit 類型（規格 §9.1）：編譯時以 `include_str!` 帶進函式庫，
//! 現場只需要單一 `machine.yaml`。新增標準類型 = 新增檔案並加進 [`STANDARD`]，
//! 同時更新 `standards/unit-types.pins`。

use crate::catalog::UnitCatalog;

static STANDARD: &[(&str, &str)] = &[
    (
        "unit-types/cylinder_double.yaml",
        include_str!("../../standards/unit-types/cylinder_double.yaml"),
    ),
    (
        "unit-types/machine_ctrl.yaml",
        include_str!("../../standards/unit-types/machine_ctrl.yaml"),
    ),
    (
        "unit-types/station_load.yaml",
        include_str!("../../standards/unit-types/station_load.yaml"),
    ),
];

/// `(來源名稱, 內容)`，順序固定（依檔名）。
pub fn standard_sources() -> &'static [(&'static str, &'static str)] {
    STANDARD
}

impl UnitCatalog {
    /// 只含內嵌標準類型的目錄。內嵌內容有誤時 panic——`tests/standard_pins.rs` 保證不會發生。
    pub fn standard() -> Self {
        Self::from_sources(STANDARD)
            .map(|(catalog, _)| catalog)
            .expect("內嵌的標準類型必須通過檢查")
    }
}
```

`lib.rs` 加 `pub mod hash;`（`pub mod family;` 之後）與 `pub mod standard;`（`pub mod schema;` 之後）。

在 `AGENTS.md` 的「規則」清單最後加一行：

```markdown
- 標準 Unit 類型放 `standards/unit-types/<id>.yaml`，並加進 `machine-model/src/standard.rs` 的 `STANDARD` 與 `standards/unit-types.pins`。改類型檔內容必須調高 `version`（`tests/standard_pins.rs` 把關）。
```

- [x] **Step 5: 產生釘選檔**

Run: `cargo test -p machine-model --test standard_pins print_pins -- --ignored --nocapture`
把輸出中形如 `cylinder_double 1 <16 位十六進位>` 的三行（不含 cargo 的其他輸出）附加到 `standards/unit-types.pins` 註解之後。

- [x] **Step 6: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS（含 `every_standard_type_is_pinned_and_unchanged`）。若 `standard_catalog_builds_with_no_errors_or_warnings` 出現 `U15` 警告，代表標準類型檔的功能名被判定拼錯——回報，不要靠刪警告過關。

- [x] **Step 7: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨。

---

### Task 5: 實例層規則（一）：單元圖（U2、U3、U5、U7、U8、U9）

**Files:**
- Create: `machine-model/src/analyze/mod.rs`, `machine-model/src/analyze/graph.rs`, `machine-model/tests/analyze_graph.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod analyze;`）、`machine-model/tests/common/mod.rs`（追加）

**Interfaces:**
- Consumes: `catalog::UnitCatalog`、`unit_type::PinRef`、`schema::{MachineModel, UnitInstance, MACHINE_UNIT, BindRef, classify_bind}`。
- Produces:
  - `analyze::check(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic>`（Task 6 完成後 = 圖規則 ＋ 接腳規則；本任務先等於圖規則）
  - `analyze::check_graph(model, catalog) -> Vec<Diagnostic>`、`analyze::check_pins(model, catalog) -> Vec<Diagnostic>`（本任務的 `check_pins` 先回空）
  - 測試工具 `common::{LEAF, MID, ROOT, catalog, TREE, graph_codes}`

演算法要求：**不得遞迴**（用 Kahn 與 BFS），避免環或深樹造成堆疊溢位或無窮迴圈。

- [x] **Step 1: 寫失敗的測試**

在 `machine-model/tests/common/mod.rs` 檔尾追加：

```rust
use machine_model::analyze;
use machine_model::catalog::UnitCatalog;
use machine_model::parse;

/// 測試用的最小類型：leaf（有 field、ctrl）、mid（兩個 leaf part）、root（一個 mid part）。
pub const LEAF: &str = "\
id: leaf
version: 1
title: Leaf
field:
  in:
    i1: { type: bool }
    i2: { type: bool, required: false }
  out:
    o1: { type: bool }
ctrl:
  man_go: {}
  par_t: { type: int16, min: 0, max: 10, default: 5 }
  error_e: { message: 逾時 }
  at_ok: {}
  pv_temp: { type: float32 }
";
pub const MID: &str = "id: mid\nversion: 1\ntitle: Mid\nparts:\n  a: leaf\n  b: leaf\n";
pub const ROOT: &str =
    "id: root\nversion: 1\ntitle: Root\nctrl:\n  man_start: {}\nparts:\n  m: mid\n";

pub fn catalog() -> UnitCatalog {
    UnitCatalog::from_sources(&[("leaf", LEAF), ("mid", MID), ("root", ROOT)])
        .expect("測試目錄必須有效")
        .0
}

/// 合法的單元樹：machine → mid1 → (leaf1, leaf2)。
pub const TREE: &str = "\
  - {id: machine, type: root, type_version: 1, bind: {m: mid1}}
  - {id: mid1, type: mid, type_version: 1, bind: {a: leaf1, b: leaf2}}
  - {id: leaf1, type: leaf, type_version: 1}
  - {id: leaf2, type: leaf, type_version: 1}
";

/// 只跑圖規則（不跑載入安全規則與接腳規則），回傳規則編號。
pub fn graph_codes(units: &str) -> Vec<&'static str> {
    let model = parse(&doc(&format!("units:\n{units}"))).expect("測試資料必須能解析");
    analyze::check_graph(&model, &catalog())
        .iter()
        .map(|d| d.code)
        .collect()
}
```

`machine-model/tests/analyze_graph.rs`：

```rust
mod common;

use common::{TREE, graph_codes};

#[test]
fn a_valid_tree_has_no_graph_diagnostics() {
    assert_eq!(graph_codes(TREE), Vec::<&str>::new());
}

#[test]
fn u2_type_must_exist_and_version_must_match() {
    let unknown = TREE.replace("type: leaf, type_version: 1}\n  - {id: leaf2", "type: ghost, type_version: 1}\n  - {id: leaf2");
    assert_eq!(graph_codes(&unknown), ["U2"]);
    // 實例釘舊版（類型目錄是 1，實例寫 2 或 0 以外的不符版本）
    let stale = TREE.replace("id: leaf2, type: leaf, type_version: 1", "id: leaf2, type: leaf, type_version: 2");
    assert_eq!(graph_codes(&stale), ["U2"]);
}

#[test]
fn u3_bind_keys_must_be_pins_or_parts_of_the_type() {
    let bad = TREE.replace("id: leaf1, type: leaf, type_version: 1", "id: leaf1, type: leaf, type_version: 1, bind: {zzz: null}");
    assert_eq!(graph_codes(&bad), ["U3"]);
    // 接腳名是合法的 key（接腳規則在 Task 6，這裡不報）
    let pin = TREE.replace("id: leaf1, type: leaf, type_version: 1", "id: leaf1, type: leaf, type_version: 1, bind: {i1: null}");
    assert_eq!(graph_codes(&pin), Vec::<&str>::new());
}

#[test]
fn u5_part_must_bind_an_existing_unit_of_the_declared_type() {
    // 不存在的單元
    let ghost = TREE.replace("a: leaf1", "a: nobody");
    assert!(graph_codes(&ghost).contains(&"U5"));
    // 類型不符：part a 宣告 leaf，卻綁了 mid 類型的單元
    let wrong = TREE.replace("a: leaf1", "a: mid1");
    assert!(graph_codes(&wrong).contains(&"U5"));
    // part 綁 Tag 而不是 Unit
    let tag = TREE.replace("a: leaf1", "a: plc1.T");
    assert!(graph_codes(&tag).contains(&"U5"));
    // null 是刻意不接，合法
    let null = TREE.replace("a: leaf1", "a: null");
    assert_eq!(graph_codes(&null), ["U8"]); // leaf1 不再被任何人綁 → 無法由 machine 到達
}

#[test]
fn u7_a_unit_has_at_most_one_parent() {
    let two = TREE.replace("b: leaf2", "b: leaf1");
    let codes = graph_codes(&two);
    assert!(codes.contains(&"U7"), "{codes:?}");
}

#[test]
fn u8_unreachable_units() {
    let orphan = format!("{TREE}  - {{id: orphan, type: leaf, type_version: 1}}\n");
    assert_eq!(graph_codes(&orphan), ["U8"]);
}

#[test]
fn u8_cycles_do_not_hang_and_are_reported() {
    // mid1 與 mid2 互相當對方的 part（同時也是 U5 類型不符）
    let units = "\
  - {id: machine, type: root, type_version: 1, bind: {m: mid1}}
  - {id: mid1, type: mid, type_version: 1, bind: {a: mid2}}
  - {id: mid2, type: mid, type_version: 1, bind: {a: mid1}}
";
    let codes = graph_codes(units);
    assert!(codes.contains(&"U8"), "{codes:?}");
    // 自我綁定
    let selfie = TREE.replace("id: leaf1, type: leaf, type_version: 1", "id: leaf1, type: mid, type_version: 1, bind: {a: leaf1}");
    let codes = graph_codes(&selfie);
    assert!(codes.contains(&"U8"), "{codes:?}");
}

#[test]
fn u9_machine_must_exist_and_be_the_root() {
    let no_machine = "\
  - {id: mid1, type: mid, type_version: 1, bind: {a: leaf1, b: leaf2}}
  - {id: leaf1, type: leaf, type_version: 1}
  - {id: leaf2, type: leaf, type_version: 1}
";
    assert_eq!(graph_codes(no_machine), ["U9"]);
    // machine 被別人當子層
    let child = TREE.replace("a: leaf1", "a: machine");
    assert!(graph_codes(&child).contains(&"U9"));
}

#[test]
fn diagnostics_are_traditional_chinese_with_location() {
    use common::{catalog, doc};
    let model = machine_model::parse(&doc("units:\n  - {id: u, type: ghost, type_version: 1}\n")).unwrap();
    let d = machine_model::analyze::check_graph(&model, &catalog());
    let text = d.iter().find(|x| x.code == "U2").unwrap().to_string();
    assert!(text.starts_with("[U2] units[u]："), "{text}");
    assert!(text.contains("類型目錄"), "{text}");
}
```

注意 `u2_...` 測試第一個案例的 `replace` 字串很容易抄錯：它的目的只是把 `leaf1` 的 `type` 換成 `ghost`；實作者可改寫成更直接的字串（例如整行替換 `  - {id: leaf1, type: leaf, type_version: 1}\n`），只要效果相同、並且預期仍是單一 `U2`。

- [ ] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test analyze_graph`
Expected: FAIL（編譯錯誤：`machine_model::analyze` 不存在）。

- [x] **Step 3: 寫實作**

`machine-model/src/analyze/mod.rs`：

```rust
//! 實例層規則（U2～U11、U13、U14，規格 §8.3）：需要類型目錄。
//! 類型層規則（U1、U12、U15、U17～U19）在 [`crate::catalog`] 建目錄時檢查。
//!
//! 先決條件：模型已通過載入安全規則（[`crate::load`]）。本模組對違反先決條件的輸入
//! 不會 panic，但診斷可能不完整。
//! 注意：規格 U1 的「units[].id 唯一」已在載入安全規則由 [`crate::rules::r26_duplicate_unit_id`]
//! 負責檢查；本模組的 [`check`] 假設模型已通過 [`crate::load`]。單元類型內名稱唯一性（U1）
//! 則由 [`crate::catalog`] 建構時檢查。

mod graph;
mod pins;

use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

/// 全部實例層規則：先圖規則，再接腳規則。
pub fn check(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = check_graph(model, catalog);
    out.extend(check_pins(model, catalog));
    out
}

/// U2、U3、U5、U7、U8、U9。
pub fn check_graph(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    graph::check(model, catalog, &mut out);
    out
}

/// U4、U6、U10、U11、U13、U14。
pub fn check_pins(model: &MachineModel, catalog: &UnitCatalog) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    pins::check(model, catalog, &mut out);
    out
}
```

`machine-model/src/analyze/pins.rs`（本任務先放空殼，Task 6 實作）：

```rust
use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

pub(super) fn check(_model: &MachineModel, _catalog: &UnitCatalog, _out: &mut Vec<Diagnostic>) {}
```

`machine-model/src/analyze/graph.rs`：

```rust
//! 單元圖規則：U2、U3、U5、U7、U8、U9。全部用迭代演算法（Kahn、BFS），不遞迴。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::schema::{BindRef, MACHINE_UNIT, MachineModel, UnitInstance, classify_bind};
use crate::unit_type::PinRef;

pub(super) fn check(m: &MachineModel, catalog: &UnitCatalog, out: &mut Vec<Diagnostic>) {
    let by_id: BTreeMap<&str, &UnitInstance> =
        m.units.iter().map(|u| (u.id.as_str(), u)).collect();
    // 只含 id 存在的邊：父 → 子（可重複，代表同一父層以多個 part 綁同一子層）
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut parents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for u in &m.units {
        let loc = format!("units[{}]", u.id);
        let Some(ty) = catalog.get(&u.ty) else {
            out.push(Diagnostic::error(
                "U2",
                &loc,
                format!("類型 `{}` 不在類型目錄內", u.ty),
            ));
            continue;
        };
        if ty.version != u.type_version {
            out.push(Diagnostic::error(
                "U2",
                &loc,
                format!(
                    "type_version {} 與類型目錄中 `{}` 的 version {} 不符；不會靜默採用，請確認要用哪一版",
                    u.type_version, u.ty, ty.version
                ),
            ));
        }
        for (key, value) in &u.bind.0 {
            match ty.lookup(key) {
                None => out.push(Diagnostic::error(
                    "U3",
                    &loc,
                    format!("bind 的鍵 `{key}` 不是類型 `{}` 的接腳或 part", u.ty),
                )),
                Some(PinRef::Part(part_ty)) => {
                    let Some(value) = value else {
                        continue; // null：刻意不接
                    };
                    let Some(BindRef::Unit(child_id)) = classify_bind(value) else {
                        out.push(Diagnostic::error(
                            "U5",
                            &loc,
                            format!("part `{key}` 必須綁單元 id（不含 `.`），而不是 `{value}`"),
                        ));
                        continue;
                    };
                    match by_id.get(child_id) {
                        None => out.push(Diagnostic::error(
                            "U5",
                            &loc,
                            format!("part `{key}` 綁定的單元 `{child_id}` 不存在"),
                        )),
                        Some(child) => {
                            // 若子層類型本身已 U2（不在目錄內），跳過 U5 的類型比對以避免連鎖雜訊（邊照加，供 U8 可達性與環分析使用）
                            if catalog.get(&child.ty).is_some() && child.ty != part_ty {
                                out.push(Diagnostic::error(
                                    "U5",
                                    &loc,
                                    format!(
                                        "part `{key}` 宣告的類型是 `{part_ty}`，但單元 `{child_id}` 的類型是 `{}`",
                                        child.ty
                                    ),
                                ));
                            }
                            children.entry(u.id.as_str()).or_default().push(child.id.as_str());
                            parents.entry(child.id.as_str()).or_default().push(u.id.as_str());
                        }
                    }
                }
                Some(_) => {} // field／ctrl 腳由接腳規則處理
            }
        }
    }

    for (child, ps) in &parents {
        if ps.len() > 1 {
            out.push(Diagnostic::error(
                "U7",
                format!("units[{child}]"),
                format!(
                    "被 {} 個 part 綁定（{}）；每個單元只能有一個父層",
                    ps.len(),
                    ps.join("、")
                ),
            ));
        }
    }

    if !by_id.contains_key(MACHINE_UNIT) {
        out.push(Diagnostic::error(
            "U9",
            "units",
            "缺少根單元 `machine`（machine.yaml 必須宣告 id 為 machine 的單元）",
        ));
    } else if let Some(ps) = parents.get(MACHINE_UNIT) {
        out.push(Diagnostic::error(
            "U9",
            format!("units[{MACHINE_UNIT}]"),
            format!("`machine` 必須是根，不得被 {} 綁定", ps.join("、")),
        ));
    }

    let cyclic = cycle_members(&by_id, &children);
    for id in &cyclic {
        out.push(Diagnostic::error(
            "U8",
            format!("units[{id}]"),
            "參與包含關係的環，或位於環之下",
        ));
    }

    if by_id.contains_key(MACHINE_UNIT) {
        let reached = reachable_from(MACHINE_UNIT, &children);
        for id in by_id.keys() {
            if !reached.contains(id) && !cyclic.contains(id) {
                out.push(Diagnostic::error(
                    "U8",
                    format!("units[{id}]"),
                    "無法由 `machine` 到達（沒有被任何父層綁定）",
                ));
            }
        }
    }
}

/// Kahn 拓撲排序：移除所有能被剝掉的節點，剩下的就是環上或環之下的節點。
fn cycle_members<'a>(
    by_id: &BTreeMap<&'a str, &UnitInstance>,
    children: &BTreeMap<&'a str, Vec<&'a str>>,
) -> BTreeSet<&'a str> {
    let mut indegree: BTreeMap<&str, usize> = by_id.keys().map(|id| (*id, 0)).collect();
    for cs in children.values() {
        for c in cs {
            if let Some(n) = indegree.get_mut(c) {
                *n += 1;
            }
        }
    }
    let mut queue: VecDeque<&str> = indegree
        .iter()
        .filter(|&(_, &n)| n == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut removed: BTreeSet<&str> = BTreeSet::new();
    while let Some(id) = queue.pop_front() {
        removed.insert(id);
        for c in children.get(id).into_iter().flatten() {
            if let Some(n) = indegree.get_mut(c) {
                *n -= 1;
                if *n == 0 {
                    queue.push_back(c);
                }
            }
        }
    }
    by_id
        .keys()
        .filter(|id| !removed.contains(*id))
        .copied()
        .collect()
}

fn reachable_from<'a>(root: &'a str, children: &BTreeMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue = VecDeque::from([root]);
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        queue.extend(children.get(id).into_iter().flatten().copied());
    }
    seen
}
```

`lib.rs` 加 `pub mod analyze;`（`pub mod address;` 之後）。

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。`u5_...` 最後一個 `null` 案例預期恰為 `["U8"]`（`leaf1` 沒人綁）；`u7` 的案例 `b: leaf1` 會讓 `leaf2` 變成孤兒，所以實際回 `["U7", "U8"]`——測試用 `contains`，不要改成相等。若任何案例卡住或超時，代表混進了遞迴／無窮迴圈，必須修實作。

- [x] **Step 5: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨。

---

### Task 6: 實例層規則（二）：接腳綁定（U4、U6、U10、U11、U13、U14）

**Files:**
- Modify: `machine-model/src/analyze/pins.rs`（取代空殼）、`machine-model/tests/common/mod.rs`（追加）
- Create: `machine-model/tests/analyze_pins.rs`

**Interfaces:**
- Consumes: Task 5 的 `analyze` 骨架；`naming::{Prefix, BindPolicy}`；`address::parse`；`types::DataType`；`schema::{split_id, classify_bind, BindRef, Tag, Plc}`；`unit_type::PinRef`。
- Produces: `analyze::check_pins`（實作完成）；測試工具 `common::{BASE_TAGS, pin_codes}`。

規則細節（實作者必須照做）：
- **U4（field 腳）**：值必須是 `null` 或 Tag id；Tag 必須存在；`field.in` 須綁唯讀輸入位址類別（`ClassProfile.read_only`，如 X）、`field.out` 須綁實體輸出位址類別（`ClassProfile.name == "Y"`）；Tag 型別與腳位 `type` 必須**完全相同**（`DataType` 相等，含陣列與字串長度；故意從嚴）。
- **U10（ctrl 腳）**：值必須是 `null` 或 Tag id；Tag 必須存在；型別與腳位有效型別（`CtrlPin::type_text`）完全相同。
- **U6**：`field` 腳（`required != Some(false)`）完全沒出現在 `bind`；`error_` 腳沒出現、或值為 `null`。（field 腳明確寫 `null` 視為刻意不接，通過。）
- **U11（多重綁定限制）**：只對存在的 Tag 進行檢查（避免幽靈 Tag 誤報）。`field.out` 綁定的 Tag 必須全機唯一（杜絕實體致動器輸出衝突）；其餘接腳（`field.in`、`par_*`、`man_*`、`at_*` 等）允許多個單元接腳綁定同一個 Tag（如共用感測輸入、共用參數、共用同動操作），但同一 Tag 綁定的所有接腳前綴與語意必須相容：禁止混用 field 與 ctrl、禁止混用寫入候選與唯讀狀態、型別必須一致。
- **U13**：`write_candidate` 前綴（`man`、`par`）的腳綁到唯讀位址類別的 Tag，或綁到 `safety_monitor: true` 的 Tag。兩個條件各報一次。
- **U14（Warning）**：Tag 明確的 `role` 與該腳的有效 role（腳的 `role`，否則前綴推測）不同。

- [x] **Step 1: 寫失敗的測試**

在 `machine-model/tests/common/mod.rs` 檔尾追加：

```rust
/// leaf1、leaf2 各自完整綁定所需的 Tag（leaf1 用 L1_*，leaf2 用 L2_*）。
pub const BASE_TAGS: &[&str] = &[
    "{id: plc1.L1_in, address: X10, type: bool}",
    "{id: plc1.L1_out, address: Y10, type: bool}",
    "{id: plc1.L1_err, address: M100, type: bool}",
    "{id: plc1.L2_in, address: X20, type: bool}",
    "{id: plc1.L2_out, address: Y20, type: bool}",
    "{id: plc1.L2_err, address: M200, type: bool}",
];

/// 只跑接腳規則。`leaf1_bind` 是 leaf1 的 bind 內容（flow 寫法，不含外層大括號），
/// `extra_tags` 附加在 [`BASE_TAGS`] 之後；`override_tag` 若給，會取代同 id 的基本 Tag。
pub fn pin_codes(leaf1_bind: &str, extra_tags: &[&str]) -> Vec<&'static str> {
    let units = format!(
        "  - {{id: machine, type: root, type_version: 1, bind: {{m: mid1}}}}\n\
         \x20 - {{id: mid1, type: mid, type_version: 1, bind: {{a: leaf1, b: leaf2}}}}\n\
         \x20 - {{id: leaf1, type: leaf, type_version: 1, bind: {{{leaf1_bind}}}}}\n\
         \x20 - {{id: leaf2, type: leaf, type_version: 1, bind: {{i1: plc1.L2_in, o1: plc1.L2_out, error_e: plc1.L2_err}}}}\n"
    );
    let mut entries: Vec<&str> = BASE_TAGS.to_vec();
    entries.extend_from_slice(extra_tags);
    let text = doc(&format!("units:\n{units}{}", tags(&entries)));
    let model = parse(&text).expect("測試資料必須能解析");
    analyze::check_pins(&model, &catalog())
        .iter()
        .map(|d| d.code)
        .collect()
}

/// leaf1 的合法基本綁定。
pub const L1_OK: &str = "i1: plc1.L1_in, o1: plc1.L1_out, error_e: plc1.L1_err";
```

`machine-model/tests/analyze_pins.rs`：

```rust
mod common;

use common::{L1_OK, pin_codes};

#[test]
fn a_fully_bound_tree_has_no_pin_diagnostics() {
    assert_eq!(pin_codes(L1_OK, &[]), Vec::<&str>::new());
}

#[test]
fn u6_required_pins_must_be_bound() {
    // 少了必接的 field 腳 o1
    assert_eq!(pin_codes("i1: plc1.L1_in, error_e: plc1.L1_err", &[]), ["U6"]);
    // 少了 error 腳；或明寫 null 也不行
    assert_eq!(pin_codes("i1: plc1.L1_in, o1: plc1.L1_out", &[]), ["U6"]);
    assert_eq!(
        pin_codes("i1: plc1.L1_in, o1: plc1.L1_out, error_e: null", &[]),
        ["U6"]
    );
    // field 腳明寫 null 視為刻意不接；required: false 的 i2 可以省略
    assert_eq!(
        pin_codes("i1: null, o1: plc1.L1_out, error_e: plc1.L1_err", &[]),
        Vec::<&str>::new()
    );
}

#[test]
fn u4_field_pins_check_existence_direction_and_type() {
    // Tag 不存在
    assert_eq!(pin_codes("i1: plc1.NOPE, o1: plc1.L1_out, error_e: plc1.L1_err", &[]), ["U4"]);
    // field.in 綁到輸出位址（Y）
    assert_eq!(pin_codes("i1: plc1.L1_out, o1: plc1.L1_in, error_e: plc1.L1_err", &[]), ["U4", "U4"]);
    // field.in 綁到非輸入類別（M）
    assert_eq!(pin_codes("i1: plc1.L1_err, o1: plc1.L1_out, error_e: plc1.L1_in", &[]), ["U4"]);
    // field.out 綁到非實體輸出類別（M）
    assert_eq!(pin_codes("i1: plc1.L1_in, o1: plc1.L1_err, error_e: plc1.L2_err", &[]), ["U4"]);
    // 型別不符：int16 的 Tag 綁 bool 腳
    assert_eq!(
        pin_codes(
            "i1: plc1.W, o1: plc1.L1_out, error_e: plc1.L1_err",
            &["{id: plc1.W, address: D1, type: int16}"]
        )
        .into_iter()
        .filter(|c| *c == "U4")
        .count(),
        2 // 型別不符，且 D 不是輸入位址
    );
    // field 腳綁 Unit id
    assert_eq!(pin_codes("i1: leaf2, o1: plc1.L1_out, error_e: plc1.L1_err", &[]), ["U4"]);
}

#[test]
fn u10_ctrl_pins_check_existence_and_type() {
    let with = |pin: &str, tags: &[&str]| pin_codes(&format!("{L1_OK}, {pin}"), tags);
    assert_eq!(with("man_go: plc1.NOPE", &[]), ["U10"]);
    assert_eq!(
        with("man_go: plc1.W", &["{id: plc1.W, address: D1, type: int16}"]),
        ["U10"]
    );
    assert_eq!(with("man_go: leaf2", &[]), ["U10"]);
    // 合法：bool 腳綁 M 位元；null 是刻意不接
    assert_eq!(
        with("man_go: plc1.G", &["{id: plc1.G, address: M300, type: bool}"]),
        Vec::<&str>::new()
    );
    assert_eq!(with("man_go: null", &[]), Vec::<&str>::new());
    // par_t 是 int16；綁 uint16 要被擋（型別必須完全相同）
    assert_eq!(
        with("par_t: plc1.P", &["{id: plc1.P, address: D1, type: uint16}"]),
        ["U10"]
    );
    assert_eq!(
        with("par_t: plc1.P", &["{id: plc1.P, address: D1, type: int16}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn u11_multi_binding_restrictions() {
    // 寫入候選（man_）與唯讀狀態（at_）混綁同一 Tag
    let mixed_rw = pin_codes(
        &format!("{L1_OK}, man_go: plc1.G, at_ok: plc1.G"),
        &["{id: plc1.G, address: M300, type: bool}"],
    );
    assert_eq!(mixed_rw, ["U11"]);

    // 跨單元：leaf1 的 man_go 綁到 leaf2 已用的 L2_err（alarm 唯讀腳）
    let cross = pin_codes(&format!("{L1_OK}, man_go: plc1.L2_err"), &[]);
    assert_eq!(cross, ["U11"]);

    // field 腳與 ctrl 腳混用同一個 Tag
    let mixed_field_ctrl = pin_codes(&format!("{L1_OK}, at_ok: plc1.L1_in"), &[]);
    assert_eq!(mixed_field_ctrl, ["U11"]);

    // 實體輸出 field.out 腳被兩個單元共用（搶同一個 Y 點）
    let shared_out = pin_codes("i1: plc1.L1_in, o1: plc1.L2_out, error_e: plc1.L1_err", &[]);
    assert_eq!(shared_out, ["U11"]);
}

#[test]
fn u11_allows_shared_inputs_parameters_and_operations() {
    // 兩個單元共用 field.in（感測輸入，如 X）
    let shared_in = pin_codes("i1: plc1.L2_in, o1: plc1.L1_out, error_e: plc1.L1_err", &[]);
    assert_eq!(shared_in, Vec::<&str>::new());

    // 兩個單元共用 par_（參數，如 D）
    let shared_par = pin_codes(
        &format!("{L1_OK}, par_t: plc1.P"),
        &["{id: plc1.P, address: D1, type: int16}"],
    );
    assert_eq!(shared_par, Vec::<&str>::new());

    // 兩個單元共用 man_（同動操作按鈕，如 M）
    let shared_man = pin_codes(
        &format!("{L1_OK}, man_go: plc1.G"),
        &["{id: plc1.G, address: M300, type: bool}"],
    );
    assert_eq!(shared_man, Vec::<&str>::new());
}

#[test]
fn u11_does_not_duplicate_errors_for_non_existent_tags() {
    // 幽靈 Tag 只報 U4/U10 不存在，不應再報 U11
    let ghost = pin_codes("i1: plc1.NOPE, o1: plc1.L1_out, error_e: plc1.L1_err, man_go: plc1.NOPE", &[]);
    assert!(!ghost.contains(&"U11"), "{ghost:?}");
}

#[test]
fn u13_operation_pins_must_not_bind_read_only_or_safety_monitor_tags() {
    let with = |pin: &str, tags: &[&str]| pin_codes(&format!("{L1_OK}, {pin}"), tags);
    // man_ 綁 X 類別
    assert_eq!(
        with("man_go: plc1.XX", &["{id: plc1.XX, address: X30, type: bool}"]),
        ["U13"]
    );
    // man_ 綁 safety_monitor
    assert_eq!(
        with(
            "man_go: plc1.S",
            &["{id: plc1.S, address: M300, type: bool, safety_monitor: true}"]
        ),
        ["U13"]
    );
    // 兩個條件都成立：各報一次
    assert_eq!(
        with(
            "man_go: plc1.XS",
            &["{id: plc1.XS, address: X31, type: bool, safety_monitor: true}"]
        ),
        ["U13", "U13"]
    );
    // par_ 同樣受限（X 是位元類別，型別也不符，所以同時有 U10）
    let par = with("par_t: plc1.XP", &["{id: plc1.XP, address: X32, type: int16}"]);
    assert!(par.contains(&"U13"), "{par:?}");
    // 非候選腳（at_）綁 X 或 safety_monitor 都可以
    assert_eq!(
        with(
            "at_ok: plc1.AX",
            &["{id: plc1.AX, address: X33, type: bool, safety_monitor: true}"]
        ),
        Vec::<&str>::new()
    );
}

#[test]
fn u14_explicit_role_that_disagrees_with_the_pin_is_a_warning() {
    let with = |tag: &str, pin: &str| pin_codes(&format!("{L1_OK}, {pin}"), &[tag]);
    // pv_temp 的前綴 role 是 process_value；Tag 明寫 command → 提示
    assert_eq!(
        with(
            "{id: plc1.T, address: D1, type: float32, role: command}",
            "pv_temp: plc1.T"
        ),
        ["U14"]
    );
    // 一致就不提示
    assert_eq!(
        with(
            "{id: plc1.T, address: D1, type: float32, role: process_value}",
            "pv_temp: plc1.T"
        ),
        Vec::<&str>::new()
    );
}

#[test]
fn u14_is_a_warning_not_an_error() {
    use common::{BASE_TAGS, L1_OK, catalog, doc, tags};
    let units = format!(
        "units:\n  - {{id: machine, type: root, type_version: 1, bind: {{m: mid1}}}}\n  - {{id: mid1, type: mid, type_version: 1, bind: {{a: leaf1, b: leaf2}}}}\n  - {{id: leaf1, type: leaf, type_version: 1, bind: {{{L1_OK}, pv_temp: plc1.T}}}}\n  - {{id: leaf2, type: leaf, type_version: 1, bind: {{i1: plc1.L2_in, o1: plc1.L2_out, error_e: plc1.L2_err}}}}\n"
    );
    let mut entries: Vec<&str> = BASE_TAGS.to_vec();
    entries.push("{id: plc1.T, address: D1, type: float32, role: command}");
    let model = machine_model::parse(&doc(&format!("{units}{}", tags(&entries)))).unwrap();
    let diags = machine_model::analyze::check_pins(&model, &catalog());
    assert_eq!(diags.len(), 1);
    assert!(!diags[0].is_error());
}
```

- [ ] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test analyze_pins`
Expected: FAIL（`check_pins` 目前是空殼，多數測試拿到空診斷）。

- [x] **Step 3: 寫實作（取代 `analyze/pins.rs`）**

```rust
//! 接腳綁定規則：U4、U6、U10、U11、U13、U14。

use std::collections::BTreeMap;

use crate::address;
use crate::catalog::UnitCatalog;
use crate::diag::Diagnostic;
use crate::family::ClassProfile;
use crate::naming::{BindPolicy, Prefix};
use crate::schema::{BindRef, MachineModel, Plc, Tag, classify_bind, split_id};
use crate::types::DataType;
use crate::unit_type::PinRef;

struct BoundPin<'a> {
    loc: String,
    is_field: bool,
    is_field_out: bool,
    prefix: Option<Prefix>,
    write_candidate: bool,
    ty: &'a str,
}

pub(super) fn check(m: &MachineModel, catalog: &UnitCatalog, out: &mut Vec<Diagnostic>) {
    let tags: BTreeMap<&str, &Tag> = m.tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let plcs: BTreeMap<&str, &Plc> = m.machine.plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    // 存在的 Tag id → 綁定它的接腳資訊
    let mut users: BTreeMap<&str, Vec<BoundPin<'_>>> = BTreeMap::new();

    for u in &m.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue; // U2 已回報
        };
        let uloc = format!("units[{}]", u.id);

        // U6：必接的 field 腳、必綁的 error 腳
        for (name, pin) in ty.field.input.0.iter().chain(ty.field.output.0.iter()) {
            if pin.required != Some(false) && !u.bind.0.contains_key(name) {
                out.push(Diagnostic::error(
                    "U6",
                    &uloc,
                    format!("field 腳 `{name}` 必須綁定（刻意不接請明確寫 null）"),
                ));
            }
        }
        for name in ty.ctrl.0.keys() {
            let Some((prefix, _)) = crate::naming::split(name) else {
                continue;
            };
            if prefix.info().bind == BindPolicy::Required
                && !matches!(u.bind.get(name), Some(Some(_)))
            {
                out.push(Diagnostic::error(
                    "U6",
                    &uloc,
                    format!("`{name}` 是 error_ 接腳，必須綁定 Tag（不得省略或寫 null）"),
                ));
            }
        }

        for (key, value) in &u.bind.0 {
            let Some(pin) = ty.lookup(key) else {
                continue; // U3 已回報
            };
            let Some(value) = value else {
                continue; // null：刻意不接
            };
            let (code, kind, pin_type): (&'static str, &str, Option<&str>) = match pin {
                PinRef::FieldIn(p) | PinRef::FieldOut(p) => ("U4", "field", Some(p.ty.as_str())),
                PinRef::Ctrl(prefix, p) => ("U10", "ctrl", p.type_text(prefix)),
                PinRef::Part(_) => continue, // 由圖規則處理
            };
            let tag_id = match classify_bind(value) {
                Some(BindRef::Tag(id)) => id,
                _ => {
                    out.push(Diagnostic::error(
                        code,
                        &uloc,
                        format!("{kind} 腳 `{key}` 必須綁 Tag（plc<N>.<Name>），而不是 `{value}`"),
                    ));
                    continue;
                }
            };
            let Some(tag) = tags.get(tag_id) else {
                out.push(Diagnostic::error(
                    code,
                    &uloc,
                    format!("{kind} 腳 `{key}` 綁定的 tag `{tag_id}` 不存在"),
                ));
                continue;
            };

            let (is_field, is_field_out, prefix, write_candidate) = match pin {
                PinRef::FieldIn(_) => (true, false, None, false),
                PinRef::FieldOut(_) => (true, true, None, false),
                PinRef::Ctrl(p, _) => (false, false, Some(p), p.info().write_candidate),
                PinRef::Part(_) => unreachable!(),
            };
            users.entry(tag_id).or_default().push(BoundPin {
                loc: format!("{}.{key}", u.id),
                is_field,
                is_field_out,
                prefix,
                write_candidate,
                ty: pin_type.unwrap_or_default(),
            });

            if let (Some(want), Ok(have)) = (pin_type, DataType::parse(&tag.ty)) {
                if DataType::parse(want).is_ok_and(|w| w != have) {
                    out.push(Diagnostic::error(
                        code,
                        &uloc,
                        format!(
                            "{kind} 腳 `{key}` 的型別是 `{want}`，但 tag `{tag_id}` 的型別是 `{}`",
                            tag.ty
                        ),
                    ));
                }
            }

            let class = class_of(tag, &plcs);
            match pin {
                PinRef::FieldIn(_) => {
                    if class.is_some_and(|c| !c.read_only) {
                        out.push(Diagnostic::error(
                            code,
                            &uloc,
                            format!("field.in 腳 `{key}` 必須綁輸入位址類別（唯讀輸入，如 X），tag `{tag_id}` 不是"),
                        ));
                    }
                }
                PinRef::FieldOut(_) => {
                    if class.is_some_and(|c| c.name != "Y") {
                        out.push(Diagnostic::error(
                            code,
                            &uloc,
                            format!(
                                "field.out 腳 `{key}` 必須綁實體輸出位址類別（如 Y），tag `{tag_id}` 位於 `{}` 類別",
                                class.map_or("未知", |c| c.name)
                            ),
                        ));
                    }
                }
                PinRef::Ctrl(prefix, p) => {
                    if prefix.info().write_candidate {
                        if class.is_some_and(|c| c.read_only) {
                            out.push(Diagnostic::error(
                                "U13",
                                &uloc,
                                format!("`{key}` 是操作／參數腳，不得綁唯讀位址類別的 tag `{tag_id}`"),
                            ));
                        }
                        if tag.safety_monitor == Some(true) {
                            out.push(Diagnostic::error(
                                "U13",
                                &uloc,
                                format!("`{key}` 是操作／參數腳，不得綁 safety_monitor 的 tag `{tag_id}`"),
                            ));
                        }
                    }
                    let effective = p.role.unwrap_or(prefix.info().role);
                    if tag.role.is_some_and(|r| r != effective) {
                        out.push(Diagnostic::warning(
                            "U14",
                            format!("tags[{tag_id}]"),
                            format!(
                                "明確的 role 與接腳 `{}.{key}` 推測的 {effective:?} 不同；以明確值為準",
                                u.id
                            ),
                        ));
                    }
                }
                PinRef::Part(_) => {}
            }
        }
    }

    for (tag_id, us) in &users {
        if us.len() <= 1 {
            continue;
        }
        let who = us.iter().map(|u| u.loc.as_str()).collect::<Vec<_>>().join("、");

        // 1. field.out 必須全機唯一
        if us.iter().filter(|u| u.is_field_out).count() > 1 {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("實體輸出 field.out 腳不得由多個單元共用同一 Tag（{who}）"),
            ));
        }

        // 2. 禁止 field 與 ctrl 混用
        let has_field = us.iter().any(|u| u.is_field);
        let has_ctrl = us.iter().any(|u| !u.is_field);
        if has_field && has_ctrl {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 不得同時被 field 腳與 ctrl 腳綁定（{who}）"),
            ));
        }

        // 3. 禁止寫入候選（man_、par_）與唯讀狀態混用
        let has_wc = us.iter().any(|u| u.write_candidate);
        let has_ro = us.iter().any(|u| !u.write_candidate && !u.is_field);
        if has_wc && has_ro {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 不得同時被寫入候選腳與唯讀狀態腳綁定（{who}）"),
            ));
        }

        // 4. 接腳型別宣告必須完全一致
        let first_ty = us[0].ty;
        if us.iter().any(|u| u.ty != first_ty) {
            out.push(Diagnostic::error(
                "U11",
                format!("tags[{tag_id}]"),
                format!("同一個 Tag 被型別不同的接腳綁定（{who}）"),
            ));
        }
    }
}

fn class_of(tag: &Tag, plcs: &BTreeMap<&str, &Plc>) -> Option<&'static ClassProfile> {
    let (plc_id, _) = split_id(&tag.id)?;
    let plc = plcs.get(plc_id)?;
    address::parse(plc.family, &tag.address).ok().map(|a| a.class)
}
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。逐一核對有可能與預期不同之處並回報（不要改規則去遷就測試）：
- `u4_...` 的「field.in 綁輸出位址」預期 `["U4","U4"]`：`i1` 綁 Y（非唯讀 → 錯）、`o1` 綁 X（唯讀 → 錯）。
- 「型別不符」案例預期 U4 恰好 2 次：`i1` 綁 `D1 int16`（型別不符＋非輸入類別）。
- `u11_...` 的 `mixed`：`at_ok` 綁 `L1_in`，而 `L1_in` 已被 `i1` 使用。
- `u13_...` 的 `par_t: plc1.XP`（X32 int16）：X 是位元類別，位址規則 R19 在這裡不跑，所以只會有 `U10`（型別 int16 對 par_t 的 int16 其實相同，不會有 U10）與 `U13`；測試只斷言包含 `U13`。

- [x] **Step 5: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨。

---

### Task 7: 推導與解析結果資料結構

**Files:**
- Create: `machine-model/src/resolve.rs`, `machine-model/tests/fixtures/valid_units.yaml`, `machine-model/tests/resolve.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod resolve;`）

**Interfaces:**
- Consumes: `schema::*`、`catalog::UnitCatalog`、`unit_type::{PinRef, Kind, Level, Reset}`、`naming`、`hash::{content_hash, hex}`、`address::parse`、`family::Family`。
- Produces（規格 §9.2；皆 `Debug, Clone, PartialEq`）:
  - `resolve::resolve(model: &MachineModel, catalog: &UnitCatalog, source_text: &str) -> ResolvedModel`（先決條件：已通過載入安全規則與 `analyze::check`；不 panic）
  - `ResolvedModel { machine_id: String, plcs: Vec<ResolvedPlc>, units: Vec<ResolvedUnit>, tags: Vec<ResolvedTag>, variables: Vec<Variable>, pins: Vec<ResolvedPin>, fingerprint: Fingerprint }`
  - `ResolvedPlc { id: String, family: Family, online: bool }`
  - `ResolvedUnit { id, ty, type_version: u32, parent: Option<String>, path: String, bind: BTreeMap<String, Option<String>> }`
  - `ResolvedTag { id, address, ty, symbol: Option<String>, label: Option<String>, description: Option<String>, scan: Scan, scale, offset, deadband: Option<f64>, unit: Option<String>, readback: Option<bool>, retain: Option<bool>, safety_monitor: bool, spare: bool, polarity: Option<Polarity>, ext: Ext, min: Option<f64>, max: Option<f64>, default: Option<Scalar>, role: Option<Role>, role_source: RoleSource, write_candidate: bool, bindings: Vec<TagBinding>, class_read_only: bool }`；輔助方法 `ResolvedTag::binding(&self) -> Option<&TagBinding>` 回傳第一個綁定
  - `RoleSource { Explicit, Pin(Prefix), Unknown }`
  - `TagBinding { unit_id: String, unit_path: String, pin: String, prefix: Option<Prefix> }`（`prefix` 為 `None` 表示 field 腳）
  - `ResolvedPin { unit_id, unit_path, pin, prefix: Prefix, function: String, label: String, ty: String, tag: Option<String>, kind: Option<Kind>, level: Option<Level>, reset: Option<Reset>, message: Option<String>, unit: Option<String>, min, max: Option<f64>, default: Option<Scalar> }`
  - `Fingerprint { source_hash: String, type_versions: BTreeMap<String, u32> }`
  - `ResolvedModel` 方法：`tag(&self, id) -> Option<&ResolvedTag>`、`operations()`（`man`）、`alarms()`（`error`）、`parameters()`（`par`）、`values()`（`pv`、`count`）、`messages()`（`msg`）皆回 `impl Iterator<Item = &ResolvedPin>`；`coverage_gaps(&self) -> Vec<&ResolvedPin>`（`BindPolicy::Optional` 且 `tag` 為 `None` 的接腳；`cmd_`／`ena_` 純接線不算）

推導規則（實作者必須照做）：
- `role`：Tag 明確 `role` ＞ 綁定的 ctrl 腳（腳的 `role`，否則前綴推測；若多重綁定取第一個）＞ 無（`Unknown`；field 腳綁定的 Tag 也是 `Unknown`）。
- `write_candidate`：若綁定的接腳中有任何一個為 `man_`／`par_` ctrl 腳才是 `true`；沒綁、只綁 field 腳、只綁其他 ctrl 腳都是 `false`。
- `min`／`max`／`default`／`unit`：Tag 自己寫了就用 Tag 的；否則取所綁 ctrl 腳的對應屬性（多重綁定取第一個有設定者）。
- `class_read_only`：位址無法解析時為 `false`。
- 標籤：腳的 `label`，否則 `naming::default_label(功能)`，再否則功能名本身。`kind`：`man_` 預設 `Momentary`；`error_` 的 `level` 預設 `Abnormal`、`reset` 預設 `Manual`；其他前綴這三個欄位保持腳上的值。
- `ResolvedUnit.path`：由 `machine` 起以 `/` 連接的單元 id（例：`machine/st1/cyl1`）；若沒有父層則只有自己的 id。沿父鏈往上最多走 `units.len()` 步以防環。
- 同一 Tag 被多個接腳綁定時（U11 允許共用感測輸入、共用參數、共用同動操作），全部記錄在 `bindings` 向量中。
- `ResolvedTag` 保留 `spare`、`polarity`、`ext`，供下游（PlcDataService 與匯出）原樣轉發而不遺失資訊；`ResolvedPlc` 則合理裁剪掉靜態設定欄位（`title`、`diff_scope`、`sources`）。

- [x] **Step 1: 建立 fixture**

> [!NOTE]
> `tests/fixtures/structural_full.yaml`（Plan 1 建立）只供載入安全規則（R 系列）測試使用，因其簡化綁定不保證過 U 規則（例如 machine.bind.stn 宣告 station_load 卻接了 cylinder_double）；實例層規則與解析測試請使用此處建立的 `valid_units.yaml`。

`machine-model/tests/fixtures/valid_units.yaml`（使用標準類型；每個單元一行，測試會用字串替換製造變體）：

```yaml
schema_version: 2
machine:
  id: demo
  plcs:
    - { id: plc1, family: inovance_easy }
units:
  - { id: machine, type: machine_ctrl, type_version: 1, bind: { stn: st1, error_estop: plc1.M_Estop, man_mode: plc1.D_ModeCmd, at_mode: plc1.D_ModeAct } }
  - { id: st1, type: station_load, type_version: 1, bind: { cyl: cyl1 } }
  - { id: cyl1, type: cylinder_double, type_version: 1, bind: { lim_ext: plc1.X_Cyl1_Ext_LS, lim_ret: plc1.X_Cyl1_Ret_LS, valve_ext: plc1.Y_Cyl1_Ext, valve_ret: plc1.Y_Cyl1_Ret, man_ext: plc1.M_Cyl1_ManExt, man_ret: plc1.M_Cyl1_ManRet, error_ext: plc1.M_Cyl1_ErrExt, error_ret: plc1.M_Cyl1_ErrRet, par_ext_time: plc1.D_Cyl1_ExtTime } }
tags:
  - { id: plc1.X_Cyl1_Ext_LS, address: X10, type: bool }
  - { id: plc1.X_Cyl1_Ret_LS, address: X11, type: bool }
  - { id: plc1.Y_Cyl1_Ext, address: Y10, type: bool }
  - { id: plc1.Y_Cyl1_Ret, address: Y11, type: bool }
  - { id: plc1.M_Cyl1_ManExt, address: M100, type: bool }
  - { id: plc1.M_Cyl1_ManRet, address: M101, type: bool }
  - { id: plc1.M_Cyl1_ErrExt, address: M110, type: bool }
  - { id: plc1.M_Cyl1_ErrRet, address: M111, type: bool }
  - { id: plc1.D_Cyl1_ExtTime, address: D100, type: int16 }
  - { id: plc1.M_Estop, address: M120, type: bool, safety_monitor: true }
  - { id: plc1.D_ModeCmd, address: D200, type: int16 }
  - { id: plc1.D_ModeAct, address: D201, type: int16 }
  - { id: plc1.M_Spare, address: M130, type: bool }
```

- [x] **Step 2: 寫失敗的測試**

`machine-model/tests/resolve.rs`：

```rust
use machine_model::catalog::UnitCatalog;
use machine_model::naming::Prefix;
use machine_model::resolve::{ResolvedModel, RoleSource, resolve};
use machine_model::schema::Role;
use machine_model::unit_type::{Kind, Level, Reset};

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn resolved() -> ResolvedModel {
    let model = machine_model::load(FIXTURE).expect("fixture 必須通過載入安全規則");
    let catalog = UnitCatalog::standard();
    let diags = machine_model::analyze::check(&model, &catalog);
    assert!(diags.is_empty(), "{diags:#?}");
    resolve(&model, &catalog, FIXTURE)
}

#[test]
fn write_candidate_is_true_only_for_man_and_par_bindings() {
    let r = resolved();
    let wc = |id: &str| r.tag(id).unwrap().write_candidate;
    for id in [
        "plc1.M_Cyl1_ManExt",
        "plc1.M_Cyl1_ManRet",
        "plc1.D_Cyl1_ExtTime", // par_ext_time
        "plc1.D_ModeCmd",      // man_mode
    ] {
        assert!(wc(id), "{id}");
    }
    for id in [
        "plc1.X_Cyl1_Ext_LS", // field 腳
        "plc1.Y_Cyl1_Ext",    // field 腳
        "plc1.M_Cyl1_ErrExt", // error_
        "plc1.M_Estop",
        "plc1.D_ModeAct", // at_mode
        "plc1.M_Spare",   // 沒被任何 Unit 綁定：fail-closed
    ] {
        assert!(!wc(id), "{id}");
    }
}

#[test]
fn role_comes_from_the_pin_prefix_unless_explicit() {
    let r = resolved();
    let t = r.tag("plc1.M_Cyl1_ManExt").unwrap();
    assert_eq!(t.role, Some(Role::Command));
    assert_eq!(t.role_source, RoleSource::Pin(Prefix::Man));
    assert_eq!(r.tag("plc1.M_Cyl1_ErrExt").unwrap().role, Some(Role::Alarm));
    assert_eq!(r.tag("plc1.D_Cyl1_ExtTime").unwrap().role, Some(Role::Setpoint));
    // field 腳與未綁定的 Tag 沒有 role
    for id in ["plc1.X_Cyl1_Ext_LS", "plc1.M_Spare"] {
        let t = r.tag(id).unwrap();
        assert_eq!((t.role, t.role_source), (None, RoleSource::Unknown), "{id}");
    }
}

#[test]
fn explicit_role_wins_and_is_reported_as_explicit() {
    let text = FIXTURE.replace(
        "{ id: plc1.M_Spare, address: M130, type: bool }",
        "{ id: plc1.M_Spare, address: M130, type: bool, role: status }",
    );
    let model = machine_model::load(&text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), &text);
    let t = r.tag("plc1.M_Spare").unwrap();
    assert_eq!((t.role, t.role_source), (Some(Role::Status), RoleSource::Explicit));
    assert!(!t.write_candidate);
}

#[test]
fn par_pin_supplies_range_default_and_unit_when_the_tag_has_none() {
    let r = resolved();
    let t = r.tag("plc1.D_Cyl1_ExtTime").unwrap();
    assert_eq!((t.min, t.max), (Some(0.0), Some(30000.0)));
    assert_eq!(t.default, Some(machine_model::schema::Scalar::Int(3000)));
    assert_eq!(t.unit.as_deref(), Some("ms"));
}

#[test]
fn tag_values_beat_pin_values() {
    let text = FIXTURE.replace(
        "{ id: plc1.D_Cyl1_ExtTime, address: D100, type: int16 }",
        "{ id: plc1.D_Cyl1_ExtTime, address: D100, type: int16, min: 100, max: 9000, default: 500, unit: s }",
    );
    let model = machine_model::load(&text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), &text);
    let t = r.tag("plc1.D_Cyl1_ExtTime").unwrap();
    assert_eq!((t.min, t.max), (Some(100.0), Some(9000.0)));
    assert_eq!(t.unit.as_deref(), Some("s"));
}

#[test]
fn class_and_binding_details() {
    let r = resolved();
    assert!(r.tag("plc1.X_Cyl1_Ext_LS").unwrap().class_read_only);
    assert!(!r.tag("plc1.M_Cyl1_ManExt").unwrap().class_read_only);
    assert!(r.tag("plc1.M_Estop").unwrap().safety_monitor);
    let b = r.tag("plc1.M_Cyl1_ManExt").unwrap().binding().unwrap();
    assert_eq!(
        (b.unit_id.as_str(), b.unit_path.as_str(), b.pin.as_str(), b.prefix),
        ("cyl1", "machine/st1/cyl1", "man_ext", Some(Prefix::Man))
    );
    assert_eq!(r.tag("plc1.M_Cyl1_ManExt").unwrap().bindings.len(), 1);
    let f = r.tag("plc1.Y_Cyl1_Ext").unwrap().binding().unwrap();
    assert_eq!(f.prefix, None);
    assert!(r.tag("plc1.M_Spare").unwrap().bindings.is_empty());
    assert!(r.tag("plc1.M_Spare").unwrap().binding().is_none());
    assert!(!r.tag("plc1.M_Spare").unwrap().spare);
    assert_eq!(r.tag("plc1.M_Spare").unwrap().polarity, None);
    assert!(r.tag("plc1.M_Spare").unwrap().ext.is_empty());
}

#[test]
fn unit_tree_and_paths() {
    let r = resolved();
    let u = |id: &str| r.units.iter().find(|u| u.id == id).unwrap();
    assert_eq!(u("machine").parent, None);
    assert_eq!(u("machine").path, "machine");
    assert_eq!(u("cyl1").parent.as_deref(), Some("st1"));
    assert_eq!(u("cyl1").path, "machine/st1/cyl1");
    assert_eq!(u("cyl1").type_version, 1);
}

#[test]
fn pin_lists_carry_defaults_and_labels() {
    let r = resolved();
    let op = r
        .operations()
        .find(|p| p.unit_id == "cyl1" && p.pin == "man_ext")
        .unwrap();
    assert_eq!(op.label, "伸出");
    assert_eq!(op.kind, Some(Kind::Momentary));
    assert_eq!(op.tag.as_deref(), Some("plc1.M_Cyl1_ManExt"));

    // 順序：依 units[] 的宣告順序（machine、st1、cyl1），單元內依接腳名字母序
    let alarms: Vec<_> = r.alarms().map(|p| (p.unit_id.as_str(), p.pin.as_str())).collect();
    assert_eq!(alarms, [("machine", "error_estop"), ("cyl1", "error_ext"), ("cyl1", "error_ret")]);
    let ext = r.alarms().find(|p| p.pin == "error_ext").unwrap();
    assert_eq!((ext.level, ext.reset), (Some(Level::Abnormal), Some(Reset::Manual)));
    assert_eq!(ext.message.as_deref(), Some("伸出逾時"));
    let estop = r.alarms().find(|p| p.pin == "error_estop").unwrap();
    assert_eq!(estop.level, Some(Level::Emergency));

    assert_eq!(
        r.parameters().map(|p| p.pin.as_str()).collect::<Vec<_>>(),
        ["par_cycle_time", "par_err_time", "par_ext_time"]
    );
    assert!(r.values().any(|p| p.pin == "count_done"));
    assert_eq!(r.messages().count(), 0);
}

#[test]
fn label_falls_back_to_the_function_name() {
    // station_load 的 at_running 沒有 label；詞典有 running → 運轉
    let r = resolved();
    let p = r.pins.iter().find(|p| p.unit_id == "st1" && p.pin == "at_running").unwrap();
    assert_eq!(p.label, "運轉");
    let p = r.pins.iter().find(|p| p.unit_id == "st1" && p.pin == "count_done").unwrap();
    assert_eq!(p.label, "done");
}

#[test]
fn coverage_gaps_list_unbound_optional_pins_but_not_wiring_pins() {
    let r = resolved();
    let gaps: Vec<_> = r
        .coverage_gaps()
        .iter()
        .map(|p| (p.unit_id.clone(), p.pin.clone()))
        .collect();
    assert!(gaps.contains(&("cyl1".to_owned(), "par_err_time".to_owned())));
    assert!(gaps.contains(&("st1".to_owned(), "man_start".to_owned())));
    // 已綁定的不在清單內；cmd_／ena_ 純接線不算
    assert!(!gaps.contains(&("cyl1".to_owned(), "man_ext".to_owned())));
    assert!(!gaps.iter().any(|(_, pin)| pin.starts_with("cmd_") || pin.starts_with("ena_")));
}

#[test]
fn fingerprint_pins_source_and_type_versions() {
    let r = resolved();
    assert_eq!(r.fingerprint.source_hash.len(), 16);
    let crlf = FIXTURE.replace('\n', "\r\n");
    let model = machine_model::load(&crlf).unwrap();
    let r2 = resolve(&model, &UnitCatalog::standard(), &crlf);
    assert_eq!(r.fingerprint.source_hash, r2.fingerprint.source_hash);
    let v: Vec<_> = r.fingerprint.type_versions.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    assert_eq!(v, [("cylinder_double", 1), ("machine_ctrl", 1), ("station_load", 1)]);
}

#[test]
fn resolve_does_not_panic_on_an_unvalidated_model() {
    // 先決條件被違反（未知類型、不存在的 Tag）時也不得 panic
    let text = "schema_version: 2\nmachine: {id: m, plcs: [{id: plc1, family: inovance_easy}]}\n\
units:\n  - {id: machine, type: ghost, type_version: 9, bind: {x: plc1.NOPE, y: other}}\n";
    let model = machine_model::parse(text).unwrap();
    let r = resolve(&model, &UnitCatalog::standard(), text);
    assert!(r.tags.is_empty());
}
```

- [ ] **Step 3: 確認測試失敗**

Run: `cargo test -p machine-model --test resolve`
Expected: FAIL（編譯錯誤：`machine_model::resolve` 不存在）。

- [x] **Step 4: 寫實作 `resolve.rs`**

```rust
//! 推導與解析結果（規格 §6、§9.2）：把 `machine.yaml` 與類型目錄變成下游直接使用的資料。
//!
//! 這裡只輸出模型層的事實（`role`、`write_candidate`…）。最終 `writable` 與人員權限
//! 不在本專案（規格 §6.3）。

use std::collections::BTreeMap;

use crate::address;
use crate::catalog::UnitCatalog;
use crate::family::Family;
use crate::hash::{content_hash, hex};
use crate::naming::{self, BindPolicy, Prefix};
use crate::schema::{
    BindRef, MachineModel, Polarity, Role, Scalar, Scan, Tag, Variable, classify_bind, split_id,
};
use crate::strict::Ext;
use crate::unit_type::{CtrlPin, Kind, Level, PinRef, Reset};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModel {
    pub machine_id: String,
    pub plcs: Vec<ResolvedPlc>,
    pub units: Vec<ResolvedUnit>,
    pub tags: Vec<ResolvedTag>,
    pub variables: Vec<Variable>,
    pub pins: Vec<ResolvedPin>,
    pub fingerprint: Fingerprint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPlc {
    pub id: String,
    pub family: Family,
    pub online: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUnit {
    pub id: String,
    pub ty: String,
    pub type_version: u32,
    pub parent: Option<String>,
    /// 由 `machine` 起以 `/` 連接，例如 `machine/st1/cyl1`。
    pub path: String,
    pub bind: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleSource {
    /// Tag 明確寫了 `role`。
    Explicit,
    /// 由綁定的 ctrl 腳的前綴推測。
    Pin(Prefix),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagBinding {
    pub unit_id: String,
    pub unit_path: String,
    pub pin: String,
    /// `None` 表示綁在 field 腳。
    pub prefix: Option<Prefix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTag {
    pub id: String,
    pub address: String,
    pub ty: String,
    pub symbol: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub scan: Scan,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub deadband: Option<f64>,
    pub unit: Option<String>,
    pub readback: Option<bool>,
    pub retain: Option<bool>,
    pub safety_monitor: bool,
    pub spare: bool,
    pub polarity: Option<Polarity>,
    pub ext: Ext,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub role: Option<Role>,
    pub role_source: RoleSource,
    /// 規格 §6.2：若綁定的接腳中有任何一個為 `man_`／`par_` 腳才是 `true`；沒綁定一律 `false`（fail-closed）。
    pub write_candidate: bool,
    /// 綁定此 Tag 的所有接腳清單（規格 §9.2）。
    pub bindings: Vec<TagBinding>,
    /// 位址類別是否為唯讀輸入；位址無法解析時為 `false`。
    pub class_read_only: bool,
}

impl ResolvedTag {
    /// 取得第一個綁定（若有）。
    pub fn binding(&self) -> Option<&TagBinding> {
        self.bindings.first()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPin {
    pub unit_id: String,
    pub unit_path: String,
    pub pin: String,
    pub prefix: Prefix,
    pub function: String,
    pub label: String,
    pub ty: String,
    pub tag: Option<String>,
    pub kind: Option<Kind>,
    pub level: Option<Level>,
    pub reset: Option<Reset>,
    pub message: Option<String>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    /// `machine.yaml` 的內容雜湊（忽略換行差異）。
    pub source_hash: String,
    /// 實際用到的類型 id → version。
    pub type_versions: BTreeMap<String, u32>,
}

impl ResolvedModel {
    pub fn tag(&self, id: &str) -> Option<&ResolvedTag> {
        self.tags.iter().find(|t| t.id == id)
    }

    fn with_prefix(&self, wanted: &'static [Prefix]) -> impl Iterator<Item = &ResolvedPin> {
        self.pins.iter().filter(move |p| wanted.contains(&p.prefix))
    }

    pub fn operations(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Man])
    }

    pub fn alarms(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Error])
    }

    pub fn parameters(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Par])
    }

    pub fn values(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Pv, Prefix::Count])
    }

    pub fn messages(&self) -> impl Iterator<Item = &ResolvedPin> {
        self.with_prefix(&[Prefix::Msg])
    }

    /// 規格 U16：沒綁定的選擇性 ctrl 腳（純接線的 `cmd_`／`ena_` 不算）。
    pub fn coverage_gaps(&self) -> Vec<&ResolvedPin> {
        self.pins
            .iter()
            .filter(|p| p.tag.is_none() && p.prefix.info().bind == BindPolicy::Optional)
            .collect()
    }
}

/// 一個 Tag 被哪個接腳綁定。
struct Binder<'a> {
    unit_id: &'a str,
    pin: &'a str,
    prefix: Option<Prefix>,
    ctrl: Option<&'a CtrlPin>,
}

pub fn resolve(model: &MachineModel, catalog: &UnitCatalog, source_text: &str) -> ResolvedModel {
    // 父層：part 綁的子單元 → 父單元 id（取第一個）
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    // Tag id → 綁定它的所有接腳清單
    let mut binders: BTreeMap<&str, Vec<Binder<'_>>> = BTreeMap::new();
    let mut type_versions: BTreeMap<String, u32> = BTreeMap::new();

    for u in &model.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue;
        };
        type_versions.insert(ty.id.clone(), ty.version);
        for (key, value) in &u.bind.0 {
            let (Some(pin), Some(value)) = (ty.lookup(key), value.as_deref()) else {
                continue;
            };
            match (pin, classify_bind(value)) {
                (PinRef::Part(_), Some(BindRef::Unit(child))) => {
                    parent.entry(child).or_insert(u.id.as_str());
                }
                (PinRef::FieldIn(_) | PinRef::FieldOut(_), Some(BindRef::Tag(id))) => {
                    binders.entry(id).or_default().push(Binder {
                        unit_id: &u.id,
                        pin: key,
                        prefix: None,
                        ctrl: None,
                    });
                }
                (PinRef::Ctrl(prefix, ctrl), Some(BindRef::Tag(id))) => {
                    binders.entry(id).or_default().push(Binder {
                        unit_id: &u.id,
                        pin: key,
                        prefix: Some(prefix),
                        ctrl: Some(ctrl),
                    });
                }
                _ => {}
            }
        }
    }

    let path_of = |id: &str| -> String {
        let mut parts = vec![id];
        let mut cur = id;
        for _ in 0..model.units.len() {
            let Some(&p) = parent.get(cur) else {
                break;
            };
            parts.push(p);
            cur = p;
        }
        parts.reverse();
        parts.join("/")
    };

    let units = model
        .units
        .iter()
        .map(|u| ResolvedUnit {
            id: u.id.clone(),
            ty: u.ty.clone(),
            type_version: u.type_version,
            parent: parent.get(u.id.as_str()).map(|p| (*p).to_owned()),
            path: path_of(&u.id),
            bind: u.bind.0.clone(),
        })
        .collect();

    let plcs: BTreeMap<&str, &crate::schema::Plc> =
        model.machine.plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    let tags = model
        .tags
        .iter()
        .map(|t| {
            let bs = binders.get(t.id.as_str()).map_or(&[] as &[_], Vec::as_slice);
            resolve_tag(t, bs, &plcs, &path_of)
        })
        .collect();

    ResolvedModel {
        machine_id: model.machine.id.clone(),
        plcs: model
            .machine
            .plcs
            .iter()
            .map(|p| ResolvedPlc {
                id: p.id.clone(),
                family: p.family,
                online: p.online,
            })
            .collect(),
        units,
        tags,
        variables: model.variables.clone(),
        pins: resolve_pins(model, catalog, &path_of),
        fingerprint: Fingerprint {
            source_hash: hex(content_hash(source_text)),
            type_versions,
        },
    }
}

fn resolve_tag(
    t: &Tag,
    tag_binders: &[Binder<'_>],
    plcs: &BTreeMap<&str, &crate::schema::Plc>,
    path_of: &dyn Fn(&str) -> String,
) -> ResolvedTag {
    let pin_role = tag_binders.iter().find_map(|b| {
        let prefix = b.prefix?;
        Some((b.ctrl.and_then(|c| c.role).unwrap_or(prefix.info().role), prefix))
    });
    let (role, role_source) = match (t.role, pin_role) {
        (Some(r), _) => (Some(r), RoleSource::Explicit),
        (None, Some((r, prefix))) => (Some(r), RoleSource::Pin(prefix)),
        (None, None) => (None, RoleSource::Unknown),
    };
    let write_candidate = tag_binders
        .iter()
        .any(|b| b.prefix.is_some_and(|p| p.info().write_candidate));
    let pin = tag_binders.iter().find_map(|b| b.ctrl);
    let class_read_only = split_id(&t.id)
        .and_then(|(plc, _)| plcs.get(plc))
        .and_then(|plc| address::parse(plc.family, &t.address).ok())
        .is_some_and(|a| a.class.read_only);

    let bindings = tag_binders
        .iter()
        .map(|b| TagBinding {
            unit_id: b.unit_id.to_owned(),
            unit_path: path_of(b.unit_id),
            pin: b.pin.to_owned(),
            prefix: b.prefix,
        })
        .collect();

    ResolvedTag {
        id: t.id.clone(),
        address: t.address.clone(),
        ty: t.ty.clone(),
        symbol: t.symbol().map(str::to_owned),
        label: t.label.clone(),
        description: t.description.clone(),
        scan: t.scan(),
        scale: t.scale,
        offset: t.offset,
        deadband: t.deadband,
        unit: t.unit.clone().or_else(|| pin.and_then(|p| p.unit.clone())),
        readback: t.readback,
        retain: t.retain,
        safety_monitor: t.safety_monitor.unwrap_or(false),
        spare: t.spare.unwrap_or(false),
        polarity: t.polarity,
        ext: t.ext.clone(),
        min: t.min.or_else(|| pin.and_then(|p| p.min)),
        max: t.max.or_else(|| pin.and_then(|p| p.max)),
        default: t.default.clone().or_else(|| pin.and_then(|p| p.default.clone())),
        role,
        role_source,
        write_candidate,
        bindings,
        class_read_only,
    }
}

fn resolve_pins(
    model: &MachineModel,
    catalog: &UnitCatalog,
    path_of: &dyn Fn(&str) -> String,
) -> Vec<ResolvedPin> {
    let mut out = Vec::new();
    for u in &model.units {
        let Some(ty) = catalog.get(&u.ty) else {
            continue;
        };
        for (name, pin) in &ty.ctrl.0 {
            let Some((prefix, rest)) = naming::split(name) else {
                continue;
            };
            let function = naming::function_of(rest);
            let tag = match u.bind.get(name) {
                Some(Some(v)) => match classify_bind(v) {
                    Some(BindRef::Tag(id)) => Some(id.to_owned()),
                    _ => None,
                },
                _ => None,
            };
            let is_error = prefix == Prefix::Error;
            out.push(ResolvedPin {
                unit_id: u.id.clone(),
                unit_path: path_of(&u.id),
                pin: name.clone(),
                prefix,
                function: function.to_owned(),
                label: pin
                    .label
                    .clone()
                    .or_else(|| naming::default_label(function).map(str::to_owned))
                    .unwrap_or_else(|| function.to_owned()),
                ty: pin.type_text(prefix).unwrap_or_default().to_owned(),
                tag,
                kind: pin.kind.or((prefix == Prefix::Man).then_some(Kind::Momentary)),
                level: pin.level.or(is_error.then_some(Level::Abnormal)),
                reset: pin.reset.or(is_error.then_some(Reset::Manual)),
                message: pin.message.clone(),
                unit: pin.unit.clone(),
                min: pin.min,
                max: pin.max,
                default: pin.default.clone(),
            });
        }
    }
    out
}
```

`lib.rs` 加 `pub mod resolve;`（`pub mod rules;` 之前）。

- [x] **Step 5: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。失敗時以實作邏輯為準回報，不要為了通過而扭曲規則。`pins` 的順序是確定的：依 `units[]` 宣告順序（`machine`、`st1`、`cyl1`），單元內依接腳名字母序；`path_of` 由根往下排列（`machine/st1/cyl1`）。

- [x] **Step 6: Checkpoint**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 乾淨。（`resolve_tag` 參數 `path_of: &dyn Fn(&str) -> String` 若 clippy 抱怨，改成泛型 `impl Fn(&str) -> String`。）

---

### Task 8: 單一入口 `load_checked` 與端到端測試

**Files:**
- Modify: `machine-model/src/lib.rs`
- Create: `machine-model/tests/e2e.rs`

**Interfaces:**
- Consumes: `load`、`analyze::check`、`resolve::resolve`、`catalog::UnitCatalog`。
- Produces:
  - `pub struct Checked { pub resolved: ResolvedModel, pub warnings: Vec<Diagnostic> }`
  - `pub fn load_checked(text: &str, catalog: &UnitCatalog) -> Result<Checked, Vec<Diagnostic>>`：先 `load`（載入安全規則；有錯就只回這些，不再往下跑 U 規則）；再 `analyze::check`；有 `Error` 回 `Err(全部診斷)`；否則 `Ok`，`warnings` 是 `Warning`／`Info` 診斷；最後 `resolve`

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/e2e.rs`：

```rust
use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;
use machine_model::load_checked;

const FIXTURE: &str = include_str!("fixtures/valid_units.yaml");

fn codes(text: &str) -> Vec<&'static str> {
    match load_checked(text, &UnitCatalog::standard()) {
        Ok(c) => c.warnings.iter().map(|d| d.code).collect(),
        Err(d) => d.iter().map(|x| x.code).collect(),
    }
}

#[test]
fn the_valid_fixture_loads_end_to_end() {
    let checked = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    assert!(checked.warnings.is_empty());
    assert_eq!(checked.resolved.machine_id, "demo");
    assert!(checked.resolved.tag("plc1.M_Cyl1_ManExt").unwrap().write_candidate);
}

#[test]
fn crlf_and_bom_load_the_same() {
    let text = format!("\u{feff}{}", FIXTURE.replace('\n', "\r\n"));
    let a = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    let b = load_checked(&text, &UnitCatalog::standard()).unwrap();
    assert_eq!(a.resolved.fingerprint, b.resolved.fingerprint);
    assert_eq!(a.resolved.tags, b.resolved.tags);
}

#[test]
fn load_rule_errors_stop_before_the_unit_rules() {
    // 位址錯誤（R18）時不再跑 U 規則，避免連鎖雜訊
    let bad = FIXTURE.replace("address: X10, type: bool", "address: Q99, type: bool");
    assert_eq!(codes(&bad), ["R18"]);
}

#[test]
fn stale_type_version_is_u2() {
    let stale = FIXTURE.replace(
        "id: cyl1, type: cylinder_double, type_version: 1",
        "id: cyl1, type: cylinder_double, type_version: 2",
    );
    assert_eq!(codes(&stale), ["U2"]);
}

#[test]
fn operation_pin_on_a_read_only_input_is_rejected() {
    let bad = FIXTURE.replace("man_ext: plc1.M_Cyl1_ManExt", "man_ext: plc1.X_Cyl1_Ext_LS");
    let c = codes(&bad);
    assert!(c.contains(&"U13"), "{c:?}");
    assert!(c.contains(&"U11"), "{c:?}"); // 同一個 X 點也被 lim_ext 綁著
}

#[test]
fn safety_monitor_tag_cannot_be_an_operation() {
    let bad = FIXTURE.replace(
        "man_ext: plc1.M_Cyl1_ManExt",
        "man_ext: plc1.M_Estop",
    );
    assert!(codes(&bad).contains(&"U13"));
}

#[test]
fn missing_error_binding_is_u6() {
    let bad = FIXTURE.replace("error_ext: plc1.M_Cyl1_ErrExt, ", "");
    assert_eq!(codes(&bad), ["U6"]);
}

#[test]
fn warnings_do_not_fail_the_load() {
    let text = FIXTURE.replace(
        "{ id: plc1.M_Cyl1_ManExt, address: M100, type: bool }",
        "{ id: plc1.M_Cyl1_ManExt, address: M100, type: bool, role: status }",
    );
    let checked = load_checked(&text, &UnitCatalog::standard()).unwrap();
    assert_eq!(checked.warnings.len(), 1);
    assert_eq!(checked.warnings[0].code, "U14");
    assert_eq!(checked.warnings[0].severity, Severity::Warning);
}

#[test]
fn a_catalog_without_the_needed_type_is_u2() {
    let empty = UnitCatalog::from_sources(&[]).unwrap().0;
    let err = load_checked(FIXTURE, &empty).unwrap_err();
    assert!(err.iter().any(|d| d.code == "U2"), "{err:#?}");
}

#[test]
fn unreferenced_tags_are_never_write_candidates() {
    let checked = load_checked(FIXTURE, &UnitCatalog::standard()).unwrap();
    let spare = checked.resolved.tag("plc1.M_Spare").unwrap();
    assert!(!spare.write_candidate);
    assert!(spare.bindings.is_empty());
    assert!(spare.binding().is_none());
}
```

- [ ] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test e2e`
Expected: FAIL（編譯錯誤：`load_checked` 未定義）。

- [x] **Step 3: 寫實作（加在 `lib.rs` 檔尾）**

```rust
/// [`load_checked`] 的成功結果。
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub resolved: resolve::ResolvedModel,
    /// `Warning` 與 `Info` 診斷（沒有 `Error`）。
    pub warnings: Vec<Diagnostic>,
}

/// 完整載入：載入安全規則 → 類型目錄上的 U 規則 → 推導。
///
/// 載入安全規則有錯時只回那些錯誤，不再執行 U 規則（避免連鎖雜訊）。
pub fn load_checked(
    text: &str,
    catalog: &catalog::UnitCatalog,
) -> Result<Checked, Vec<Diagnostic>> {
    let model = load(text)?;
    let diagnostics = analyze::check(&model, catalog);
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(diagnostics);
    }
    Ok(Checked {
        resolved: resolve::resolve(&model, catalog, text),
        warnings: diagnostics,
    })
}
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test --workspace`
Expected: PASS。`a_catalog_without_the_needed_type_is_u2` 的白名單只是防止連鎖雜訊以外的規則混入；若實際出現其他編號（例如 `U5`），把它加進白名單並說明原因即可，但必須保留 `U2` 存在的斷言。

- [x] **Step 5: 最終驗證（回報給控制端）**

Run:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
grep -rnE '"(R3|R4|R20|R23)"|writable|write_permission' machine-model/src || echo "OK: 沒有已廢除的規則與欄位"
cargo tree -p machine-model --edges normal --prefix none | sort -u
```

Expected：測試全過；`grep` 輸出 `OK: …`（`schema.rs` 測試裡拿 `writable`／`write_permission` 當「必須被拒絕的欄位」的字串、以及 `unit_type.rs` 同類測試是允許的，請在回報中逐一列出）；`cargo tree` 只有 `machine-model`、`serde*`、`serde_yaml_ng` 及其依賴（`indexmap`、`itoa`、`ryu`、`unsafe-libyaml` 之類）。回報測試總數與對規格的任何偏離。**不要提交。**

---

## Self-Review（作者填寫，執行者不需處理）

- **Spec coverage（§12 第 1 項後半）**：前綴表（Task 1）、類型檔型別與 SC2／SC3（Task 2）、目錄含 U17／U1／U12／U15／U18／U19 與本地覆蓋（Task 3）、內嵌標準類型與版本釘選（Task 4）、U2／U3／U5／U7／U8／U9（Task 5）、U4／U6／U10／U11／U13／U14（Task 6）、推導與 §9.2 資料結構、U16 覆蓋率（Task 7）、單一入口（Task 8）。
- **規格解讀（已獲使用者確認並固定）**：
  1. U7 只管 Unit；Tag 多重綁定歸 U11 規範。
  2. U4 從嚴：`field.in` 須為唯讀輸入類別（如 X），`field.out` 須為實體輸出類別（如 Y）；型別必須完全相等。
  3. U11 多重綁定規則：只有 `field.out` 必須全機唯一（防止雙重輸出爭奪）；其餘接腳（`field.in`、`par_*`、`man_*`、`at_*` 等）允許多個單元共用同一 Tag（如共用感測輸入、共用參數、共用同動操作），但同一 Tag 綁定的所有接腳前綴與語意必須相容（禁止混用 field/ctrl、禁止混用可寫候選與唯讀、型別必須一致）。
  4. 解析結果結構：`ResolvedTag.bindings: Vec<TagBinding>` 記錄綁定該 Tag 的所有單元與接腳，並提供 `binding()` 輔助方法回傳第一個綁定。
  5. U6：field 腳明寫 `null` 視為刻意不接而通過；`error_` 腳 `null` 不通過。
  6. U15 以「功能名＝前綴後第一段」為單位，只在單一類型的 `ctrl` 內比較。
  7. `cmd_`／`ena_` 允許綁 Tag（規格只說「不需」），不算覆蓋缺口。
  8. SC2／SC3 在本計畫以程式碼實作；JSON Schema 版（下一份計畫）必須與其一致。
  9. U2／U5 連鎖防護：`analyze/graph.rs` 在子層單元之類型不在目錄內（已報 U2）時，跳過父層 part 的 U5 類型比對，避免連鎖雜訊。
  10. `ResolvedTag` 完整性：收錄 `spare`、`polarity`、`ext`，確保下游（PlcDataService 與匯出）原樣轉發不失真；`ResolvedPlc` 則合理裁剪掉靜態設定欄位（`title`、`diff_scope`、`sources`）。
  11. U1 職責分工：`units[].id` 唯一由載入階段之 R26 負責，單元類型內名稱唯一性由目錄建構時的 U1 負責。
  12. Fixture 區隔：`structural_full.yaml` 僅供載入安全規則測試；`valid_units.yaml` 才是符合 U 規則的標準實例。
- **尚未涵蓋**：`Serialize` 衍生（`dump` 指令用，屬 CLI 計畫）、`schema` feature、命令列、PlcDataService 整合、HMI 覆蓋率報告的輸出格式。
- **型別一致**：`UnitCatalog::get`／`standard`／`from_sources`／`with_local`（Task 3、4）在 Task 5～8 照用；`PinRef`（Task 2）在 Task 5、6、7 照用；`analyze::{check, check_graph, check_pins}`（Task 5）在 Task 6、8 照用；`ResolvedModel` 欄位（Task 7）在 Task 8 照用。
- **已知手動核對點**：Task 5 測試字串替換（`replace`）容易抄錯，已註明可改寫；Task 7 的 pins 排序期望已註明依實作順序修正。
