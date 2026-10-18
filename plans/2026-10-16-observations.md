# 觀察項目（結案時記錄，不阻擋結案）

記錄日期：2026-10-16。這些項目不影響 v0.1 的範圍（規格 §12 第 1～3 項），以下逐項記錄現況與後續動作。

## 1. 測試偶發崩潰（環境相關，未查明根因）

- **現象**：本次對話中，`cargo test` 多次在不同的測試 binary（`catalog`、`values`、`analyze_pins`、`parse`）出現 SIGSEGV，或 ld.so 報 relocation 錯誤；另有一次 `getrandom` 的 panic。
- **已排除**：乾淨重建（`cargo clean`）後，連續執行多次全部通過；單獨執行崩潰的 binary 也能通過。
- **可能原因**：WSL2 在 `/mnt/c` 上建置，且當時記憶體壓力高（實測約 2.5 GB swap 使用中）。未證實。
- **若再發生**：先確認記憶體與 swap；再考慮把 `target/` 移到 Linux 檔案系統（例如 `CARGO_TARGET_DIR`）。

## 2. 計畫 2 的紅燈步驟未勾選

- 計畫 2 共 8 個「確認測試失敗」步驟原先未勾選（見計畫檔頭的勾選說明）。紅燈結果沒有留在 git 歷史中，無法回溯驗證。
- **2026-10-18 已補跑並勾選**：在暫存副本中還原各模組至實作前狀態後執行，8 項全部如預期失敗；未變異的基準全部通過。這是重建的紅燈，不是原始執行順序。

## 3. 規格中「P4」的指稱不一致

- §11 #6 把 FB 的 ST ↔ 類型檔一致性檢查「延到 P4」。
- §12 的範圍說明把「ST 產生器」列為 P4 不在範圍。
- §12 第 4 項（PlcDataService 整合）在計畫編號上也屬於第 4 項。
- 三者用字相同，但內容不同。後續開發時需統一用語。

## 4. 待釐清：部分計畫項目可能屬於 AIAutomation

- 使用者指出，計畫勾選框中未勾的部分項目，可能是 AIAutomation 的需求，需要逐項確認。
- 具體項目待確認。

## 5. 規格 §12 第 4～6 項不在本 repo

- 第 4 項（PlcDataService 整合）、第 5 項（貼標機黃金案例）、第 6 項（AIAutomation 遷移）都屬於 AIAutomation 專案，計畫應放在該專案。
- 尚未驗證：`Point` 欄位能否與 `machinemodel dump` 的輸出一一對應。

## 6. 規格範圍外的延後項目（後續開發）

- ST 產生器（規格 §12、§7 的 FB 型匯出）。
- §11 #6：FB 的 ST ↔ 類型檔一致性檢查。
- §11 #7：Unit 陣列（如 `cyl[8]`）。
- §11 #8：`bind` 的常數與運算式（v2 保留、不接受）。
- §11 #5：機台根 `machine` 的完整框架（P6）。

## 7. 工作區與另行開發的工具

- `tools/model-debugger` 是另行開發的應用，會使用 `machine-model` 做驗證測試。已完成，workspace 現在可以建置，`--all-features` 測試、clippy `-D warnings`、fmt 皆通過（2026-10-16）。
- 該工具的快速開啟路徑已由 `valid_full.yaml` 改為 `structural_full.yaml`。
- `tools/model-debugger` 目前尚未納入 git 追蹤，也還沒有提交；`Cargo.lock` 因其相依項（`eframe` 等）而有變更。
- 同一個快速開啟清單中，`tests/fixtures/valid_units.yaml` 的路徑與實際位置（`machine-model/tests/fixtures/valid_units.yaml`）不一致，屬於該工具的問題，未修改。

## 8. 測試用 fixture 的角色

- `machine-model/tests/fixtures/structural_full.yaml`（原 `valid_full.yaml`）只做結構測試，不代表目錄層級有效。
- 目錄層級的有效範例是 `machine-model/tests/fixtures/valid_units.yaml`。

## 9. role 層級的規格與計畫落差（已處理）

- **現象**：計畫 2 寫的 role 優先序有三層（Tag 明確值 ＞ 接腳 role ＞ 前綴推測），但 v0.3 已移除接腳 role（規格 §4.1，提交 `9d92594`）。計畫文字沒有同步更新，讓人誤以為規格缺少 role 優先序。
- **已處理**：規格 §6.1 與 U14 改寫為「Tag 上明確寫的 `role`」，並註明接腳 role 不參與優先序；計畫 2 檔頭加註記，列出受影響的行。
- **影響判斷**：計畫的 role 測試（Task 6 的 U14、Task 7 的 role 測試）只測 Tag 明確值與前綴推測，不受影響；紅燈步驟無法重現的原因與此無關（見第 2 項）。
- **待確認**：這些紅燈步驟的重現尚未逐一實際執行。
