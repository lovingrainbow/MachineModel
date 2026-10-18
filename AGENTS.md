# Agent Instructions

MachineModel：`machine.yaml` v2（Unit／Tag 模型）的函式庫與工具。規格是根目錄的 `2026-10-16-unit-tag-model-design.md`；欄位與規則要改，先改規格。計畫放 `plans/`。

## 規則

- `machine-model` 函式庫本體只依賴 `serde`、`serde_yaml_ng` 與同層的 `plc-common`（`plc-common` 只依賴 `serde`）。不得依賴 `tokio`、`axum`、`reqwest`、`hyper`、`clap`；`jsonschema` 只能在 `schema` feature 之後（`default-features = false`）。由 `tests/dependency_rules.rs` 把關。
- 讀取端拒絕未知欄位，擴充只能走 `ext`。Tag 沒有 `unit_id`、`writable`、`write_permission`。
- 診斷訊息用繁體中文，並帶規則編號（`R…`、`U…`、`SC…`）。
- 家族的位址類別、範圍、可用協定是 `plc-common/src/family.rs` 的資料表（`machine-model` 以 `machine_model::family` 重新匯出）。`plc-common` 與 MachineModel 並排放在 `projects/` 之下（`../plc-common`），MachineModel 需要它才能編譯。未經實機驗證的範圍要在註解標明。
- 不使用 let-chains（`rust-version = "1.85"`）。
- 只有人確認過的 Unit 類型才能進標準目錄；AI 不能自創。
- 標準 Unit 類型放 `standards/unit-types/<id>.yaml`，並加進 `machine-model/src/standard.rs` 的 `STANDARD` 與 `standards/unit-types.pins`。改類型檔內容必須調高 `version`（`tests/standard_pins.rs` 把關）。

## 驗證

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
