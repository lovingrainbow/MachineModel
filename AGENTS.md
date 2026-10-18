# Agent Instructions

MachineModel：`machine.yaml` v2（Unit／Tag 模型）的函式庫與工具。規格是根目錄的 `2026-10-16-unit-tag-model-design.md`；欄位與規則要改，先改規格。計畫放 `plans/`。

## 規則

- `machine-model` 函式庫本體只依賴 `serde`、`serde_yaml_ng` 與 `plc-common`（`plc-common` 只依賴 `serde`）。不得依賴 `tokio`、`axum`、`reqwest`、`hyper`、`clap`；`jsonschema` 只能在 `schema` feature 之後（`default-features = false`）。由 `tests/dependency_rules.rs` 把關。
- 讀取端拒絕未知欄位，擴充只能走 `ext`。Tag 沒有 `unit_id`、`writable`、`write_permission`。
- 診斷訊息用繁體中文，並帶規則編號（`R…`、`U…`、`SC…`）。
- 家族的位址類別、範圍、可用協定是 `plc-common/src/family.rs` 的資料表（`machine-model` 以 `machine_model::family` 重新匯出）。`plc-common` 以 git tag 依賴（`github.com/lovingrainbow/plc-common`，見 `machine-model/Cargo.toml`）；改它要先在 plc-common 打新 tag，再更新這裡的 tag。專案之間的依賴一律以 git 為主，不再要求並排目錄。DataService 的 `plc-data-service`、`datasvc-demo`、`src-tauri` 依賴本專案的 `machine-model`（machine-model 只有這一份，DataService 不再有自己的複本）；改公開 API 前先確認不會破壞它們。未經實機驗證的範圍要在註解標明。
- 不使用 let-chains（`rust-version = "1.85"`）。
- 只有人確認過的 Unit 類型才能進標準目錄；AI 不能自創。
- 標準 Unit 類型放 `standards/unit-types/<id>.yaml`，並加進 `machine-model/src/standard.rs` 的 `STANDARD` 與 `standards/unit-types.pins`。改類型檔內容必須調高 `version`（`tests/standard_pins.rs` 把關）。

## 驗證

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check
