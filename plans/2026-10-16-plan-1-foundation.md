# MachineModel 函式庫（一）：專案骨架、v2 型別與載入安全規則 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 MachineModel 專案建立 Cargo workspace 與 `machine-model` 函式庫的第一層：`machine.yaml` v2 的型別、解析入口與全部「不需類型目錄」的載入安全規則。

**Architecture:** 單一函式庫 crate `machine-model`，依賴只有 `serde` 與 `serde_yaml_ng`。家族／位址／型別／嚴格 map 等資料表自 AIAutomation 專案 `model/machine-model` 原樣搬入；`schema.rs`（v2 型別）與 `rules/`（載入安全規則）依規格 v0.3 重寫：Tag 沒有 `unit_id`／`writable`／`write_permission`，`units[]` 是帶 `type_version` 的單元實例，沒有 v1。類型檔、前綴表、U 規則與推導在下一份計畫（Plan 2）。

**Tech Stack:** Rust edition 2024，`rust-version = "1.85"`，`serde 1.0.229`，`serde_yaml_ng 0.10.0`。

**Spec:** `2026-10-16-unit-tag-model-design.md`（本專案根目錄，v0.3）。主要依據 §3（machine.yaml v2）、§8.3「載入安全規則」、§8.5（依賴限制）、§12 第 1 項。

**Source for ported code:** `/mnt/c/Users/wydct/orca/projects/AIAutomation/model/machine-model/`（以下簡稱「舊 crate」，唯讀，不得修改）。

## Global Constraints

- 本專案根目錄：`/mnt/c/Users/wydct/orca/projects/MachineModel`（git repo，分支沿用現有）。所有指令在此目錄執行，除非另有說明。
- 函式庫本體依賴只有 `serde`、`serde_yaml_ng`；不得依賴 `tokio`、`axum`、`reqwest`、`hyper`、`clap`、`jsonschema`（由依賴規則測試把關，Task 7）。
- 不使用 let-chains（`rust-version = "1.85"`）。
- 診斷訊息用繁體中文並帶規則編號（`R1`…`R28`、`PARSE`）。
- 讀取端拒絕未知欄位（`deny_unknown_fields`）；擴充只能走 `ext`。Tag **沒有** `unit_id`、`writable`、`write_permission`，寫了就是 `R22`。
- 本計畫涵蓋的規則：`R1`、`R2`、`R5`、`R6`、`R8`、`R9`、`R17`、`R18`、`R19`、`R21`、`R22`、`R24`、`R25`、`R26`、`R27`、`R28`。`R3`、`R4`、`R20`、`R23` 在 v2 **不存在**，任何地方都不得出現這些編號。
- 每個任務結束時必須通過：`cargo test -p machine-model`、`cargo clippy -p machine-model --all-targets -- -D warnings`、`cargo fmt -p machine-model --check`。
- **不要 `git add`／`git commit`**：提交由控制端在審查後、使用者同意時進行。計畫中的 Checkpoint 步驟只跑驗證。

## Review Focus

以下輸入規格沒逐項點名，但最可能咬到使用者；各自在對應任務有測試：

1. 舊 v1 寫法（Tag 帶 `writable`／`write_permission`／`unit_id`，或頂層 `devices`／`alarms`／`parameters`）→ 必須是 `R22`，不是靜默忽略（Task 3）。
2. `schema_version` 缺漏／為 1 → 缺漏是 `PARSE`，為 1 是 `R27` 且不再往下解析（Task 3）。
3. `bind` 值為空字串、含空白、`type_version: 0`、bind 鍵重複 → 分別是 `R28`、`R28`、`R28`、`PARSE`（Task 3、4）。
4. UTF-8 BOM 與 CRLF 的檔案可正常載入（Task 3）。
5. id 含中文或連字號、tag 與 variable 同名 → `R1`（Task 4）。

## File Structure

```
MachineModel/
  Cargo.toml                      workspace（Task 1）
  .gitignore                      （Task 1）
  AGENTS.md / CLAUDE.md           （Task 1）
  machine-model/
    Cargo.toml                    （Task 1）
    src/lib.rs                    入口：parse／validate／load（Task 1 骨架，Task 3 完成）
    src/diag.rs                   Diagnostic（Task 1，原樣搬入）
    src/family.rs                 PLC 家族與位址類別表（Task 1，原樣搬入）
    src/address.rs                位址解析（Task 1，原樣搬入）
    src/types.rs                  標量型別詞彙（Task 1，原樣搬入）
    src/strict.rs                 StrictMap／Ext（Task 1，原樣搬入）
    src/schema.rs                 v2 型別（Task 2，重寫）
    src/rules/mod.rs              規則入口 run（Task 3，隨後各任務擴充）
    src/rules/structure.rs        R21、R1、R27（Task 4）
    src/rules/units.rs            R25、R26、R28（Task 4）
    src/rules/values.rs           R2、R5、R6、R9、R17（Task 5）
    src/rules/addresses.rs        R8、R18、R19、R24（Task 6）
    tests/common/mod.rs           共用測試工具（Task 3）
    tests/parse.rs                （Task 3）
    tests/structure.rs            （Task 4）
    tests/values.rs               （Task 5）
    tests/addresses.rs            （Task 6）
    tests/fixture.rs              （Task 7）
    tests/fixtures/valid_full.yaml（Task 7）
    tests/dependency_rules.rs     （Task 7）
```

---

### Task 1: 專案骨架與原樣搬入的資料表

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `AGENTS.md`, `CLAUDE.md`, `machine-model/Cargo.toml`, `machine-model/src/lib.rs`
- Copy（原樣）: `diag.rs`, `family.rs`, `address.rs`, `types.rs`, `strict.rs`

**Interfaces:**
- Produces（供後續任務使用，簽名不變）:
  - `diag::{Diagnostic, Severity}`；`Diagnostic::error(code: &'static str, location: impl Into<String>, message: impl Into<String>) -> Diagnostic`；`Diagnostic::is_error(&self) -> bool`；欄位 `code`、`severity`、`message`、`location: Option<String>`。
  - `family::{Family, ClassProfile, Width}`；`Family::profile()`、`Family::as_str()`；`ClassProfile` 欄位 `name`、`max`、`width`、`read_only`；`profile().protocols`。
  - `address::{parse, Address}`；`address::parse(family: Family, text: &str) -> Result<Address, String 類錯誤>`；`Address` 欄位 `class: &'static ClassProfile`、`number: u32`、`bit: Option<u8>`。
  - `types::{DataType, ScalarType}`；`DataType::parse(&str)`、欄位 `scalar`、`array`、`len`；方法 `is_bool()`、`words()`、`int_bounds()`。
  - `strict::{Ext, StrictMap<T>}`；`StrictMap<T>(pub BTreeMap<String, T>)`。

- [x] **Step 1: 建立 workspace 與 crate 檔案**

`Cargo.toml`：

```toml
[workspace]
resolver = "3"
members = ["machine-model"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
module_name_repetitions = "allow"

[workspace.dependencies]
serde = { version = "1.0.229", features = ["derive"] }
serde_yaml_ng = "0.10.0"
```

`machine-model/Cargo.toml`：

```toml
[package]
name = "machine-model"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true
description = "machine.yaml v2（Unit／Tag 模型）的型別、解析、載入安全規則與推導"

[dependencies]
serde = { workspace = true }
serde_yaml_ng = { workspace = true }

[lints]
workspace = true
```

`.gitignore`：

```
/target
```

`AGENTS.md`：

```markdown
# Agent Instructions

MachineModel：`machine.yaml` v2（Unit／Tag 模型）的函式庫與工具。規格是根目錄的 `2026-10-16-unit-tag-model-design.md`；欄位與規則要改，先改規格。計畫放 `plans/`。

## 規則

- `machine-model` 函式庫本體只依賴 `serde` 與 `serde_yaml_ng`。不得依賴 `tokio`、`axum`、`reqwest`、`hyper`、`clap`；`jsonschema` 只能在 `schema` feature 之後（`default-features = false`）。由 `tests/dependency_rules.rs` 把關。
- 讀取端拒絕未知欄位，擴充只能走 `ext`。Tag 沒有 `unit_id`、`writable`、`write_permission`。
- 診斷訊息用繁體中文，並帶規則編號（`R…`、`U…`、`SC…`）。
- 家族的位址類別、範圍、可用協定是 `src/family.rs` 的資料表。未經實機驗證的範圍要在註解標明。
- 不使用 let-chains（`rust-version = "1.85"`）。
- 只有人確認過的 Unit 類型才能進標準目錄；AI 不能自創。

## 驗證

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
```

`CLAUDE.md`：

```
@AGENTS.md
```

- [x] **Step 2: 原樣搬入五個檔案並建立骨架 lib.rs**

```bash
SRC=/mnt/c/Users/wydct/orca/projects/AIAutomation/model/machine-model/src
mkdir -p machine-model/src
for f in diag family address types strict; do cp "$SRC/$f.rs" machine-model/src/$f.rs; done
```

`machine-model/src/lib.rs`（骨架，Task 3 會取代）：

```rust
//! `machine.yaml` v2（Unit／Tag 模型）的型別、解析與載入安全規則。

pub mod address;
pub mod diag;
pub mod family;
pub mod strict;
pub mod types;
```

- [x] **Step 3: 驗證搬入的檔案單獨可編譯、其內建測試通過**

Run: `cargo test -p machine-model`
Expected: PASS（`diag`、`family`、`address`、`types`、`strict` 內的單元測試；無編譯錯誤）。

- [x] **Step 4: Checkpoint**

Run: `cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 無警告、無格式差異。若 `diag.rs` 內建測試引用的 `R3` 字樣只是測試字串，可保留；若 clippy 對搬入檔案有新警告，只做最小修正並回報。

---

### Task 2: v2 型別 `schema.rs`

**Files:**
- Create: `machine-model/src/schema.rs`
- Modify: `machine-model/src/lib.rs`（加 `pub mod schema;`）

**Interfaces:**
- Consumes: `family::Family`、`strict::{Ext, StrictMap}`。
- Produces:
  - `schema::MachineModel { schema_version: u32, machine: Machine, units: Vec<UnitInstance>, tags: Vec<Tag>, variables: Vec<Variable> }`
  - `schema::Machine { id: String, title: Option<String>, plcs: Vec<Plc> }`
  - `schema::Plc { id, title, family: Family, online: bool, diff_scope: Vec<String>, sources: Option<Sources> }`；`Sources`、`IoSource`、`ExportSource`（同舊 crate）
  - `schema::UnitInstance { id: String, ty: String (yaml: type), type_version: u32, bind: StrictMap<Option<String>> }`
  - `schema::Tag`（欄位見規格 §3.2；**無** `unit_id`／`writable`／`write_permission`），方法 `scan() -> Scan`、`symbol() -> Option<&str>`
  - `schema::Variable { id, symbol, ty, description, retain, default, address: Option<String>, ext }`
  - `schema::{Scan, Role, Polarity, Scalar}`
  - 常數 `UNASSIGNED_UNIT = "unassigned"`、`MACHINE_UNIT = "machine"`
  - `schema::{is_valid_plc_id, is_valid_name, split_id}`
  - `schema::BindRef<'a> { Tag(&'a str), Unit(&'a str) }`、`schema::classify_bind(&str) -> Option<BindRef<'_>>`

- [x] **Step 1: 寫 `schema.rs`（含內建測試，測試先寫在檔尾）**

```rust
//! `machine.yaml` v2 的型別（規格 §3）。每個 struct 都拒絕未知欄位；`ext` 是唯一的擴充通道。
//!
//! Tag 沒有 `unit_id`（歸屬由父層 `bind` 宣告）、`writable`、`write_permission`
//! （最終可寫由 PlcDataService 決定，規格 §6.3）。

use serde::Deserialize;

use crate::family::Family;
use crate::strict::{Ext, StrictMap};

/// 保留的單元實例 id：機台根。
pub const MACHINE_UNIT: &str = "machine";
/// 保留的單元 id：不得宣告。
pub const UNASSIGNED_UNIT: &str = "unassigned";

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineModel {
    pub schema_version: u32,
    pub machine: Machine,
    #[serde(default)]
    pub units: Vec<UnitInstance>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub variables: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub id: String,
    pub title: Option<String>,
    #[serde(default)]
    pub plcs: Vec<Plc>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plc {
    pub id: String,
    pub title: Option<String>,
    pub family: Family,
    #[serde(default = "default_true")]
    pub online: bool,
    /// 位址類別或區間；語法由漂移比對解析，這裡只保存。
    #[serde(default)]
    pub diff_scope: Vec<String>,
    pub sources: Option<Sources>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub io_table: Option<IoSource>,
    pub plc_export: Option<ExportSource>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoSource {
    pub file: String,
    pub sheet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSource {
    pub file: String,
    pub reader: Option<String>,
}

/// 單元實例：由 `type`（Unit 類型檔 id）、`type_version` 與 `bind` 組成（規格 §3.1）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitInstance {
    pub id: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// 必填，必須等於類型目錄中該類型的 `version`（U2，Plan 2）；`0` 是 `R28`。
    pub type_version: u32,
    /// 接腳名或 part 名 → 值。值是 Tag id、Unit id，或 `null`（刻意不接）。
    #[serde(default)]
    pub bind: StrictMap<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scan {
    Fast,
    Normal,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ProcessValue,
    Setpoint,
    Command,
    Mode,
    Status,
    Alarm,
    Counter,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Polarity {
    #[serde(rename = "NO")]
    NormallyOpen,
    #[serde(rename = "NC")]
    NormallyClosed,
}

/// `default` 的值。順序影響 untagged 解析：布林、整數、浮點、字串。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// 一個 PLC 位址上的點（規格 §3.2）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub address: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub symbol: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub scan: Option<Scan>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
    pub deadband: Option<f64>,
    /// 工程單位（如 `degC`）。
    pub unit: Option<String>,
    pub readback: Option<bool>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub default: Option<Scalar>,
    pub retain: Option<bool>,
    /// 明確覆寫用，通常省略（規格 §6.1）。
    pub role: Option<Role>,
    pub polarity: Option<Polarity>,
    pub safety_monitor: Option<bool>,
    pub spare: Option<bool>,
    #[serde(default)]
    pub ext: Ext,
}

impl Tag {
    /// 省略時為 `normal`。
    pub fn scan(&self) -> Scan {
        self.scan.unwrap_or(Scan::Normal)
    }

    /// `symbol` 省略時取 `id` 的名稱部分。
    pub fn symbol(&self) -> Option<&str> {
        self.symbol
            .as_deref()
            .or_else(|| split_id(&self.id).map(|(_, name)| name))
    }
}

/// 沒有軟元件位址的 PLC 變數。`address` 欄位只為了在有人誤填時報 `R25`。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    pub id: String,
    pub symbol: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub description: Option<String>,
    pub retain: Option<bool>,
    pub default: Option<Scalar>,
    pub address: Option<String>,
    #[serde(default)]
    pub ext: Ext,
}

/// PLC id 格式：`plc` 加不以 0 開頭的正整數。
pub fn is_valid_plc_id(id: &str) -> bool {
    let Some(n) = id.strip_prefix("plc") else {
        return false;
    };
    !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && !n.starts_with('0')
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// 把 `plc1.Name` 拆成 `("plc1", "Name")`；格式不符回 `None`。
pub fn split_id(id: &str) -> Option<(&str, &str)> {
    let (plc, name) = id.split_once('.')?;
    (is_valid_plc_id(plc) && is_name(name)).then_some((plc, name))
}

/// 單元實例 id、類型 id、接腳名的字元規則：ASCII 英數與底線。
pub fn is_valid_name(s: &str) -> bool {
    is_name(s)
}

/// `bind` 值的種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindRef<'a> {
    Tag(&'a str),
    Unit(&'a str),
}

/// 含 `.` 的是 Tag id（`plc<N>.<Name>`），不含 `.` 的是 Unit id；格式不符回 `None`。
/// 不檢查 PLC 是否已宣告，也不檢查引用是否存在。
pub fn classify_bind(value: &str) -> Option<BindRef<'_>> {
    if value.contains('.') {
        split_id(value).map(|_| BindRef::Tag(value))
    } else {
        is_valid_name(value).then_some(BindRef::Unit(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_id_accepts_only_plc_prefixed_ids() {
        assert_eq!(split_id("plc1.X000"), Some(("plc1", "X000")));
        assert_eq!(
            split_id("plc12.I_LeftGripper_ClosedLimit").unwrap().0,
            "plc12"
        );
        for bad in [
            "X000", "plc.X000", "plc0.X000", "plc01.X", "plc1.", "plc1.a-b", "plc1.貼標",
            "plc1.a.b", ".X",
        ] {
            assert_eq!(split_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn tag_defaults() {
        let t: Tag = serde_yaml_ng::from_str("id: plc1.A\naddress: M1\ntype: bool\n").unwrap();
        assert_eq!(t.scan(), Scan::Normal);
        assert_eq!(t.symbol(), Some("A"));
    }

    #[test]
    fn tag_rejects_v1_only_fields() {
        for field in ["writable: true", "write_permission: P01", "unit_id: st1"] {
            let text = format!("id: plc1.A\naddress: M1\ntype: bool\n{field}\n");
            let err = serde_yaml_ng::from_str::<Tag>(&text).unwrap_err().to_string();
            assert!(err.contains("unknown field"), "{field}: {err}");
        }
    }

    #[test]
    fn unit_instance_requires_type_version_and_allows_null_bind() {
        let ok: UnitInstance = serde_yaml_ng::from_str(
            "id: cyl1\ntype: cylinder_double\ntype_version: 1\nbind: {a: plc1.A, b: null, c: st1}\n",
        )
        .unwrap();
        assert_eq!(ok.bind.0["b"], None);
        assert_eq!(ok.bind.0["a"].as_deref(), Some("plc1.A"));
        let missing = serde_yaml_ng::from_str::<UnitInstance>("id: cyl1\ntype: cylinder_double\n");
        assert!(missing.is_err());
    }

    #[test]
    fn classify_bind_distinguishes_tag_unit_and_garbage() {
        assert_eq!(classify_bind("plc1.A"), Some(BindRef::Tag("plc1.A")));
        assert_eq!(classify_bind("st1"), Some(BindRef::Unit("st1")));
        for bad in ["", "st-1", "a b", "plc1.", "x.y", "貼標"] {
            assert_eq!(classify_bind(bad), None, "{bad}");
        }
    }
}
```

- [x] **Step 2: 在 `lib.rs` 加 `pub mod schema;`（置於 `pub mod family;` 之後，維持字母序）**

- [x] **Step 3: 執行測試**

Run: `cargo test -p machine-model schema`
Expected: PASS（5 個 schema 測試）。若 `tag_rejects_v1_only_fields` 的錯誤字串不含 `unknown field`，代表 serde_yaml_ng 措辭不同——先 `cargo test` 看實際字串，回報而不是改測試。

- [x] **Step 4: Checkpoint**

Run: `cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 乾淨。（`fmt` 可能要重排測試裡的長陣列；執行 `cargo fmt -p machine-model` 後再檢查。）

---

### Task 3: 解析入口 `parse`／`load` 與共用測試工具

**Files:**
- Modify: `machine-model/src/lib.rs`（取代骨架）
- Create: `machine-model/src/rules/mod.rs`（暫時只有版本檢查）、`machine-model/src/rules/structure.rs`（暫時只有 `version_error`）
- Create: `machine-model/tests/common/mod.rs`, `machine-model/tests/parse.rs`

**Interfaces:**
- Consumes: `schema::MachineModel`、`diag::Diagnostic`。
- Produces:
  - `pub const SCHEMA_VERSION: u32 = 2;`
  - `pub fn parse(text: &str) -> Result<MachineModel, Diagnostic>`
  - `pub fn validate(model: &MachineModel) -> Vec<Diagnostic>`（呼叫 `rules::run`）
  - `pub fn load(text: &str) -> Result<MachineModel, Vec<Diagnostic>>`
  - `rules::run(&MachineModel) -> Vec<Diagnostic>`、`pub(crate) rules::structure::version_error(found: u32, supported: u32) -> Diagnostic`
  - 測試工具 `common::{HEAD, doc, tags, codes, tag_codes}`

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/common/mod.rs`：

```rust
//! 各測試檔共用的小工具。
#![allow(dead_code)]

use machine_model::load;

/// 兩個 PLC 的固定表頭（plc2 離線）。
pub const HEAD: &str = "\
schema_version: 2
machine:
  id: m1
  plcs:
    - id: plc1
      family: inovance_easy
    - id: plc2
      family: mitsubishi_fx3g
      online: false
";

/// 把其餘區塊接在固定表頭後面。
pub fn doc(body: &str) -> String {
    format!("{HEAD}{body}")
}

/// 以 flow 寫法組出 `tags:` 區塊。
pub fn tags(entries: &[&str]) -> String {
    format!("tags:\n  - {}\n", entries.join("\n  - "))
}

/// 載入並回傳全部診斷的規則編號；沒有錯誤時回空。
pub fn codes(text: &str) -> Vec<&'static str> {
    match load(text) {
        Ok(_) => vec![],
        Err(d) => d.iter().map(|x| x.code).collect(),
    }
}

pub fn tag_codes(entries: &[&str]) -> Vec<&'static str> {
    codes(&doc(&tags(entries)))
}
```

`machine-model/tests/parse.rs`：

```rust
mod common;

use common::{HEAD, codes, doc, tags};
use machine_model::schema::Scan;
use machine_model::{load, parse, validate};

#[test]
fn defaults_apply_when_fields_are_omitted() {
    let model = parse(&doc(&tags(&["{id: plc1.A, address: M1, type: bool}"]))).unwrap();
    assert_eq!(model.tags[0].scan(), Scan::Normal);
    assert!(model.units.is_empty());
    assert!(model.machine.plcs[0].online);
    assert!(!model.machine.plcs[1].online);
}

#[test]
fn utf8_bom_and_crlf_are_accepted() {
    let text = format!(
        "\u{feff}{}",
        doc(&tags(&["{id: plc1.A, address: M1, type: bool}"])).replace('\n', "\r\n")
    );
    assert_eq!(codes(&text), Vec::<&str>::new());
}

#[test]
fn unknown_field_is_r22() {
    let diag = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, bogus: 1}",
    ])))
    .unwrap_err();
    assert_eq!(diag.code, "R22");
}

#[test]
fn unknown_top_level_field_is_r22() {
    let diag = parse(&format!("{HEAD}extra: 1\n")).unwrap_err();
    assert_eq!(diag.code, "R22");
}

#[test]
fn v1_only_fields_are_r22_not_ignored() {
    for field in ["writable: true", "write_permission: P01", "unit_id: st1"] {
        let entry = format!("{{id: plc1.A, address: M1, type: bool, {field}}}");
        assert_eq!(parse(&doc(&tags(&[&entry]))).unwrap_err().code, "R22", "{field}");
    }
    for block in ["devices: []", "alarms: []", "parameters: []"] {
        let diag = parse(&format!("{HEAD}{block}\n")).unwrap_err();
        assert_eq!(diag.code, "R22", "{block}");
    }
}

#[test]
fn other_syntax_problems_are_parse() {
    for text in [
        String::new(),
        "schema_version: 2\nmachine: [".to_owned(),
        // 缺少必填欄位 type
        doc(&tags(&["{id: plc1.A, address: M1}"])),
        // 未知家族
        HEAD.replace("inovance_easy", "siemens"),
        // 缺少 schema_version
        HEAD.replace("schema_version: 2\n", ""),
        // 單元缺 type_version
        doc("units:\n  - {id: u1, type: t}\n"),
    ] {
        let diag = parse(&text).unwrap_err();
        assert_eq!(diag.code, "PARSE", "{text}");
    }
}

#[test]
fn yaml_no_is_not_silently_a_boolean() {
    // `no` 填給布林欄位：報錯，不會被當成 false
    let diag = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, retain: no}",
    ])))
    .unwrap_err();
    assert_eq!(diag.code, "PARSE");
    // `no` 填給字串欄位：保持字串
    let model = parse(&doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, description: no}",
    ])))
    .unwrap();
    assert_eq!(model.tags[0].description.as_deref(), Some("no"));
}

#[test]
fn duplicate_keys_are_rejected() {
    let dup_field = doc("tags:\n  - id: plc1.A\n    id: plc1.B\n    address: M1\n    type: bool\n");
    assert_eq!(parse(&dup_field).unwrap_err().code, "PARSE");
    let dup_ext = doc(&tags(&[
        "{id: plc1.A, address: M1, type: bool, ext: {inovance: {seq: 1, seq: 2}}}",
    ]));
    assert_eq!(parse(&dup_ext).unwrap_err().code, "PARSE");
    let dup_bind = doc("units:\n  - {id: u1, type: t, type_version: 1, bind: {a: st1, a: st2}}\n");
    assert_eq!(parse(&dup_bind).unwrap_err().code, "PARSE");
}

#[test]
fn schema_version_1_is_r27_even_with_unknown_fields() {
    let text = "schema_version: 1\nmachine: {id: m1}\nfuture_block: []\n";
    assert_eq!(parse(text).unwrap_err().code, "R27");
    let diags = load(text).unwrap_err();
    assert_eq!(diags.len(), 1);
}

#[test]
fn validate_flags_a_wrong_version_on_a_constructed_model() {
    let mut model = parse(&doc("")).unwrap();
    model.schema_version = 3;
    let codes: Vec<_> = validate(&model).iter().map(|d| d.code).collect();
    assert_eq!(codes, ["R27"]);
}

#[test]
fn parse_diagnostics_are_traditional_chinese() {
    let unknown = parse(&format!("{HEAD}extra: 1\n")).unwrap_err();
    assert!(unknown.message.starts_with("未知欄位"), "{}", unknown.message);
    let syntax = parse("schema_version: 2\nmachine: [").unwrap_err();
    assert!(
        syntax.message.starts_with("YAML 格式或型別錯誤"),
        "{}",
        syntax.message
    );
}
```

- [x] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test parse`
Expected: FAIL（編譯錯誤：`machine_model::parse` 等未定義）。

- [x] **Step 3: 寫實作**

`machine-model/src/lib.rs`（取代骨架）：

```rust
//! `machine.yaml` v2（Unit／Tag 模型）的型別、解析與載入安全規則。
//!
//! 執行期（PlcDataService）與設計期工具共用本 crate。規格見專案根目錄
//! `2026-10-16-unit-tag-model-design.md`。

pub mod address;
pub mod diag;
pub mod family;
pub mod rules;
pub mod schema;
pub mod strict;
pub mod types;

use diag::Diagnostic;
use schema::MachineModel;
use serde::Deserialize;

/// 本 crate 支援的 `schema_version`。
pub const SCHEMA_VERSION: u32 = 2;

#[derive(Deserialize)]
struct Probe {
    schema_version: Option<u32>,
}

fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// 解析 `machine.yaml` v2 文字。
///
/// - 開頭的 UTF-8 BOM（Windows 編輯器常加）會被忽略。
/// - `schema_version` 不是 [`SCHEMA_VERSION`] 時回 `R27`，不再往下解析。
/// - 未知欄位回 `R22`；其他語法或型別錯誤回 `PARSE`。
pub fn parse(text: &str) -> Result<MachineModel, Diagnostic> {
    let text = strip_bom(text);

    let declared = serde_yaml_ng::from_str::<Probe>(text)
        .ok()
        .and_then(|p| p.schema_version);
    if let Some(v) = declared {
        if v != SCHEMA_VERSION {
            return Err(rules::version_error(v, SCHEMA_VERSION));
        }
    }

    serde_yaml_ng::from_str::<MachineModel>(text).map_err(|e| {
        let detail = e.to_string();
        // R22 的判定依賴 serde 的英文用語；由 R22 相關測試釘住。
        let (code, message) = if detail.contains("unknown field") {
            (
                "R22",
                format!("未知欄位（只接受規格定義的欄位，擴充請放在 ext）：{detail}"),
            )
        } else {
            ("PARSE", format!("YAML 格式或型別錯誤：{detail}"))
        };
        let location = e.location().map_or_else(
            || "文件".to_owned(),
            |l| format!("第 {} 行第 {} 欄", l.line(), l.column()),
        );
        Diagnostic::error(code, location, message)
    })
}

/// 對已解析的模型執行全部載入安全規則。
pub fn validate(model: &MachineModel) -> Vec<Diagnostic> {
    rules::run(model)
}

/// 解析並驗證。沒有任何錯誤時回模型；否則回全部診斷。
pub fn load(text: &str) -> Result<MachineModel, Vec<Diagnostic>> {
    let model = parse(text).map_err(|d| vec![d])?;
    let diagnostics = validate(&model);
    if diagnostics.iter().any(Diagnostic::is_error) {
        Err(diagnostics)
    } else {
        Ok(model)
    }
}
```

`machine-model/src/rules/mod.rs`：

```rust
//! 載入安全規則（規格 §8.3）。PlcDataService 載入與命令列工具共用同一份。
//!
//! R22（未知欄位）與 R27 的解析階段檢查在 [`crate::parse`]；其餘規則在這裡。
//! 引用是否存在、單親、無環、類型符合性需要類型目錄，屬 U 規則（Plan 2）。

mod structure;

pub(crate) use structure::version_error;

use crate::SCHEMA_VERSION;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

/// 執行全部載入安全規則。順序固定：版本、機台與 PLC、id、單元、欄位值、位址。
pub fn run(model: &MachineModel) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if model.schema_version != SCHEMA_VERSION {
        out.push(version_error(model.schema_version, SCHEMA_VERSION));
    }
    out
}
```

`machine-model/src/rules/structure.rs`（暫時）：

```rust
//! 結構規則：R1、R21、R27。

use crate::diag::Diagnostic;

pub(crate) fn version_error(found: u32, supported: u32) -> Diagnostic {
    Diagnostic::error(
        "R27",
        "schema_version",
        format!("不支援 schema_version {found}，此工具只支援 {supported}"),
    )
}
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model --test parse`
Expected: PASS（10 個測試）。`other_syntax_problems_are_parse` 若某筆回的不是 `PARSE`，把實際 code 與該筆 text 回報，不要改期望值。

- [x] **Step 5: Checkpoint**

Run: `cargo test -p machine-model && cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 全過。（`structure.rs` 暫時未使用的 `use` 不會有；`dead_code` 警告若出現在 `version_error`，代表 `rules/mod.rs` 沒用到——它有用到。）

---

### Task 4: 結構與單元規則（R21、R1、R25、R26、R28）

**Files:**
- Modify: `machine-model/src/rules/structure.rs`（完整取代）、`machine-model/src/rules/mod.rs`
- Create: `machine-model/src/rules/units.rs`, `machine-model/tests/structure.rs`

**Interfaces:**
- Consumes: `schema::{Machine, MachineModel, UNASSIGNED_UNIT, is_valid_name, is_valid_plc_id, split_id, BindRef, classify_bind}`、`diag::Diagnostic`、`family::Family::{profile, as_str}`（`profile().protocols`）。
- Produces:
  - `structure::check_machine_and_plcs<'a>(&'a Machine, &mut Vec<Diagnostic>) -> BTreeSet<&'a str>`（回傳已宣告的 PLC id）
  - `structure::check_ids<'a>(impl Iterator<Item = (&'static str, &'a str)>, &BTreeSet<&str>, &mut Vec<Diagnostic>)`
  - `units::check(&MachineModel, &BTreeSet<&str>, &mut Vec<Diagnostic>)`

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/structure.rs`：

```rust
mod common;

use common::{HEAD, codes, doc, tag_codes, tags};

fn unit_codes(units: &str) -> Vec<&'static str> {
    codes(&doc(&format!("units:\n{units}")))
}

#[test]
fn r21_machine_and_plc_ids() {
    for id in ["a b", "貼標", "", "a/b"] {
        let text = HEAD.replace("id: m1", &format!("id: '{id}'"));
        assert_eq!(codes(&text), ["R21"], "{id:?}");
    }
    assert_eq!(codes(&HEAD.replace("id: m1", "id: demo-2_a")), Vec::<&str>::new());
    for plc in ["plc0", "plc", "PLC1", "plc01"] {
        let text = HEAD.replace("id: plc2", &format!("id: {plc}"));
        assert_eq!(codes(&text), ["R21"], "{plc}");
    }
    let dup = HEAD.replace("id: plc2", "id: plc1");
    assert_eq!(codes(&dup), ["R21"]);
}

#[test]
fn r1_tag_id_format_prefix_and_uniqueness() {
    for id in ["X000", "plc9.A", "plc1.a-b", "plc1.貼標", "plc1.", "plc0.A"] {
        let entry = format!("{{id: '{id}', address: M1, type: bool}}");
        assert_eq!(tag_codes(&[&entry]), ["R1"], "{id}");
    }
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: M1, type: bool}",
            "{id: plc1.A, address: M2, type: bool}",
        ]),
        ["R1"]
    );
}

#[test]
fn r1_tags_and_variables_share_a_namespace() {
    let body = format!(
        "{}variables:\n  - {{id: plc1.A, type: int16}}\n",
        tags(&["{id: plc1.A, address: M1, type: bool}"])
    );
    assert_eq!(codes(&doc(&body)), ["R1"]);
}

#[test]
fn r25_variables_must_not_have_an_address() {
    let body = "variables:\n  - {id: plc1.V, type: int16, address: D1}\n";
    assert_eq!(codes(&doc(body)), ["R25"]);
}

#[test]
fn r26_unit_instance_ids() {
    let unit = |id: &str| format!("  - {{id: {id}, type: t, type_version: 1}}\n");
    assert_eq!(unit_codes(&unit("unassigned")), ["R26"]);
    assert_eq!(unit_codes(&unit("'st-1'")), ["R26"]);
    assert_eq!(unit_codes(&unit("'貼標'")), ["R26"]);
    assert_eq!(
        unit_codes(&format!("{}{}", unit("st1"), unit("st1"))),
        ["R26"]
    );
    // `machine` 是根，允許宣告
    assert_eq!(unit_codes(&unit("machine")), Vec::<&str>::new());
}

#[test]
fn r28_type_version_type_and_bind_syntax() {
    let unit = |ty: &str, ver: &str, bind: &str| {
        format!("  - {{id: u1, type: {ty}, type_version: {ver}, bind: {{{bind}}}}}\n")
    };
    // 合法：Tag、Unit、null 三種值
    assert_eq!(
        unit_codes(&unit("t", "1", "a: plc1.A, b: null, c: st1")),
        Vec::<&str>::new()
    );
    assert_eq!(unit_codes(&unit("'bad-type'", "1", "")), ["R28"]);
    assert_eq!(unit_codes(&unit("t", "0", "")), ["R28"]);
    assert_eq!(unit_codes(&unit("t", "1", "'bad-key': st1")), ["R28"]);
    for bad in ["''", "'st-1'", "'a b'", "'plc1.'", "'x.y'", "'貼標'"] {
        assert_eq!(unit_codes(&unit("t", "1", &format!("a: {bad}"))), ["R28"], "{bad}");
    }
    // 引用了沒有宣告的 PLC
    assert_eq!(unit_codes(&unit("t", "1", "a: plc9.A")), ["R28"]);
}

#[test]
fn diagnostics_carry_chinese_message_code_and_location() {
    let diags = machine_model::load(&doc("variables:\n  - {id: plc1.V, type: int16, address: D1}\n"))
        .unwrap_err();
    let text = diags[0].to_string();
    assert!(text.starts_with("[R25] variables[plc1.V]："), "{text}");
    assert!(text.contains("不得有 address"), "{text}");
}
```

- [x] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test structure`
Expected: FAIL（規則尚未實作，多數測試回空診斷）。

- [x] **Step 3: 寫實作**

`machine-model/src/rules/structure.rs`（完整取代）：

```rust
//! 結構規則：R1、R21、R27。

use std::collections::BTreeSet;

use crate::diag::Diagnostic;
use crate::schema::{Machine, is_valid_plc_id, split_id};

pub(crate) fn version_error(found: u32, supported: u32) -> Diagnostic {
    Diagnostic::error(
        "R27",
        "schema_version",
        format!("不支援 schema_version {found}，此工具只支援 {supported}"),
    )
}

fn is_machine_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// R21：機台 id、PLC id 格式與唯一，離線以外的 PLC 必須有可用協定。回傳已宣告的 PLC id。
pub(super) fn check_machine_and_plcs<'a>(
    machine: &'a Machine,
    out: &mut Vec<Diagnostic>,
) -> BTreeSet<&'a str> {
    if !is_machine_id(&machine.id) {
        out.push(Diagnostic::error(
            "R21",
            "machine.id",
            format!(
                "機台 id `{}` 格式不符，只能使用英數、底線與連字號（ASCII）",
                machine.id
            ),
        ));
    }
    let mut seen = BTreeSet::new();
    for plc in &machine.plcs {
        let loc = format!("machine.plcs[{}]", plc.id);
        if !is_valid_plc_id(&plc.id) {
            out.push(Diagnostic::error(
                "R21",
                &loc,
                format!(
                    "PLC id `{}` 格式不符，必須是 plc 加正整數（如 plc1）",
                    plc.id
                ),
            ));
        }
        if !seen.insert(plc.id.as_str()) {
            out.push(Diagnostic::error("R21", &loc, "PLC id 重複"));
        }
        if plc.online && plc.family.profile().protocols.is_empty() {
            out.push(Diagnostic::error(
                "R21",
                &loc,
                format!(
                    "家族 {} 沒有可用的連線協定，必須設為 online: false",
                    plc.family.as_str()
                ),
            ));
        }
    }
    seen
}

/// R1：id 格式 `plc<N>.<Name>`、前綴已宣告、全機唯一（tags 與 variables 共用命名空間）。
pub(super) fn check_ids<'a>(
    ids: impl Iterator<Item = (&'static str, &'a str)>,
    plc_ids: &BTreeSet<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (section, id) in ids {
        let loc = format!("{section}[{id}]");
        match split_id(id) {
            None => out.push(Diagnostic::error(
                "R1",
                &loc,
                "id 格式不符，必須是 plc<N>.<Name>，Name 只能使用英數與底線",
            )),
            Some((plc, _)) if !plc_ids.contains(plc) => out.push(Diagnostic::error(
                "R1",
                &loc,
                format!("id 的前綴 `{plc}` 沒有在 machine.plcs 宣告"),
            )),
            Some(_) => {}
        }
        if !seen.insert(id) {
            out.push(Diagnostic::error(
                "R1",
                &loc,
                "id 重複（tags 與 variables 共用命名空間，全機唯一）",
            ));
        }
    }
}
```

`machine-model/src/rules/units.rs`：

```rust
//! 單元實例規則：R25（variables 不得有 address）、R26（單元實例 id）、
//! R28（type、type_version 與 bind 的語法）。
//!
//! 引用是否存在、單親、無環、類型符合性需要類型目錄或整張圖，屬 U 規則（Plan 2）。

use std::collections::BTreeSet;

use crate::diag::Diagnostic;
use crate::schema::{BindRef, MachineModel, UNASSIGNED_UNIT, classify_bind, is_valid_name, split_id};

pub(super) fn check(m: &MachineModel, plc_ids: &BTreeSet<&str>, out: &mut Vec<Diagnostic>) {
    for v in &m.variables {
        if v.address.is_some() {
            out.push(Diagnostic::error(
                "R25",
                format!("variables[{}]", v.id),
                "variables 不得有 address；有位址的點位請放在 tags",
            ));
        }
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for u in &m.units {
        let loc = format!("units[{}]", u.id);
        if u.id == UNASSIGNED_UNIT {
            out.push(Diagnostic::error(
                "R26",
                &loc,
                format!("`{}` 是保留的單元 id，不得宣告", u.id),
            ));
        } else if !is_valid_name(&u.id) {
            out.push(Diagnostic::error("R26", &loc, "單元 id 只能使用英數與底線"));
        }
        if !seen.insert(u.id.as_str()) {
            out.push(Diagnostic::error("R26", &loc, "單元 id 重複"));
        }
        if !is_valid_name(&u.ty) {
            out.push(Diagnostic::error(
                "R28",
                &loc,
                format!("type `{}` 只能使用英數與底線（Unit 類型檔 id）", u.ty),
            ));
        }
        if u.type_version == 0 {
            out.push(Diagnostic::error(
                "R28",
                &loc,
                "type_version 必須是正整數（與類型檔的 version 相同）",
            ));
        }
        for (pin, value) in &u.bind.0 {
            if !is_valid_name(pin) {
                out.push(Diagnostic::error(
                    "R28",
                    &loc,
                    format!("bind 的鍵 `{pin}` 只能使用英數與底線"),
                ));
            }
            let Some(value) = value else {
                continue; // null：刻意不接
            };
            match classify_bind(value) {
                None => out.push(Diagnostic::error(
                    "R28",
                    &loc,
                    format!(
                        "bind.{pin} 的值 `{value}` 格式不符，必須是 null、單元 id，或 plc<N>.<Name> 的 tag id"
                    ),
                )),
                Some(BindRef::Tag(id)) => {
                    if let Some((plc, _)) = split_id(id) {
                        if !plc_ids.contains(plc) {
                            out.push(Diagnostic::error(
                                "R28",
                                &loc,
                                format!(
                                    "bind.{pin} 的 tag `{id}` 所屬的 PLC `{plc}` 沒有在 machine.plcs 宣告"
                                ),
                            ));
                        }
                    }
                }
                Some(BindRef::Unit(_)) => {}
            }
        }
    }
}
```

`machine-model/src/rules/mod.rs`（完整取代）：

```rust
//! 載入安全規則（規格 §8.3）。PlcDataService 載入與命令列工具共用同一份。
//!
//! R22（未知欄位）與 R27 的解析階段檢查在 [`crate::parse`]；其餘規則在這裡。
//! 引用是否存在、單親、無環、類型符合性需要類型目錄，屬 U 規則（Plan 2）。

mod structure;
mod units;

pub(crate) use structure::version_error;

use crate::SCHEMA_VERSION;
use crate::diag::Diagnostic;
use crate::schema::MachineModel;

/// 執行全部載入安全規則。順序固定：版本、機台與 PLC、id、單元、欄位值、位址。
pub fn run(model: &MachineModel) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if model.schema_version != SCHEMA_VERSION {
        out.push(version_error(model.schema_version, SCHEMA_VERSION));
    }
    let plc_ids = structure::check_machine_and_plcs(&model.machine, &mut out);
    let ids = model
        .tags
        .iter()
        .map(|t| ("tags", t.id.as_str()))
        .chain(model.variables.iter().map(|v| ("variables", v.id.as_str())));
    structure::check_ids(ids, &plc_ids, &mut out);
    units::check(model, &plc_ids, &mut out);
    out
}
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。注意 `r21_machine_and_plc_ids` 中 `id: ''`（空字串）要得到恰好一個 `R21`；`plc01` 也是。若某筆多出別的規則編號，回報實際結果。

- [x] **Step 5: Checkpoint**

Run: `cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 乾淨（`fmt` 先 `cargo fmt -p machine-model` 重排 import 與長行）。

---

### Task 5: 欄位值規則（R2、R5、R6、R9、R17）

**Files:**
- Create: `machine-model/src/rules/values.rs`, `machine-model/tests/values.rs`
- Modify: `machine-model/src/rules/mod.rs`

**Interfaces:**
- Consumes: `schema::{MachineModel, Scalar, Tag, Variable}`、`types::{DataType, ScalarType}`（`DataType::parse(&str) -> Result<DataType, E: Display>`，`ty.array`、`ty.scalar`、`ty.int_bounds() -> Option<(i64, i64)>`）。
- Produces: `values::check(&MachineModel, &mut Vec<Diagnostic>)`。

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/values.rs`：

```rust
mod common;

use common::{codes, doc, tag_codes};

#[test]
fn r2_types() {
    for ty in ["int", "string:0", "int16[501]", "string:4[2]", "Bool"] {
        let entry = format!("{{id: plc1.A, address: D1, type: '{ty}'}}");
        assert_eq!(tag_codes(&[&entry]), ["R2"], "{ty}");
    }
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: 'int16[10]'}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r2_applies_to_variables() {
    let body = "variables:\n  - {id: plc1.V, type: nonsense}\n";
    assert_eq!(codes(&doc(body)), ["R2"]);
}

#[test]
fn r5_min_must_be_below_max() {
    for (min, max) in [(5, 5), (6, 5)] {
        let entry = format!("{{id: plc1.A, address: D1, type: int16, min: {min}, max: {max}}}");
        assert_eq!(tag_codes(&[&entry]), ["R5"], "{min}/{max}");
    }
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, min: 4, max: 5}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r6_default_must_fit_type_and_range() {
    let bad = [
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: 11}",
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: -1}",
        "{id: plc1.A, address: D1, type: int16, default: 40000}",
        "{id: plc1.A, address: D1, type: int16, default: true}",
        "{id: plc1.A, address: D1, type: int16, default: 1.5}",
        "{id: plc1.A, address: M1, type: bool, default: 1}",
        "{id: plc1.A, address: D1, type: 'int16[2]', default: 1}",
        "{id: plc1.A, address: D1, type: 'string:3', default: abcd}",
        "{id: plc1.A, address: D1, type: float32, min: 0.0, max: 1.0, default: 1.5}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 0.25}",
        "{id: plc1.A, address: D1, type: uint16, default: -1}",
        "{id: plc1.A, address: D1, type: uint32, default: -1}",
        "{id: plc1.A, address: D1, type: 'string:3', default: 'éé'}",
    ];
    for c in bad {
        assert_eq!(tag_codes(&[c]), ["R6"], "{c}");
    }
    let ok = [
        "{id: plc1.A, address: D1, type: int16, min: 0, max: 10, default: 10}",
        "{id: plc1.A, address: D2, type: float32, default: 2}",
        "{id: plc1.A, address: D4, type: float32, default: 0.5}",
        "{id: plc1.A, address: M1, type: bool, default: true}",
        "{id: plc1.A, address: D1, type: 'string:3', default: abc}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 0.5}",
        "{id: plc1.A, address: D1, type: float32, min: 0.5, max: 1.0, default: 1.0}",
        "{id: plc1.A, address: D1, type: 'string:3', default: 'é'}",
        "{id: plc1.A, address: D1, type: uint16, default: 65535}",
        "{id: plc1.A, address: D1, type: float64, default: 1.5}",
    ];
    for c in ok {
        assert_eq!(tag_codes(&[c]), Vec::<&str>::new(), "{c}");
    }
}

#[test]
fn r6_applies_to_variables_too() {
    let body = "variables:\n  - {id: plc1.V, type: int16, default: 40000}\n";
    assert_eq!(codes(&doc(body)), ["R6"]);
}

#[test]
fn r9_scale_zero() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, scale: 0}"]),
        ["R9"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D1, type: int16, scale: 0.1}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r17_address_required() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: '', type: bool}"]),
        ["R17"]
    );
}
```

- [x] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test values`
Expected: FAIL（尚無欄位值規則）。

- [x] **Step 3: 寫實作**

`machine-model/src/rules/values.rs`：

```rust
//! 欄位值規則：R2、R5、R6、R9、R17。
//!
//! v2 的 Tag 沒有 `writable`／`write_permission`，所以沒有 R3、R4、R23。

use std::cmp::Ordering;

use crate::diag::Diagnostic;
use crate::schema::{MachineModel, Scalar, Tag, Variable};
use crate::types::{DataType, ScalarType};

pub(super) fn check(m: &MachineModel, out: &mut Vec<Diagnostic>) {
    for t in &m.tags {
        check_tag(t, out);
    }
    for v in &m.variables {
        check_variable(v, out);
    }
}

fn parse_type(ty: &str, loc: &str, out: &mut Vec<Diagnostic>) -> Option<DataType> {
    match DataType::parse(ty) {
        Ok(t) => Some(t),
        Err(e) => {
            out.push(Diagnostic::error("R2", loc, format!("type `{ty}`：{e}")));
            None
        }
    }
}

fn check_tag(t: &Tag, out: &mut Vec<Diagnostic>) {
    let loc = format!("tags[{}]", t.id);

    if t.address.is_empty() {
        out.push(Diagnostic::error("R17", &loc, "tag 必須有 address"));
    }
    let ty = parse_type(&t.ty, &loc, out);

    if let (Some(min), Some(max)) = (t.min, t.max) {
        if min.partial_cmp(&max) != Some(Ordering::Less) {
            out.push(Diagnostic::error(
                "R5",
                &loc,
                format!("min ({min}) 必須小於 max ({max})"),
            ));
        }
    }

    #[allow(clippy::float_cmp)]
    if t.scale == Some(0.0) {
        out.push(Diagnostic::error("R9", &loc, "scale 不得為 0"));
    }

    if let (Some(default), Some(ty)) = (&t.default, ty) {
        check_default(&ty, default, t.min, t.max, &loc, out);
    }
}

fn check_variable(v: &Variable, out: &mut Vec<Diagnostic>) {
    let loc = format!("variables[{}]", v.id);
    let ty = parse_type(&v.ty, &loc, out);
    if let (Some(default), Some(ty)) = (&v.default, ty) {
        check_default(&ty, default, None, None, &loc, out);
    }
}

#[allow(clippy::cast_precision_loss)]
fn numeric_value(d: &Scalar) -> Option<f64> {
    match d {
        Scalar::Int(i) => Some(*i as f64),
        Scalar::Float(f) => Some(*f),
        _ => None,
    }
}

fn check_default(
    ty: &DataType,
    default: &Scalar,
    min: Option<f64>,
    max: Option<f64>,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) {
    if ty.array {
        out.push(Diagnostic::error("R6", loc, "陣列型別不支援 default"));
        return;
    }
    let compatible = match (ty.scalar, default) {
        (ScalarType::Bool, Scalar::Bool(_))
        | (ScalarType::Float32 | ScalarType::Float64, Scalar::Int(_) | Scalar::Float(_)) => true,
        (ScalarType::Text(n), Scalar::Text(s)) => s.len() <= n,
        (_, Scalar::Int(i)) => ty
            .int_bounds()
            .is_some_and(|(lo, hi)| (lo..=hi).contains(i)),
        _ => false,
    };
    if !compatible {
        out.push(Diagnostic::error(
            "R6",
            loc,
            format!(
                "default {default:?} 與型別 `{}` 不相容或超出型別範圍",
                describe(ty)
            ),
        ));
        return;
    }
    if let Some(value) = numeric_value(default) {
        let below = min.is_some_and(|lo| value < lo);
        let above = max.is_some_and(|hi| value > hi);
        if below || above {
            out.push(Diagnostic::error(
                "R6",
                loc,
                format!("default {value} 不在 min／max 範圍內"),
            ));
        }
    }
}

fn describe(ty: &DataType) -> String {
    format!("{:?}", ty.scalar)
}
```

`machine-model/src/rules/mod.rs`：加入 `mod values;`（置於 `mod units;` 之後），並在 `run` 內 `units::check(...)` 之後加：

```rust
    values::check(model, &mut out);
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。

- [x] **Step 5: Checkpoint**

Run: `cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 乾淨。若 clippy 不接受語句上的 `#[allow(clippy::float_cmp)]`（屬性放在 `if` 運算式上），改成把整個 `check_tag` 函式加上 `#[allow(clippy::float_cmp)]`，行為不變。

---

### Task 6: 位址規則（R8、R18、R19、R24）

**Files:**
- Create: `machine-model/src/rules/addresses.rs`, `machine-model/tests/addresses.rs`
- Modify: `machine-model/src/rules/mod.rs`

**Interfaces:**
- Consumes: `address::{self, Address}`（`address::parse(family, &str)`）、`family::{ClassProfile, Width}`、`schema::{MachineModel, Plc, Tag, split_id}`、`types::DataType`。
- Produces: `addresses::check(&MachineModel, &mut Vec<Diagnostic>)`。**沒有** R20：v2 的 Tag 沒有 `write_permission`；「操作腳不得綁唯讀位址類別」屬 U13（Plan 2），它會用 `ClassProfile.read_only`。

- [x] **Step 1: 寫失敗的測試**

`machine-model/tests/addresses.rs`：

```rust
mod common;

use common::{codes, doc, tag_codes, tags};

#[test]
fn r18_address_syntax_and_range() {
    for addr in ["X008", "X2000", "D8000", "Q1", "D", "R10.16", "M10.1"] {
        let entry = format!("{{id: plc1.A, address: '{addr}', type: bool}}");
        assert!(tag_codes(&[&entry]).contains(&"R18"), "{addr}");
    }
    // D 與 R 接受位元後綴
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'D10.1', type: bool}"]),
        Vec::<&str>::new()
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'D10.1', type: int16}"]),
        ["R19"]
    );
    // 三菱沒有 D 類別（以家族表為準）
    assert!(tag_codes(&["{id: plc2.A, address: D100, type: int16}"]).contains(&"R18"));
}

#[test]
fn r18_span_must_stay_inside_the_class() {
    assert!(tag_codes(&["{id: plc1.A, address: D7999, type: int32}"]).contains(&"R18"));
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D7998, type: int32}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r19_type_must_fit_address_class() {
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: M10, type: int16}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: D10, type: bool}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: R10.3, type: int16}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: 'R10.3', type: 'bool[2]'}"]),
        ["R19"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.A, address: R10.3, type: bool}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r24_polarity_only_on_boolean_inputs() {
    assert_eq!(
        tag_codes(&["{id: plc1.Y1, address: Y1, type: bool, polarity: NO}"]),
        ["R24"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.D1, address: D1, type: int16, polarity: NC}"]),
        ["R24"]
    );
    assert_eq!(
        tag_codes(&["{id: plc1.X1, address: X1, type: bool, polarity: NC}"]),
        Vec::<&str>::new()
    );
}

#[test]
fn r8_overlap() {
    // 同一位址的別名（X010 與 X10 是同一點）
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: X010, type: bool}",
            "{id: plc1.B, address: X10, type: bool}",
        ]),
        ["R8"]
    );
    // int32 佔 D100、D101
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: int32}",
            "{id: plc1.B, address: D101, type: int16}",
        ]),
        ["R8"]
    );
    // 陣列與字串
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: 'int16[4]'}",
            "{id: plc1.B, address: D103, type: 'string:4'}",
        ]),
        ["R8"]
    );
    // 位元後綴：同一位元重疊
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: R10.3, type: bool}",
            "{id: plc1.B, address: R10.3, type: bool}",
        ]),
        ["R8"]
    );
    // 位元與涵蓋該字組的 tag 重疊
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: R10, type: int16}",
            "{id: plc1.B, address: R10.3, type: bool}",
        ]),
        ["R8"]
    );
    // 不重疊：相鄰位址、不同位元、不同 PLC 的相同位址
    assert_eq!(
        tag_codes(&[
            "{id: plc1.A, address: D100, type: int16}",
            "{id: plc1.B, address: D101, type: int16}",
            "{id: plc1.C, address: R10.3, type: bool}",
            "{id: plc1.E, address: R10.4, type: bool}",
            "{id: plc2.A, address: X007, type: bool}",
            "{id: plc1.F, address: X007, type: bool}",
        ]),
        Vec::<&str>::new()
    );
}

#[test]
fn mitsubishi_octal_io_loads() {
    let body = tags(&[
        "{id: plc2.X007, address: X007, type: bool}",
        "{id: plc2.X010, address: X010, type: bool}",
        "{id: plc2.Y017, address: Y017, type: bool}",
    ]);
    assert_eq!(codes(&doc(&body)), Vec::<&str>::new());
}

#[test]
fn v2_has_no_r20_the_writable_concept_does_not_exist() {
    // X 類別本來就不可寫；v2 的 Tag 沒有 writable，所以什麼都不會被擋，也沒有 R20。
    assert_eq!(
        tag_codes(&["{id: plc1.X1, address: X1, type: bool}"]),
        Vec::<&str>::new()
    );
}
```

- [x] **Step 2: 確認測試失敗**

Run: `cargo test -p machine-model --test addresses`
Expected: FAIL（`r18`／`r19`／`r24`／`r8` 測試期待的診斷沒出現）。

- [x] **Step 3: 寫實作**

`machine-model/src/rules/addresses.rs`：

```rust
//! 位址規則：R8、R18、R19、R24。
//!
//! v2 沒有 R20：Tag 沒有 `write_permission`。「操作腳不得綁唯讀位址類別」是 U13（Plan 2）。

use std::collections::{BTreeMap, BTreeSet};

use crate::address::{self, Address};
use crate::diag::Diagnostic;
use crate::family::{ClassProfile, Width};
use crate::schema::{MachineModel, Plc, Tag, split_id};
use crate::types::DataType;

/// 一個已通過 R18、R19 的 tag 在某個位址類別上佔用的空間。
enum Slot<'a> {
    /// 半開區間 `[start, end)`，單位是該類別的位址（位元類為位元，字組類為字組）。
    Span { start: u32, end: u32, tag: &'a Tag },
    /// 字組類別中的單一位元（`R10.3`）。
    Bit { word: u32, bit: u8, tag: &'a Tag },
}

pub(super) fn check(m: &MachineModel, out: &mut Vec<Diagnostic>) {
    check_tags(&m.machine.plcs, &m.tags, out);
}

fn check_tags(plcs: &[Plc], tags: &[Tag], out: &mut Vec<Diagnostic>) {
    let plcs: BTreeMap<&str, &Plc> = plcs.iter().map(|p| (p.id.as_str(), p)).collect();
    // (plc id, 類別名稱) → 佔用
    let mut placed: BTreeMap<(&str, &'static str), Vec<Slot<'_>>> = BTreeMap::new();

    for t in tags {
        let Some((plc_id, _)) = split_id(&t.id) else {
            continue;
        };
        let Some(plc) = plcs.get(plc_id) else {
            continue;
        };
        if t.address.is_empty() {
            continue;
        }
        let loc = format!("tags[{}]", t.id);
        let addr = match address::parse(plc.family, &t.address) {
            Ok(a) => a,
            Err(e) => {
                out.push(Diagnostic::error(
                    "R18",
                    &loc,
                    format!("address `{}`：{e}", t.address),
                ));
                continue;
            }
        };
        let class = addr.class;

        let Ok(ty) = DataType::parse(&t.ty) else {
            continue; // R2 已回報
        };
        check_polarity(t, &ty, class, &loc, out);
        if let Err(msg) = type_fits(class, &addr, &ty) {
            out.push(Diagnostic::error("R19", &loc, msg));
            continue;
        }
        let Some(slot) = make_slot(t, class, &addr, &ty, &loc, out) else {
            continue;
        };
        placed
            .entry((plc_id, addr.class.name))
            .or_default()
            .push(slot);
    }

    for ((plc_id, class_name), slots) in &placed {
        check_overlap(plc_id, class_name, slots, out);
    }
}

fn check_polarity(
    t: &Tag,
    ty: &DataType,
    class: &ClassProfile,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) {
    if t.polarity.is_some() && !(ty.is_bool() && !ty.array && class.read_only) {
        out.push(Diagnostic::error(
            "R24",
            loc,
            "polarity 只能用在 bool 型別的輸入點（唯讀輸入位址類別）",
        ));
    }
}

fn type_fits(class: &ClassProfile, addr: &Address, ty: &DataType) -> Result<(), String> {
    match class.width {
        Width::Bit => {
            if ty.is_bool() {
                Ok(())
            } else {
                Err(format!("{} 類別是位元位址，只能使用 bool", class.name))
            }
        }
        Width::Word => match (ty.is_bool(), addr.bit) {
            (true, None) => Err(format!(
                "字組類別 {} 上的 bool 必須使用位元後綴（例如 {}10.3）",
                class.name, class.name
            )),
            (true, Some(_)) if ty.array => Err("位元位址不支援 bool 陣列".to_owned()),
            (false, Some(_)) => Err("位元後綴只能搭配 bool".to_owned()),
            _ => Ok(()),
        },
    }
}

fn make_slot<'a>(
    t: &'a Tag,
    class: &ClassProfile,
    addr: &Address,
    ty: &DataType,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<Slot<'a>> {
    if let Some(bit) = addr.bit {
        return Some(Slot::Bit {
            word: addr.number,
            bit,
            tag: t,
        });
    }
    let span = match class.width {
        Width::Bit => ty.len,
        Width::Word => ty.words(),
    };
    let span = u32::try_from(span).unwrap_or(u32::MAX);
    let end = addr.number.saturating_add(span);
    if end > class.max.saturating_add(1) {
        out.push(Diagnostic::error(
            "R18",
            loc,
            format!(
                "{} 類別從 {} 起佔用 {span} 個位址，超過上限 {}",
                class.name, addr.number, class.max
            ),
        ));
        return None;
    }
    Some(Slot::Span {
        start: addr.number,
        end,
        tag: t,
    })
}

fn check_overlap(plc_id: &str, class_name: &str, slots: &[Slot<'_>], out: &mut Vec<Diagnostic>) {
    let mut spans: Vec<(u32, u32, &Tag)> = slots
        .iter()
        .filter_map(|s| match s {
            Slot::Span { start, end, tag } => Some((*start, *end, *tag)),
            Slot::Bit { .. } => None,
        })
        .collect();
    spans.sort_by_key(|(start, _, tag)| (*start, tag.id.as_str()));

    let mut holder: Option<(u32, &Tag)> = None; // (目前最遠的 end, 持有者)
    for (start, end, tag) in &spans {
        if let Some((max_end, owner)) = holder {
            if *start < max_end {
                out.push(overlap(plc_id, class_name, tag, owner));
            }
        }
        if holder.is_none_or(|(max_end, _)| *end > max_end) {
            holder = Some((*end, tag));
        }
    }

    let mut seen_bits: BTreeMap<(u32, u8), &Tag> = BTreeMap::new();
    let mut bit_slots: Vec<(u32, u8, &Tag)> = slots
        .iter()
        .filter_map(|s| match s {
            Slot::Bit { word, bit, tag } => Some((*word, *bit, *tag)),
            Slot::Span { .. } => None,
        })
        .collect();
    bit_slots.sort_by_key(|(w, b, tag)| (*w, *b, tag.id.as_str()));
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    for (word, bit, tag) in bit_slots {
        if let Some(other) = seen_bits.get(&(word, bit)) {
            out.push(overlap(plc_id, class_name, tag, other));
            continue;
        }
        seen_bits.insert((word, bit), tag);
        if let Some((_, _, owner)) = spans.iter().find(|(s, e, _)| (*s..*e).contains(&word)) {
            if reported.insert(tag.id.as_str()) {
                out.push(overlap(plc_id, class_name, tag, owner));
            }
        }
    }
}

fn overlap(plc_id: &str, class_name: &str, tag: &Tag, other: &Tag) -> Diagnostic {
    Diagnostic::error(
        "R8",
        format!("tags[{}]", tag.id),
        format!(
            "{plc_id} 的 {class_name} 類別位址 `{}` 與 `{}`（{}）重疊",
            tag.address, other.id, other.address
        ),
    )
}
```

`machine-model/src/rules/mod.rs`：加入 `mod addresses;`（置於最前，維持字母序），並在 `values::check(...)` 之後加：

```rust
    addresses::check(model, &mut out);
```

- [x] **Step 4: 執行測試確認通過**

Run: `cargo test -p machine-model`
Expected: PASS。若 `r8_overlap` 的「不重疊」情境出現 `R8`，先檢查是不是自己抄錯位址；`plc1.F`（X007）與 `plc2.A`（X007）在不同 PLC，不得互相重疊。

- [x] **Step 5: Checkpoint**

Run: `cargo clippy -p machine-model --all-targets -- -D warnings && cargo fmt -p machine-model --check`
Expected: 乾淨。

---

### Task 7: 完整範例、依賴規則與收尾

**Files:**
- Create: `machine-model/tests/fixtures/valid_full.yaml`, `machine-model/tests/fixture.rs`, `machine-model/tests/dependency_rules.rs`

**Interfaces:**
- Consumes: `machine_model::{parse, validate, load}`。
- Produces: 無（只有測試）。

- [x] **Step 1: 寫 fixture 與測試**

`machine-model/tests/fixtures/valid_full.yaml`（涵蓋全部區塊，所有規則都通過）：

```yaml
schema_version: 2
machine:
  id: demo
  title: 示範機台
  plcs:
    - { id: plc1, family: inovance_easy }
    - { id: plc2, family: mitsubishi_fx3g, online: false }
units:
  - id: cyl1
    type: cylinder_double
    type_version: 1
    bind:
      lim_ext: plc1.X_Cyl1_Ext_LS
      lim_ret: null
      valve_ext: plc1.Y_Cyl1_Ext
      man_ext: plc1.M_Cyl1_ManExt
  - id: machine
    type: machine_ctrl
    type_version: 1
    bind:
      stn: cyl1
tags:
  - { id: plc1.X_Cyl1_Ext_LS, address: X010, type: bool, polarity: NO, safety_monitor: false }
  - { id: plc1.Y_Cyl1_Ext, address: Y10, type: bool }
  - { id: plc1.M_Cyl1_ManExt, address: M100, type: bool }
  - { id: plc1.D_ExtTime, address: D100, type: int16, min: 0, max: 30000, default: 3000, unit: ms, scan: slow }
  - { id: plc1.D_Temp, address: D200, type: float32, scale: 0.1, offset: 0, deadband: 0.5, unit: degC, role: process_value }
  - { id: plc1.R_Bit, address: R10.3, type: bool, retain: true, readback: true, spare: true }
  - { id: plc1.D_Name, address: D300, type: 'string:8', default: abc, ext: { inovance: { seq: 1 } } }
  - { id: plc2.X007, address: X007, type: bool }
variables:
  - { id: plc1.V_Mode, type: int16, default: 0, retain: true }
```

`machine-model/tests/fixture.rs`：

```rust
mod common;

use machine_model::{load, parse, validate};

const FULL: &str = include_str!("fixtures/valid_full.yaml");

#[test]
fn full_fixture_loads_with_no_diagnostics() {
    let model = parse(FULL).expect("parse");
    let diags = validate(&model);
    assert!(diags.is_empty(), "{diags:#?}");
    assert_eq!(model.tags.len(), 8);
    assert_eq!(model.variables.len(), 1);
    assert_eq!(model.units.len(), 2);
    assert_eq!(model.units[0].type_version, 1);
}

#[test]
fn full_fixture_survives_crlf_and_bom() {
    let text = format!("\u{feff}{}", FULL.replace('\n', "\r\n"));
    assert!(load(&text).is_ok());
}

#[test]
fn removing_a_required_field_from_the_fixture_is_reported_not_ignored() {
    // 每個 tag 都需要 type；拿掉一個要得到 PARSE，而不是被當成預設值
    let broken = FULL.replace("{ id: plc1.Y_Cyl1_Ext, address: Y10, type: bool }", "{ id: plc1.Y_Cyl1_Ext, address: Y10 }");
    assert_ne!(broken, FULL);
    let diag = parse(&broken).unwrap_err();
    assert_eq!(diag.code, "PARSE");
}
```

`machine-model/tests/dependency_rules.rs`：

```rust
//! 函式庫本體的依賴限制（規格 §8.5）：只有 serde 與 serde_yaml_ng 的樹。

use std::process::Command;

const BANNED: &[&str] = &[
    "tokio",
    "axum",
    "reqwest",
    "hyper",
    "clap",
    "jsonschema",
];

#[test]
fn library_dependency_tree_has_no_runtime_or_network_stacks() {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "machine-model",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--color",
            "never",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("無法執行 cargo tree");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("cargo tree 輸出不是 UTF-8");
    let names: Vec<&str> = tree
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert!(names.contains(&"serde"), "{tree}");
    for banned in BANNED {
        assert!(!names.contains(banned), "依賴樹含有 {banned}：\n{tree}");
    }
}
```

- [x] **Step 2: 執行測試**

Run: `cargo test -p machine-model`
Expected: PASS。`fixture.rs` 的 `mod common;` 若造成 `unused` 警告，刪掉那行（本檔沒用到共用工具）。若 `valid_full.yaml` 有診斷，**不要**改規則；用 `cargo test -p machine-model --test fixture -- --nocapture` 看診斷，回報是 fixture 寫錯（例如 `Y10` 位址對該家族無效）還是規則有誤，並以家族表為準修 fixture。

- [x] **Step 3: 全專案驗證**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: 全過。另確認不存在已廢除規則編號：

Run: `grep -rnE '"R(3|4|20|23)"' machine-model/src machine-model/tests || echo "OK: 沒有 R3/R4/R20/R23"`
Expected: 輸出 `OK: 沒有 R3/R4/R20/R23`（`diag.rs` 內建測試若出現 `R3` 字樣，把該測試的字串改成 `R1`，期望字串同步改）。

- [x] **Step 4: Checkpoint（回報給控制端）**

回報：測試總數、clippy／fmt 結果、`cargo tree -p machine-model --edges normal` 的第一層依賴，以及對 spec 的任何偏離。**不要提交。**

---

## Self-Review（作者填寫，執行者不需處理）

- **Spec coverage（§12 第 1 項前半）**：v2 型別（Task 2）、`type_version`（Task 2、4）、R1／R2／R5／R6／R8／R9／R17～R19／R21／R22／R24～R28（Task 3～6）、家族與位址表（Task 1）、依賴限制（Task 7）。**尚未涵蓋、屬 Plan 2**：前綴表、類型檔型別與目錄、內嵌標準類型與 `type_version` 比對（U2）、U1／U3～U19、推導（`role`、`write_candidate`）、解析結果資料結構、標準類型內容雜湊釘版本。
- **規則編號一致**：R3、R4、R20、R23 不存在；R20 的意圖交給 U13（Plan 2）。
- **型別一致**：`UnitInstance.type_version: u32`；`classify_bind`／`BindRef` 在 Task 2 定義、Task 4 使用；`rules::version_error` 在 Task 3 定義、`lib.rs` 與 `rules::run` 使用。
- **風險**：Task 6 的「R10 int16 + R10.3 bool 重疊」測試依 `check_overlap` 邏輯推導，舊測試檔該段被截斷，未逐字比對；失敗時以邏輯為準回報。
