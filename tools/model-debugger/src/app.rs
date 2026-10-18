use std::time::Instant;

use eframe::egui::{self, Color32, RichText, Stroke};
use machine_model::Checked;
use machine_model::diag::{Diagnostic, Severity};
use machine_model::family::Family;
use machine_model::naming::{self, Prefix};
use machine_model::schema::{Polarity, Scalar};

use crate::io_import::{
    generate_inovance_easy_preset, generate_mitsubishi_preset, parse_delimited, parse_yaml_tags,
};
use crate::model::{ProjectModel, TagConfig};
use crate::type_editor::TypeEditTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Assembly,
    TagExplorer,
    UnitCatalog,
    YamlPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagFilter {
    All,
    ErrorsOnly,
    WarningsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlViewMode {
    RawSource,
    ResolvedDump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportMethod {
    Delimited,
    YamlBlock,
    Preset,
}

pub struct ModelDebuggerApp {
    pub project: ProjectModel,
    pub active_tab: ActiveTab,

    // Selection
    pub selected_unit_id: Option<String>,
    pub selected_catalog_type: String,

    // Tag Explorer Filters
    pub tag_search: String,
    pub tag_prefix_filter: String,
    pub tag_unbound_only: bool,

    // Diagnostics State
    pub validation_result: Result<Checked, Vec<Diagnostic>>,
    pub yaml_text_cache: String,
    pub diagnostics_expanded: bool,
    pub diagnostics_filter: DiagFilter,

    // YAML Preview State
    pub yaml_view_mode: YamlViewMode,
    pub yaml_dump_cache: String,

    // Modals
    pub show_add_unit_modal: bool,
    pub new_unit_id: String,
    pub new_unit_type: String,

    pub show_rename_unit_modal: bool,
    pub rename_unit_old: String,
    pub rename_unit_new: String,

    pub show_quick_tag_modal: bool,
    pub quick_tag_unit_id: String,
    pub quick_tag_slot_name: String,
    pub quick_tag_id: String,
    pub quick_tag_address: String,
    pub quick_tag_type: String,
    pub quick_tag_label: String,

    pub show_import_modal: bool,
    pub import_method: ImportMethod,
    pub import_text: String,
    pub import_preview_tags: Vec<TagConfig>,
    pub import_error: Option<String>,

    pub show_custom_type_modal: bool,
    pub custom_type_source_name: String,
    pub custom_type_yaml: String,
    pub custom_type_error: Option<String>,

    // Unit Type Catalog Management
    pub type_editor_state: crate::type_editor::TypeEditorModalState,
    pub catalog_search_query: String,
    pub show_delete_type_confirm: Option<String>,
    pub show_export_type_modal: bool,
    pub export_type_path: String,
    pub export_type_id: String,
    pub export_type_yaml: String,

    pub show_tag_edit_modal: bool,
    pub editing_tag: Option<TagConfig>,

    // File Management
    pub current_file_path: Option<String>,
    pub show_new_project_modal: bool,
    pub new_project_machine_id: String,
    pub new_project_title: String,
    pub new_project_family: Family,
    pub new_project_template_is_demo: bool,

    pub show_open_file_modal: bool,
    pub open_file_path_input: String,
    pub open_file_error: Option<String>,

    pub show_save_as_modal: bool,
    pub save_as_path_input: String,
    pub save_as_error: Option<String>,

    // UI Scale / Font Zoom
    pub ui_scale: f32,

    // Toast Notification
    pub toast: Option<(String, Color32, Instant)>,
}

impl Default for ModelDebuggerApp {
    fn default() -> Self {
        let project = ProjectModel::demo();
        let (validation_result, yaml_text_cache) = project.validate();
        let yaml_dump_cache = match &validation_result {
            Ok(c) => serde_yaml_ng::to_string(&c.resolved).unwrap_or_default(),
            Err(_) => String::new(),
        };

        Self {
            project,
            active_tab: ActiveTab::Assembly,
            selected_unit_id: Some("cyl1".to_string()),
            selected_catalog_type: "cylinder_double".to_string(),

            tag_search: String::new(),
            tag_prefix_filter: "ALL".to_string(),
            tag_unbound_only: false,

            validation_result,
            yaml_text_cache,
            diagnostics_expanded: false,
            diagnostics_filter: DiagFilter::All,

            yaml_view_mode: YamlViewMode::RawSource,
            yaml_dump_cache,

            show_add_unit_modal: false,
            new_unit_id: String::new(),
            new_unit_type: "cylinder_double".to_string(),

            show_rename_unit_modal: false,
            rename_unit_old: String::new(),
            rename_unit_new: String::new(),

            show_quick_tag_modal: false,
            quick_tag_unit_id: String::new(),
            quick_tag_slot_name: String::new(),
            quick_tag_id: String::new(),
            quick_tag_address: String::new(),
            quick_tag_type: "bool".to_string(),
            quick_tag_label: String::new(),

            show_import_modal: false,
            import_method: ImportMethod::Delimited,
            import_text: String::new(),
            import_preview_tags: Vec::new(),
            import_error: None,

            show_custom_type_modal: false,
            custom_type_source_name: "custom_unit.yaml".to_string(),
            custom_type_yaml: String::new(),
            custom_type_error: None,

            type_editor_state: crate::type_editor::TypeEditorModalState::default(),
            catalog_search_query: String::new(),
            show_delete_type_confirm: None,
            show_export_type_modal: false,
            export_type_path: "custom_type.yaml".to_string(),
            export_type_id: String::new(),
            export_type_yaml: String::new(),

            show_tag_edit_modal: false,
            editing_tag: None,

            current_file_path: None,
            show_new_project_modal: false,
            new_project_machine_id: "machine_01".to_string(),
            new_project_title: "新機台專案".to_string(),
            new_project_family: Family::InovanceEasyTcp,
            new_project_template_is_demo: false,

            show_open_file_modal: false,
            open_file_path_input: "machine.yaml".to_string(),
            open_file_error: None,

            show_save_as_modal: false,
            save_as_path_input: "machine.yaml".to_string(),
            save_as_error: None,

            ui_scale: 1.15,

            toast: None,
        }
    }
}

impl ModelDebuggerApp {
    /// 觸發模型重新驗證並更新快取
    pub fn trigger_validation(&mut self) {
        let (res, yaml) = self.project.validate();
        self.yaml_text_cache = yaml;
        self.yaml_dump_cache = match &res {
            Ok(c) => serde_yaml_ng::to_string(&c.resolved).unwrap_or_default(),
            Err(_) => String::new(),
        };
        self.validation_result = res;
    }

    /// 直接儲存目前專案至目前檔案路徑，若無檔案路徑則開啟「另存新檔」彈窗。
    pub fn save_current_project(&mut self) {
        if let Some(ref path) = self.current_file_path.clone() {
            if let Some(parent) = std::path::Path::new(path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
            {
                let _ = std::fs::create_dir_all(parent);
            }
            let yaml_content = self.project.to_yaml_string();
            match std::fs::write(path, yaml_content) {
                Ok(()) => {
                    self.set_toast(format!("已成功儲存至 `{}`", path), Color32::LIGHT_GREEN);
                }
                Err(e) => {
                    self.set_toast(format!("儲存失敗：{}", e), Color32::LIGHT_RED);
                }
            }
        } else {
            self.save_as_path_input = format!("{}.yaml", self.project.machine_id);
            self.save_as_error = None;
            self.show_save_as_modal = true;
        }
    }

    pub fn set_toast(&mut self, msg: impl Into<String>, color: Color32) {
        self.toast = Some((msg.into(), color, Instant::now()));
    }

    pub fn count_errors_and_warnings(&self) -> (usize, usize) {
        match &self.validation_result {
            Ok(c) => {
                let w = c
                    .warnings
                    .iter()
                    .filter(|d| d.severity == Severity::Warning)
                    .count();
                (0, w)
            }
            Err(diags) => {
                let e = diags
                    .iter()
                    .filter(|d| d.severity == Severity::Error)
                    .count();
                let w = diags
                    .iter()
                    .filter(|d| d.severity == Severity::Warning)
                    .count();
                (e, w)
            }
        }
    }
}

impl eframe::App for ModelDebuggerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(self.ui_scale);
        self.render_top_panel(ctx);
        self.render_bottom_diagnostics(ctx);
        self.render_left_units_panel(ctx);
        self.render_central_tab(ctx);
        self.render_all_modals(ctx);
        self.render_toast_overlay(ctx);
    }
}

// ---------------------------------------------------------------------------
// Top Panel: Header, Machine Settings, Navigation Tabs
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_header_panel")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(Color32::from_rgb(20, 24, 30))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new("🛠️ MachineModel Studio v2")
                            .color(Color32::from_rgb(100, 200, 255))
                            .strong(),
                    );

                    // 檔案狀態標記
                    if let Some(ref path) = self.current_file_path {
                        ui.label(
                            RichText::new(format!("📄 {}", path))
                                .color(Color32::from_rgb(140, 220, 160))
                                .monospace()
                                .small(),
                        );
                    } else {
                        ui.label(
                            RichText::new("📄 [未存檔新專案]")
                                .color(Color32::KHAKI)
                                .small(),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 介面縮放選單 (Zoom factor)
                        egui::ComboBox::from_id_salt("zoom_scale_combo")
                            .selected_text(format!("🔍 縮放: {:.0}%", self.ui_scale * 100.0))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.ui_scale, 1.0, "100% (預設)");
                                ui.selectable_value(&mut self.ui_scale, 1.15, "115% (舒適推薦)");
                                ui.selectable_value(&mut self.ui_scale, 1.25, "125% (清晰放大)");
                                ui.selectable_value(&mut self.ui_scale, 1.40, "140% (高解析大字)");
                                ui.selectable_value(&mut self.ui_scale, 1.60, "160% (特大)");
                            });

                        ui.separator();

                        // 快速動作按鈕
                        if ui
                            .button(RichText::new("🔄 即時驗證").color(Color32::LIGHT_GRAY))
                            .clicked()
                        {
                            self.trigger_validation();
                            self.set_toast("已重新驗證模型規格", Color32::LIGHT_BLUE);
                        }

                        if ui
                            .button(
                                RichText::new("🚀 示範範本")
                                    .color(Color32::from_rgb(180, 220, 120)),
                            )
                            .clicked()
                        {
                            self.project = ProjectModel::demo();
                            self.current_file_path = None;
                            self.selected_unit_id = Some("cyl1".to_string());
                            self.trigger_validation();
                            self.set_toast("已重設為標準示範機台", Color32::LIGHT_GREEN);
                        }

                        if ui
                            .button(RichText::new("📂 載入自訂類型").color(Color32::KHAKI))
                            .clicked()
                        {
                            self.show_custom_type_modal = true;
                            self.custom_type_error = None;
                        }

                        if ui
                            .button(RichText::new("📥 匯入 PLC I/O").color(Color32::LIGHT_BLUE))
                            .clicked()
                        {
                            self.show_import_modal = true;
                            self.import_preview_tags.clear();
                            self.import_error = None;
                        }

                        ui.separator();

                        // 專案檔案操作
                        if ui
                            .button(
                                RichText::new("💾 另存...").color(Color32::from_rgb(180, 240, 180)),
                            )
                            .clicked()
                        {
                            self.save_as_path_input = self
                                .current_file_path
                                .clone()
                                .unwrap_or_else(|| format!("{}.yaml", self.project.machine_id));
                            self.save_as_error = None;
                            self.show_save_as_modal = true;
                        }

                        if ui
                            .button(
                                RichText::new("💾 儲存")
                                    .strong()
                                    .color(Color32::LIGHT_GREEN),
                            )
                            .clicked()
                        {
                            self.save_current_project();
                        }

                        if ui
                            .button(
                                RichText::new("📂 開啟").color(Color32::from_rgb(100, 200, 255)),
                            )
                            .clicked()
                        {
                            self.show_open_file_modal = true;
                            self.open_file_error = None;
                        }

                        if ui
                            .button(RichText::new("📄 新增專案").strong().color(Color32::WHITE))
                            .clicked()
                        {
                            self.new_project_machine_id = "machine_01".to_string();
                            self.new_project_title = "新機台專案".to_string();
                            self.show_new_project_modal = true;
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // 機台與 PLC 設定列 + Tab 選擇器
                ui.horizontal(|ui| {
                    ui.label(RichText::new("機台 ID:").strong());
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.project.machine_id)
                                .return_key(None)
                                .desired_width(100.0),
                        )
                        .changed()
                    {
                        self.trigger_validation();
                    }

                    ui.label(RichText::new("機台標題:").strong());
                    let mut title_buf = self.project.machine_title.clone().unwrap_or_default();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut title_buf)
                                .return_key(None)
                                .desired_width(140.0),
                        )
                        .changed()
                    {
                        self.project.machine_title = if title_buf.trim().is_empty() {
                            None
                        } else {
                            Some(title_buf)
                        };
                        self.trigger_validation();
                    }

                    ui.separator();

                    if let Some(first_plc) = self.project.plcs.first_mut() {
                        ui.label(RichText::new(format!("PLC ({}):", first_plc.id)).strong());
                        let current_family = first_plc.family;
                        egui::ComboBox::from_id_salt("plc_family_combo")
                            .selected_text(current_family.as_str())
                            .show_ui(ui, |ui| {
                                for &f in Family::ALL {
                                    if ui
                                        .selectable_value(&mut first_plc.family, f, f.as_str())
                                        .clicked()
                                    {
                                        // changed
                                    }
                                }
                            });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Tab 切換
                        let tabs = [
                            (ActiveTab::YamlPreview, "📝 YAML 預覽"),
                            (ActiveTab::UnitCatalog, "📚 Unit 類型庫"),
                            (ActiveTab::TagExplorer, "📋 PLC 點表管理"),
                            (ActiveTab::Assembly, "🌲 裝配樹與接腳綁定"),
                        ];
                        for (tab, label) in tabs {
                            let is_active = self.active_tab == tab;
                            let btn = if is_active {
                                egui::Button::new(
                                    RichText::new(label).strong().color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(45, 100, 160))
                            } else {
                                egui::Button::new(RichText::new(label).color(Color32::LIGHT_GRAY))
                            };
                            if ui.add(btn).clicked() {
                                self.active_tab = tab;
                            }
                        }
                    });
                });
            });
    }
}

// ---------------------------------------------------------------------------
// Bottom Panel: Live Diagnostics Drawer
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_bottom_diagnostics(&mut self, ctx: &egui::Context) {
        let (errors, warnings) = self.count_errors_and_warnings();
        let has_issues = errors > 0 || warnings > 0;

        let bg_color = if errors > 0 {
            Color32::from_rgb(60, 20, 20)
        } else if warnings > 0 {
            Color32::from_rgb(50, 45, 20)
        } else {
            Color32::from_rgb(20, 40, 25)
        };

        let panel_id = if self.diagnostics_expanded {
            "bottom_diag_expanded"
        } else {
            "bottom_diag_collapsed"
        };

        let mut panel = egui::TopBottomPanel::bottom(panel_id).frame(
            egui::Frame::side_top_panel(&ctx.style())
                .fill(bg_color)
                .inner_margin(egui::Margin::symmetric(14, 6)),
        );

        if self.diagnostics_expanded {
            panel = panel
                .resizable(true)
                .default_height(320.0)
                .min_height(100.0)
                .max_height(ctx.screen_rect().height() * 0.85);
        } else {
            panel = panel.resizable(false);
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                if errors > 0 {
                    ui.label(
                        RichText::new(format!(
                            "❌ 規格驗證未通過：{} 個錯誤，{} 個警告",
                            errors, warnings
                        ))
                        .strong()
                        .color(Color32::from_rgb(255, 120, 120)),
                    );
                } else if warnings > 0 {
                    ui.label(
                        RichText::new(format!("⚠️ 規格驗證通過，但有 {} 個警告", warnings))
                            .strong()
                            .color(Color32::from_rgb(255, 220, 100)),
                    );
                } else {
                    ui.label(
                        RichText::new(
                            "✅ 規格檢核完全通過（0 錯誤，0 警告，符合 machine.yaml v2 標準）",
                        )
                        .strong()
                        .color(Color32::from_rgb(120, 240, 150)),
                    );
                }

                if self.diagnostics_expanded {
                    ui.label(
                        RichText::new("（↕ 上緣可直接拖曳拉大高度）")
                            .color(Color32::from_rgb(200, 200, 200))
                            .small(),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let toggle_text = if self.diagnostics_expanded {
                        "▼ 收合診斷面板"
                    } else {
                        "▲ 展開診斷明細"
                    };
                    if ui.button(toggle_text).clicked() {
                        self.diagnostics_expanded = !self.diagnostics_expanded;
                    }

                    if has_issues && self.diagnostics_expanded {
                        // 快速複製全部診斷
                        if ui
                            .button(RichText::new("📋 複製診斷文字").small())
                            .clicked()
                        {
                            let all_diags: Vec<&Diagnostic> = match &self.validation_result {
                                Ok(c) => c.warnings.iter().collect(),
                                Err(diags) => diags.iter().collect(),
                            };
                            let mut lines = Vec::new();
                            for d in all_diags {
                                lines.push(format!(
                                    "[{:?}] [{}] {}: {}",
                                    d.severity,
                                    d.code,
                                    d.location.as_deref().unwrap_or("-"),
                                    d.message
                                ));
                            }
                            ui.ctx().copy_text(lines.join("\n"));
                            self.set_toast("已複製所有診斷項目至剪貼簿", Color32::LIGHT_GREEN);
                        }

                        ui.separator();
                        ui.selectable_value(&mut self.diagnostics_filter, DiagFilter::All, "全部");
                        ui.selectable_value(
                            &mut self.diagnostics_filter,
                            DiagFilter::ErrorsOnly,
                            "僅錯誤",
                        );
                        ui.selectable_value(
                            &mut self.diagnostics_filter,
                            DiagFilter::WarningsOnly,
                            "僅警告",
                        );
                    }
                });
            });

            if self.diagnostics_expanded {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("diagnostics_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let all_diags: Vec<&Diagnostic> = match &self.validation_result {
                            Ok(c) => c.warnings.iter().collect(),
                            Err(diags) => diags.iter().collect(),
                        };

                        let filtered: Vec<&&Diagnostic> = all_diags
                            .iter()
                            .filter(|d| match self.diagnostics_filter {
                                DiagFilter::All => true,
                                DiagFilter::ErrorsOnly => d.severity == Severity::Error,
                                DiagFilter::WarningsOnly => d.severity == Severity::Warning,
                            })
                            .collect();

                        if filtered.is_empty() {
                            ui.label(
                                RichText::new("目前篩選條件下沒有任何診斷項目。")
                                    .color(Color32::GRAY),
                            );
                        } else {
                            egui::Grid::new("diag_grid")
                                .num_columns(4)
                                .spacing([14.0, 6.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(RichText::new("層級").strong());
                                    ui.label(RichText::new("規則編號").strong());
                                    ui.label(RichText::new("位置").strong());
                                    ui.label(RichText::new("說明").strong());
                                    ui.end_row();

                                    for d in filtered {
                                        let (badge_txt, badge_color) = match d.severity {
                                            Severity::Error => {
                                                ("錯誤", Color32::from_rgb(255, 90, 90))
                                            }
                                            Severity::Warning => {
                                                ("警告", Color32::from_rgb(255, 200, 60))
                                            }
                                            Severity::Info => {
                                                ("提示", Color32::from_rgb(100, 180, 255))
                                            }
                                        };
                                        ui.label(
                                            RichText::new(badge_txt).color(badge_color).strong(),
                                        );
                                        ui.label(
                                            RichText::new(d.code)
                                                .color(Color32::LIGHT_YELLOW)
                                                .monospace(),
                                        );
                                        ui.label(
                                            RichText::new(d.location.as_deref().unwrap_or("-"))
                                                .color(Color32::LIGHT_GRAY)
                                                .monospace(),
                                        );
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&d.message).color(Color32::WHITE),
                                            )
                                            .wrap(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Left Panel: Unit Hierarchy Tree & CRUD
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_left_units_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_units_dock")
            .resizable(true)
            .default_width(260.0)
            .width_range(200.0..=400.0)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(Color32::from_rgb(24, 27, 33))
                    .inner_margin(egui::Margin::symmetric(10, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("裝配單元 (Units)").size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("共 {} 個", self.project.units.len()))
                                .small()
                                .color(Color32::GRAY),
                        );
                    });
                });

                ui.add_space(8.0);

                // Action buttons: Add / Rename / Delete
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("➕ 新增").color(Color32::LIGHT_GREEN))
                        .clicked()
                    {
                        self.show_add_unit_modal = true;
                        self.new_unit_id = format!("unit_{}", self.project.units.len() + 1);
                        self.new_unit_type = "cylinder_double".to_string();
                    }

                    let has_sel = self.selected_unit_id.is_some();
                    let rename_clicked = ui
                        .add_enabled(
                            has_sel,
                            egui::Button::new(
                                RichText::new("✏️ 重新命名").color(Color32::LIGHT_BLUE),
                            ),
                        )
                        .clicked();
                    if let Some(sel) = self.selected_unit_id.as_ref().filter(|_| rename_clicked) {
                        self.rename_unit_old = sel.clone();
                        self.rename_unit_new = sel.clone();
                        self.show_rename_unit_modal = true;
                    }

                    let delete_clicked = ui
                        .add_enabled(
                            has_sel,
                            egui::Button::new(
                                RichText::new("🗑️ 刪除").color(Color32::from_rgb(255, 100, 100)),
                            ),
                        )
                        .clicked();
                    if let Some(sel) = self.selected_unit_id.clone().filter(|_| delete_clicked) {
                        self.project.delete_unit(&sel);
                        self.selected_unit_id = self.project.units.first().map(|u| u.id.clone());
                        self.trigger_validation();
                        self.set_toast(format!("已刪除 Unit `{}`", sel), Color32::LIGHT_RED);
                    }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // Unit list
                egui::ScrollArea::vertical()
                    .id_salt("unit_list_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let unit_ids: Vec<String> =
                            self.project.units.iter().map(|u| u.id.clone()).collect();

                        for uid in &unit_ids {
                            let unit_opt = self.project.units.iter().find(|u| &u.id == uid);
                            let Some(unit) = unit_opt else { continue };

                            let is_selected =
                                self.selected_unit_id.as_deref() == Some(uid.as_str());

                            // 計算綁定進度
                            let total_slots = unit.bind.len();
                            let bound_slots = unit
                                .bind
                                .values()
                                .filter(|v| v.as_ref().is_some_and(|s| !s.is_empty()))
                                .count();

                            let icon = match unit.ty.as_str() {
                                "machine_ctrl" => "⚙️",
                                "station_load" => "📦",
                                "cylinder_double" => "🦾",
                                _ => "🧩",
                            };

                            let parent_info = self.project.unit_parent(uid);

                            ui.horizontal(|ui| {
                                if parent_info.is_some() {
                                    ui.label(RichText::new("  └").color(Color32::DARK_GRAY));
                                }

                                let label_text = format!("{} {}", icon, uid);
                                let btn = ui.selectable_label(is_selected, label_text);
                                if btn.clicked() {
                                    self.selected_unit_id = Some(uid.clone());
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let badge_col =
                                            if bound_slots == total_slots && total_slots > 0 {
                                                Color32::from_rgb(100, 220, 120)
                                            } else if bound_slots > 0 {
                                                Color32::from_rgb(255, 200, 80)
                                            } else {
                                                Color32::from_rgb(200, 100, 100)
                                            };

                                        ui.label(
                                            RichText::new(format!(
                                                "{}/{}",
                                                bound_slots, total_slots
                                            ))
                                            .color(badge_col)
                                            .small(),
                                        );
                                    },
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "    類型: {} (v{})",
                                        unit.ty, unit.type_version
                                    ))
                                    .color(Color32::GRAY)
                                    .small(),
                                );
                            });
                            ui.add_space(4.0);
                        }

                        if unit_ids.is_empty() {
                            ui.add_space(12.0);
                            ui.label(RichText::new("尚無任何裝配單元。").color(Color32::GRAY));
                            ui.add_space(6.0);
                            if ui
                                .button(
                                    RichText::new("➕ 新增第一個 Unit").color(Color32::LIGHT_GREEN),
                                )
                                .clicked()
                            {
                                self.new_unit_id = "machine".to_string();
                                self.new_unit_type = "machine_ctrl".to_string();
                                self.show_add_unit_modal = true;
                            }
                        }
                    });
            });
    }
}

// ---------------------------------------------------------------------------
// Central Panel: Tabs Router
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_central_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(Color32::from_rgb(16, 18, 22))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| match self.active_tab {
                ActiveTab::Assembly => self.render_assembly_tab(ui),
                ActiveTab::TagExplorer => self.render_tag_explorer_tab(ui),
                ActiveTab::UnitCatalog => self.render_unit_catalog_tab(ui),
                ActiveTab::YamlPreview => self.render_yaml_preview_tab(ui),
            });
    }
}

// ---------------------------------------------------------------------------
// Tab 1: Assembly & Pin Inspector
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_assembly_tab(&mut self, ui: &mut egui::Ui) {
        if self.project.units.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading(
                    RichText::new("📦 目前專案中尚無任何 Unit 裝配單元")
                        .size(20.0)
                        .color(Color32::WHITE),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "在 MachineModel v2 架構中，機台由層級化的 Units（如機台主控、工站、氣壓缸、感測器）組成。\n您可以點擊下方按鈕直接透過 UI 挑選類型模板進行新增，或從「📚 Unit 類型庫」選取：",
                    )
                    .color(Color32::LIGHT_GRAY),
                );
                ui.add_space(16.0);
                if ui
                    .button(
                        RichText::new("➕ 建立第一個裝配單元 (Add Unit)")
                            .size(16.0)
                            .strong()
                            .color(Color32::LIGHT_GREEN),
                    )
                    .clicked()
                {
                    self.new_unit_id = "machine".to_string();
                    self.new_unit_type = "machine_ctrl".to_string();
                    self.show_add_unit_modal = true;
                }
            });
            return;
        }

        let selected_id = match &self.selected_unit_id {
            Some(id) => id.clone(),
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("請由左側單元清單選擇一個 Unit 進行裝配與接腳綁定檢視。")
                            .color(Color32::GRAY)
                            .size(16.0),
                    );
                });
                return;
            }
        };

        let unit_idx = self.project.units.iter().position(|u| u.id == selected_id);
        let Some(idx) = unit_idx else {
            ui.label(RichText::new("選擇的 Unit 不存在").color(Color32::RED));
            return;
        };

        let unit = self.project.units[idx].clone();
        let unit_type_opt = self.project.catalog.get(&unit.ty).cloned();

        // Unit Header Card
        egui::Frame::new()
            .fill(Color32::from_rgb(26, 30, 38))
            .inner_margin(egui::Margin::same(12))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new(format!("裝配單元：{}", unit.id))
                            .color(Color32::WHITE)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("［類型：{} v{}］", unit.ty, unit.type_version))
                            .color(Color32::LIGHT_BLUE),
                    );

                    if let Some(ref ut) = unit_type_opt {
                        ui.label(RichText::new(format!("— {}", ut.title)).color(Color32::KHAKI));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let bound = unit
                            .bind
                            .values()
                            .filter(|v| v.as_ref().is_some_and(|s| !s.is_empty()))
                            .count();
                        ui.label(
                            RichText::new(format!("綁定進度：{}/{}", bound, unit.bind.len()))
                                .color(if bound == unit.bind.len() {
                                    Color32::LIGHT_GREEN
                                } else {
                                    Color32::LIGHT_YELLOW
                                }),
                        );
                    });
                });
            });

        ui.add_space(10.0);

        let Some(unit_type) = unit_type_opt else {
            ui.colored_label(
                Color32::RED,
                format!("類型目錄中未找到類型 `{}`，請確認或匯入類型檔。", unit.ty),
            );
            return;
        };

        egui::ScrollArea::vertical()
            .id_salt("unit_pins_scroll")
            .show(ui, |ui| {
                // Group 1: 現場 I/O (Field In / Field Out)
                let field_in_pins: Vec<(String, _)> = unit_type
                    .field
                    .input
                    .0
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let field_out_pins: Vec<(String, _)> = unit_type
                    .field
                    .output
                    .0
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                if !field_in_pins.is_empty() || !field_out_pins.is_empty() {
                    ui.label(
                        RichText::new("🔌 現場 I/O 實體接腳 (field.in / field.out)")
                            .strong()
                            .size(15.0)
                            .color(Color32::from_rgb(120, 200, 255)),
                    );
                    ui.add_space(4.0);

                    egui::Grid::new("field_pins_grid")
                        .num_columns(6)
                        .spacing([14.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("接腳名稱").strong());
                            ui.label(RichText::new("方向").strong());
                            ui.label(RichText::new("要求").strong());
                            ui.label(RichText::new("型別").strong());
                            ui.label(RichText::new("當前綁定 Tag").strong());
                            ui.label(RichText::new("操作").strong());
                            ui.end_row();

                            // Field In
                            for (name, pin) in field_in_pins {
                                self.render_slot_binding_row(
                                    ui,
                                    &unit.id,
                                    &name,
                                    "輸入 [DI]",
                                    Color32::from_rgb(80, 160, 255),
                                    pin.required.unwrap_or(true),
                                    &pin.ty,
                                    pin.label.as_deref(),
                                    "X",
                                );
                            }

                            // Field Out
                            for (name, pin) in field_out_pins {
                                self.render_slot_binding_row(
                                    ui,
                                    &unit.id,
                                    &name,
                                    "輸出 [DO]",
                                    Color32::from_rgb(220, 140, 255),
                                    pin.required.unwrap_or(true),
                                    &pin.ty,
                                    pin.label.as_deref(),
                                    "Y",
                                );
                            }
                        });

                    ui.add_space(14.0);
                }

                // Group 2: 控制、警報與參數 (ctrl.*)
                let ctrl_pins: Vec<(String, _)> = unit_type
                    .ctrl
                    .0
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if !ctrl_pins.is_empty() {
                    ui.label(
                        RichText::new("🎛️ 控制、狀態、警報與參數插槽 (ctrl.*)")
                            .strong()
                            .size(15.0)
                            .color(Color32::from_rgb(255, 210, 100)),
                    );
                    ui.add_space(4.0);

                    egui::Grid::new("ctrl_pins_grid")
                        .num_columns(6)
                        .spacing([14.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("插槽名稱").strong());
                            ui.label(RichText::new("前綴分類").strong());
                            ui.label(RichText::new("必要性").strong());
                            ui.label(RichText::new("型別 / 屬性").strong());
                            ui.label(RichText::new("當前綁定 Tag").strong());
                            ui.label(RichText::new("操作").strong());
                            ui.end_row();

                            for (name, ctrl) in ctrl_pins {
                                let prefix = machine_model::naming::split(&name).map(|(p, _)| p);
                                let (dir_txt, dir_col, rec_plc_prefix) = match prefix {
                                    Some(Prefix::Cmd) => ("命令 (cmd)", Color32::LIGHT_BLUE, "M"),
                                    Some(Prefix::Man) => ("手動 (man)", Color32::LIGHT_GREEN, "M"),
                                    Some(Prefix::Ena) => ("致能 (ena)", Color32::LIGHT_YELLOW, "M"),
                                    Some(Prefix::At) => ("狀態 (at)", Color32::KHAKI, "M"),
                                    Some(Prefix::Sts) => ("狀態 (sts)", Color32::KHAKI, "M"),
                                    Some(Prefix::Error) => {
                                        ("警報 (error)", Color32::LIGHT_RED, "M")
                                    }
                                    Some(Prefix::Par) => {
                                        ("參數 (par)", Color32::from_rgb(255, 160, 100), "D")
                                    }
                                    Some(Prefix::Count) => {
                                        ("計數 (count)", Color32::LIGHT_BLUE, "D")
                                    }
                                    Some(Prefix::Pv) => ("量測 (pv)", Color32::LIGHT_BLUE, "D"),
                                    Some(Prefix::Msg) => ("訊息 (msg)", Color32::LIGHT_GRAY, "D"),
                                    None => ("控制", Color32::GRAY, "M"),
                                };

                                let ty_str = if let Some(ref t) = ctrl.ty {
                                    t.clone()
                                } else if let Some(p) = prefix {
                                    p.info().default_type.unwrap_or("bool").to_string()
                                } else {
                                    "bool".to_string()
                                };

                                let mut extra_info = ty_str.clone();
                                if let Some(ref m) = ctrl.message {
                                    extra_info = format!("警報訊息: {}", m);
                                } else if let (Some(min), Some(max)) = (ctrl.min, ctrl.max) {
                                    extra_info = format!(
                                        "{}~{}{}",
                                        min,
                                        max,
                                        ctrl.unit.as_deref().unwrap_or("")
                                    );
                                }

                                self.render_slot_binding_row(
                                    ui,
                                    &unit.id,
                                    &name,
                                    dir_txt,
                                    dir_col,
                                    true,
                                    &extra_info,
                                    ctrl.label.as_deref().or(ctrl.message.as_deref()),
                                    rec_plc_prefix,
                                );
                            }
                        });

                    ui.add_space(14.0);
                }

                // Group 3: 子部件組裝 (parts.*)
                let part_slots: Vec<(String, String)> = unit_type
                    .parts
                    .0
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if !part_slots.is_empty() {
                    ui.label(
                        RichText::new("🧩 子部件層級裝配 (parts.*)")
                            .strong()
                            .size(15.0)
                            .color(Color32::from_rgb(140, 240, 180)),
                    );
                    ui.add_space(4.0);

                    egui::Grid::new("parts_grid")
                        .num_columns(5)
                        .spacing([14.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("子部件插槽").strong());
                            ui.label(RichText::new("需求類型").strong());
                            ui.label(RichText::new("當前綁定單元").strong());
                            ui.label(RichText::new("操作").strong());
                            ui.label(RichText::new("備註").strong());
                            ui.end_row();

                            for (part_name, target_type) in part_slots {
                                ui.label(RichText::new(&part_name).monospace().strong());
                                ui.label(RichText::new(&target_type).color(Color32::KHAKI));

                                // 當前綁定的 Unit ID
                                let current_bound = self
                                    .project
                                    .units
                                    .iter()
                                    .find(|u| u.id == unit.id)
                                    .and_then(|u| u.bind.get(&part_name).cloned().flatten());

                                let mut selected_target = current_bound.clone().unwrap_or_default();

                                let mut changed = false;
                                egui::ComboBox::from_id_salt(format!(
                                    "part_{}_{}",
                                    unit.id, part_name
                                ))
                                .selected_text(if selected_target.is_empty() {
                                    "<未綁定>"
                                } else {
                                    &selected_target
                                })
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_value(
                                            &mut selected_target,
                                            String::new(),
                                            "<未綁定 / None>",
                                        )
                                        .clicked()
                                    {
                                        changed = true;
                                    }

                                    // 尋找符合 target_type 的現有 Unit
                                    let candidates: Vec<String> = self
                                        .project
                                        .units
                                        .iter()
                                        .filter(|u| u.ty == target_type && u.id != unit.id)
                                        .map(|u| u.id.clone())
                                        .collect();

                                    for cand in candidates {
                                        if ui
                                            .selectable_value(
                                                &mut selected_target,
                                                cand.clone(),
                                                format!("{} ({})", cand, target_type),
                                            )
                                            .clicked()
                                        {
                                            changed = true;
                                        }
                                    }
                                });

                                if changed {
                                    if let Some(u_mut) =
                                        self.project.units.iter_mut().find(|u| u.id == unit.id)
                                    {
                                        if selected_target.is_empty() {
                                            u_mut.bind.insert(part_name.clone(), None);
                                        } else {
                                            u_mut
                                                .bind
                                                .insert(part_name.clone(), Some(selected_target));
                                        }
                                    }
                                    self.trigger_validation();
                                }

                                // 快速新增並綁定符合該類型的 Unit 按鈕
                                if ui
                                    .button(
                                        RichText::new("➕ 建立新單元").color(Color32::LIGHT_GREEN),
                                    )
                                    .clicked()
                                {
                                    let new_id =
                                        format!("{}_{}", part_name, self.project.units.len());
                                    let ver = self
                                        .project
                                        .catalog
                                        .get(&target_type)
                                        .map(|t| t.version)
                                        .unwrap_or(1);
                                    self.project
                                        .add_unit(new_id.clone(), target_type.clone(), ver);
                                    if let Some(u_mut) =
                                        self.project.units.iter_mut().find(|u| u.id == unit.id)
                                    {
                                        u_mut.bind.insert(part_name.clone(), Some(new_id.clone()));
                                    }
                                    self.trigger_validation();
                                    self.set_toast(
                                        format!(
                                            "已建立 Unit `{}` 並綁定至 `{}`",
                                            new_id, part_name
                                        ),
                                        Color32::LIGHT_GREEN,
                                    );
                                }

                                ui.label(
                                    RichText::new("父子階層綁定").color(Color32::GRAY).small(),
                                );
                                ui.end_row();
                            }
                        });
                }
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_slot_binding_row(
        &mut self,
        ui: &mut egui::Ui,
        unit_id: &str,
        slot_name: &str,
        dir_text: &str,
        dir_color: Color32,
        required: bool,
        type_str: &str,
        label_hint: Option<&str>,
        rec_prefix: &str,
    ) {
        // 1. 接腳名稱
        ui.label(RichText::new(slot_name).monospace().strong());

        // 2. 方向
        ui.label(RichText::new(dir_text).color(dir_color).small());

        // 3. 要求
        if required {
            ui.label(RichText::new("必填").color(Color32::LIGHT_RED).small());
        } else {
            ui.label(RichText::new("可選").color(Color32::GRAY).small());
        }

        // 4. 型別
        ui.label(RichText::new(type_str).monospace().small());

        // 5. 當前綁定 Dropdown
        let current_target = self
            .project
            .units
            .iter()
            .find(|u| u.id == unit_id)
            .and_then(|u| u.bind.get(slot_name).cloned().flatten());

        let mut selected_tag = current_target.clone().unwrap_or_default();
        let mut selection_changed = false;

        egui::ComboBox::from_id_salt(format!("slot_{}_{}", unit_id, slot_name))
            .width(220.0)
            .selected_text(if selected_tag.is_empty() {
                "<未綁定>"
            } else {
                &selected_tag
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut selected_tag, String::new(), "<未綁定 / 清除>")
                    .clicked()
                {
                    selection_changed = true;
                }

                for t in &self.project.tags {
                    let display = if let Some(ref l) = t.label {
                        format!("{} [{}] ({})", t.id, t.address, l)
                    } else {
                        format!("{} [{}]", t.id, t.address)
                    };

                    if ui
                        .selectable_value(&mut selected_tag, t.id.clone(), display)
                        .clicked()
                    {
                        selection_changed = true;
                    }
                }
            });

        if selection_changed {
            if let Some(u_mut) = self.project.units.iter_mut().find(|u| u.id == unit_id) {
                if selected_tag.is_empty() {
                    u_mut.bind.insert(slot_name.to_string(), None);
                } else {
                    u_mut.bind.insert(slot_name.to_string(), Some(selected_tag));
                }
            }
            self.trigger_validation();
        }

        // 6. 快速建立並綁定 Tag 按鈕
        if ui
            .button(RichText::new("⚡ 快速建立 Tag").color(Color32::LIGHT_BLUE))
            .clicked()
        {
            let plc_id = self
                .project
                .plcs
                .first()
                .map(|p| p.id.as_str())
                .unwrap_or("plc1");
            let next_addr = self.project.next_available_address(rec_prefix);
            let suggested_id = format!("{}.{}_{}_{}", plc_id, rec_prefix, unit_id, slot_name);

            self.quick_tag_unit_id = unit_id.to_string();
            self.quick_tag_slot_name = slot_name.to_string();
            self.quick_tag_id = suggested_id;
            self.quick_tag_address = next_addr;
            self.quick_tag_type = if type_str.contains("int") {
                "int16".to_string()
            } else if type_str.contains("float") {
                "float32".to_string()
            } else {
                "bool".to_string()
            };
            self.quick_tag_label = label_hint.unwrap_or(slot_name).to_string();
            self.show_quick_tag_modal = true;
        }

        ui.end_row();
    }
}

// ---------------------------------------------------------------------------
// Tab 2: Tag Explorer & I/O Import
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_tag_explorer_tab(&mut self, ui: &mut egui::Ui) {
        // KPI Cards Summary
        let total_tags = self.project.tags.len();
        let mut bound_count = 0;
        let mut x_count = 0;
        let mut y_count = 0;
        let mut m_count = 0;
        let mut d_count = 0;

        for t in &self.project.tags {
            if !self.project.tag_bound_to(&t.id).is_empty() {
                bound_count += 1;
            }
            let upper = t.address.to_uppercase();
            if upper.starts_with('X') {
                x_count += 1;
            } else if upper.starts_with('Y') {
                y_count += 1;
            } else if upper.starts_with('M') {
                m_count += 1;
            } else if upper.starts_with('D') {
                d_count += 1;
            }
        }
        let unbound_count = total_tags.saturating_sub(bound_count);

        ui.horizontal(|ui| {
            render_kpi_card(
                ui,
                "總 Tag 點位",
                &total_tags.to_string(),
                "已宣告總數",
                Color32::from_rgb(100, 200, 255),
            );
            render_kpi_card(
                ui,
                "已綁定插槽",
                &bound_count.to_string(),
                "對應至單元",
                Color32::from_rgb(120, 220, 140),
            );
            render_kpi_card(
                ui,
                "未綁定點位",
                &unbound_count.to_string(),
                if unbound_count > 0 {
                    "待裝配使用"
                } else {
                    "已全部綁定"
                },
                if unbound_count > 0 {
                    Color32::from_rgb(255, 180, 80)
                } else {
                    Color32::GRAY
                },
            );
            render_kpi_card(
                ui,
                "I/O 類別分布",
                &format!("X:{} Y:{}", x_count, y_count),
                &format!("M:{} D:{}", m_count, d_count),
                Color32::from_rgb(220, 160, 255),
            );
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Search & Filter Toolbar
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍 搜尋:").strong());
            ui.add(
                egui::TextEdit::singleline(&mut self.tag_search)
                    .return_key(None)
                    .hint_text("搜尋 ID、位址或標籤...")
                    .desired_width(180.0),
            );

            ui.separator();
            ui.label(RichText::new("前綴:").strong());
            for pf in ["ALL", "X", "Y", "M", "D", "OTHER"] {
                if ui
                    .selectable_label(self.tag_prefix_filter == pf, pf)
                    .clicked()
                {
                    self.tag_prefix_filter = pf.to_string();
                }
            }

            ui.separator();
            ui.checkbox(&mut self.tag_unbound_only, "只顯示未綁定點位");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("➕ 新增 Tag").color(Color32::LIGHT_GREEN))
                    .clicked()
                {
                    let plc_id = self
                        .project
                        .plcs
                        .first()
                        .map(|p| p.id.as_str())
                        .unwrap_or("plc1");
                    let next_addr = self.project.next_available_address("M");
                    self.editing_tag = Some(TagConfig {
                        id: format!("{}.M_{}", plc_id, next_addr),
                        address: next_addr,
                        ty: "bool".to_string(),
                        label: Some("新點位".to_string()),
                        description: None,
                        polarity: None,
                        safety_monitor: None,
                        scan: None,
                        unit: None,
                        min: None,
                        max: None,
                        default: None,
                        spare: None,
                    });
                    self.show_tag_edit_modal = true;
                }

                if ui
                    .button(RichText::new("📥 匯入 I/O 表格").color(Color32::LIGHT_BLUE))
                    .clicked()
                {
                    self.show_import_modal = true;
                    self.import_preview_tags.clear();
                    self.import_error = None;
                }
            });
        });

        ui.add_space(8.0);

        // Tags Table
        egui::ScrollArea::vertical()
            .id_salt("tags_table_scroll")
            .show(ui, |ui| {
                let search_term = self.tag_search.to_lowercase();
                let prefix_filter = self.tag_prefix_filter.clone();
                let unbound_only = self.tag_unbound_only;

                let mut tag_to_delete = None;
                let mut tag_to_edit = None;

                egui::Grid::new("tags_table_grid")
                    .num_columns(8)
                    .spacing([14.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Tag ID").strong());
                        ui.label(RichText::new("PLC 位址").strong());
                        ui.label(RichText::new("資料型別").strong());
                        ui.label(RichText::new("名稱 / 標籤").strong());
                        ui.label(RichText::new("極性").strong());
                        ui.label(RichText::new("安全監控").strong());
                        ui.label(RichText::new("綁定單元與插槽").strong());
                        ui.label(RichText::new("操作").strong());
                        ui.end_row();

                        for t in &self.project.tags {
                            // 篩選：搜尋
                            if !search_term.is_empty() {
                                let match_id = t.id.to_lowercase().contains(&search_term);
                                let match_addr = t.address.to_lowercase().contains(&search_term);
                                let match_label = t
                                    .label
                                    .as_deref()
                                    .map(|l| l.to_lowercase().contains(&search_term))
                                    .unwrap_or(false);
                                if !match_id && !match_addr && !match_label {
                                    continue;
                                }
                            }

                            // 篩選：前綴
                            let addr_upper = t.address.to_uppercase();
                            if prefix_filter != "ALL" {
                                if prefix_filter == "OTHER" {
                                    if addr_upper.starts_with('X')
                                        || addr_upper.starts_with('Y')
                                        || addr_upper.starts_with('M')
                                        || addr_upper.starts_with('D')
                                    {
                                        continue;
                                    }
                                } else if !addr_upper.starts_with(&prefix_filter) {
                                    continue;
                                }
                            }

                            // 篩選：未綁定
                            let bindings = self.project.tag_bound_to(&t.id);
                            if unbound_only && !bindings.is_empty() {
                                continue;
                            }

                            // 欄位 1: ID
                            ui.label(RichText::new(&t.id).monospace().strong());

                            // 欄位 2: 位址
                            let addr_col = if addr_upper.starts_with('X') {
                                Color32::from_rgb(100, 180, 255)
                            } else if addr_upper.starts_with('Y') {
                                Color32::from_rgb(220, 140, 255)
                            } else if addr_upper.starts_with('M') {
                                Color32::from_rgb(140, 230, 160)
                            } else {
                                Color32::from_rgb(255, 200, 100)
                            };
                            ui.label(RichText::new(&t.address).color(addr_col).monospace());

                            // 欄位 3: 型別
                            ui.label(RichText::new(&t.ty).small());

                            // 欄位 4: 標籤
                            ui.label(
                                RichText::new(t.label.as_deref().unwrap_or("-"))
                                    .color(Color32::LIGHT_GRAY),
                            );

                            // 欄位 5: 極性
                            let pol_txt = match t.polarity {
                                Some(Polarity::NormallyOpen) => "NO (常開)",
                                Some(Polarity::NormallyClosed) => "NC (常閉)",
                                None => "-",
                            };
                            ui.label(RichText::new(pol_txt).small());

                            // 欄位 6: 安全監控
                            if let Some(true) = t.safety_monitor {
                                ui.label(RichText::new("🛡️ 是").color(Color32::LIGHT_RED).small());
                            } else {
                                ui.label(RichText::new("-").color(Color32::GRAY));
                            }

                            // 欄位 7: 綁定單元與插槽
                            if bindings.is_empty() {
                                ui.label(RichText::new("［未綁定］").color(Color32::GRAY).small());
                            } else {
                                let bound_desc = bindings
                                    .iter()
                                    .map(|(u, s)| format!("{}.{}", u, s))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                ui.label(
                                    RichText::new(bound_desc)
                                        .color(Color32::from_rgb(100, 200, 255))
                                        .small(),
                                );
                            }

                            // 欄位 8: 操作按鈕
                            ui.horizontal(|ui| {
                                if ui.small_button("✏️ 編輯").clicked() {
                                    tag_to_edit = Some(t.clone());
                                }
                                if ui
                                    .small_button(RichText::new("🗑️").color(Color32::LIGHT_RED))
                                    .clicked()
                                {
                                    tag_to_delete = Some(t.id.clone());
                                }
                            });

                            ui.end_row();
                        }
                    });

                if let Some(tag_id) = tag_to_delete {
                    self.project.delete_tag(&tag_id);
                    self.trigger_validation();
                    self.set_toast(format!("已刪除 Tag `{}`", tag_id), Color32::LIGHT_RED);
                }

                if let Some(tag) = tag_to_edit {
                    self.editing_tag = Some(tag);
                    self.show_tag_edit_modal = true;
                }
            });
    }
}

fn render_kpi_card(
    ui: &mut egui::Ui,
    title: &str,
    main_val: &str,
    sub_text: &str,
    accent: Color32,
) {
    egui::Frame::new()
        .fill(Color32::from_rgb(24, 28, 36))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 48, 60)))
        .show(ui, |ui| {
            ui.set_min_width(160.0);
            ui.label(RichText::new(title).color(Color32::GRAY).small());
            ui.add_space(2.0);
            ui.label(RichText::new(main_val).size(20.0).strong().color(accent));
            ui.add_space(2.0);
            ui.label(RichText::new(sub_text).color(Color32::LIGHT_GRAY).small());
        });
}

// ---------------------------------------------------------------------------
// Tab 3: Unit Type Catalog & Custom Types
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_unit_catalog_tab(&mut self, ui: &mut egui::Ui) {
        // 頂部導覽列與操作
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("📚 Unit 類型目錄與自訂類型管理器 (Unit Type Catalog Studio)")
                    .size(16.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        RichText::new("📥 載入外部 YAML 類型檔")
                            .color(Color32::from_rgb(100, 200, 255)),
                    )
                    .clicked()
                {
                    self.show_custom_type_modal = true;
                    self.custom_type_error = None;
                }

                if ui
                    .button(
                        RichText::new("➕ 新建 Unit 類型")
                            .strong()
                            .color(Color32::LIGHT_GREEN),
                    )
                    .clicked()
                {
                    self.type_editor_state.open_new();
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        ui.columns(2, |cols| {
            // 左欄：類型清單與搜尋
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍 搜尋:").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.catalog_search_query)
                            .return_key(None)
                            .desired_width(ui.available_width() - 8.0)
                            .hint_text("輸入 ID 或名稱關鍵字..."),
                    );
                });
                ui.add_space(6.0);

                let all_types: Vec<_> = self.project.catalog.iter().cloned().collect();
                let query = self.catalog_search_query.trim().to_lowercase();
                let filtered_types: Vec<_> = all_types
                    .into_iter()
                    .filter(|ut| {
                        if query.is_empty() {
                            true
                        } else {
                            ut.id.to_lowercase().contains(&query)
                                || ut.title.to_lowercase().contains(&query)
                        }
                    })
                    .collect();

                let std_count = filtered_types
                    .iter()
                    .filter(|ut| self.project.is_standard_type(&ut.id))
                    .count();
                let custom_count = filtered_types.len() - std_count;

                ui.label(
                    RichText::new(format!(
                        "共 {} 個類型（內嵌標準: {} | 本地自訂: {}）",
                        filtered_types.len(),
                        std_count,
                        custom_count
                    ))
                    .color(Color32::GRAY)
                    .small(),
                );
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("catalog_types_scroll")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height() - 10.0)
                    .show(ui, |ui| {
                        for ut in filtered_types {
                            let is_sel = self.selected_catalog_type == ut.id;
                            let is_standard = self.project.is_standard_type(&ut.id);

                            let (badge_text, badge_color) = if is_standard {
                                ("［標準］", Color32::from_rgb(100, 180, 255))
                            } else {
                                ("［自訂］", Color32::from_rgb(120, 240, 150))
                            };

                            egui::Frame::new()
                                .fill(if is_sel {
                                    Color32::from_rgb(35, 45, 60)
                                } else {
                                    Color32::from_rgb(22, 25, 32)
                                })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if is_sel {
                                        Color32::from_rgb(100, 180, 255)
                                    } else {
                                        Color32::from_rgb(38, 44, 56)
                                    },
                                ))
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(badge_text).color(badge_color).small(),
                                        );
                                        let btn = ui.selectable_label(
                                            is_sel,
                                            RichText::new(format!("{} (v{})", ut.id, ut.version))
                                                .strong(),
                                        );
                                        if btn.clicked() {
                                            self.selected_catalog_type = ut.id.clone();
                                        }
                                    });
                                    ui.label(
                                        RichText::new(format!("   {}", ut.title))
                                            .color(Color32::LIGHT_GRAY)
                                            .small(),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                    });
            });

            // 右欄：選取之類型詳情與 CRUD 控制
            cols[1].vertical(|ui| {
                let Some(ut) = self
                    .project
                    .catalog
                    .get(&self.selected_catalog_type)
                    .cloned()
                else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("請由左側清單選擇一個 Unit 類型檢視。")
                                .color(Color32::GRAY),
                        );
                    });
                    return;
                };

                let is_standard = self.project.is_standard_type(&ut.id);

                // 標題列與屬性標籤
                ui.horizontal(|ui| {
                    ui.heading(
                        RichText::new(format!("{} (v{})", ut.id, ut.version)).color(Color32::WHITE),
                    );
                    if is_standard {
                        ui.label(
                            RichText::new("🔒 核心內嵌標準（唯讀）")
                                .color(Color32::from_rgb(100, 180, 255))
                                .small(),
                        );
                    } else {
                        ui.label(
                            RichText::new("✏️ 使用者自訂類型")
                                .color(Color32::from_rgb(120, 240, 150))
                                .small(),
                        );
                    }
                });

                ui.label(
                    RichText::new(format!("名稱：{}", ut.title))
                        .size(16.0)
                        .color(Color32::KHAKI),
                );
                ui.add_space(6.0);

                // 操作工具列
                ui.horizontal_wrapped(|ui| {
                    if is_standard {
                        if ui
                            .button(
                                RichText::new("📑 複製為自訂類型並編輯")
                                    .color(Color32::from_rgb(180, 230, 255)),
                            )
                            .clicked()
                        {
                            self.type_editor_state.open_clone(&ut);
                        }
                    } else {
                        if ui
                            .button(
                                RichText::new("✏️ 視覺化編輯類型")
                                    .strong()
                                    .color(Color32::LIGHT_BLUE),
                            )
                            .clicked()
                        {
                            self.type_editor_state.open_edit(&ut);
                        }

                        if ui
                            .button(
                                RichText::new("🗑️ 刪除自訂類型")
                                    .color(Color32::from_rgb(255, 100, 100)),
                            )
                            .clicked()
                        {
                            self.show_delete_type_confirm = Some(ut.id.clone());
                        }

                        if ui
                            .button(
                                RichText::new("💾 匯出為 .yaml 檔")
                                    .color(Color32::from_rgb(160, 230, 160)),
                            )
                            .clicked()
                        {
                            self.export_type_id = ut.id.clone();
                            self.export_type_path = format!("types/{}.yaml", ut.id);
                            self.export_type_yaml = self
                                .project
                                .get_custom_type_yaml(&ut.id)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    crate::type_editor::UnitTypeDraft::from_unit_type(&ut)
                                        .to_yaml_string()
                                });
                            self.show_export_type_modal = true;
                        }
                    }

                    if ui
                        .button(
                            RichText::new("➕ 以此類型建立 Unit 至專案")
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        self.new_unit_type = ut.id.clone();
                        self.new_unit_id = format!("{}_{}", ut.id, self.project.units.len() + 1);
                        self.show_add_unit_modal = true;
                    }

                    if ui.button(RichText::new("📋 複製 YAML").small()).clicked() {
                        let yaml = self
                            .project
                            .get_custom_type_yaml(&ut.id)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                crate::type_editor::UnitTypeDraft::from_unit_type(&ut)
                                    .to_yaml_string()
                            });
                        ui.ctx().copy_text(yaml);
                        self.set_toast(
                            format!("已複製 `{}` 的 YAML 定義至剪貼簿", ut.id),
                            Color32::LIGHT_GREEN,
                        );
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // 詳情捲動區域
                egui::ScrollArea::vertical()
                    .id_salt("catalog_type_details_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // 1. 實體現場接腳
                        ui.label(RichText::new("📍 實體現場接腳 (field)").strong().size(15.0));
                        if ut.field.input.is_empty() && ut.field.output.is_empty() {
                            ui.label(
                                RichText::new("  (無實體 field 接腳)")
                                    .color(Color32::GRAY)
                                    .small(),
                            );
                        } else {
                            egui::Grid::new("field_pins_grid")
                                .num_columns(5)
                                .spacing([12.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(RichText::new("方向").strong().small());
                                    ui.label(RichText::new("名稱").strong().small());
                                    ui.label(RichText::new("型別").strong().small());
                                    ui.label(RichText::new("必接").strong().small());
                                    ui.label(RichText::new("標籤 / 單位").strong().small());
                                    ui.end_row();

                                    for (name, pin) in &ut.field.input.0 {
                                        ui.label(
                                            RichText::new("輸入 (DI)")
                                                .color(Color32::from_rgb(100, 180, 255))
                                                .monospace(),
                                        );
                                        ui.label(RichText::new(name).strong());
                                        ui.label(
                                            RichText::new(&pin.ty)
                                                .color(Color32::KHAKI)
                                                .monospace(),
                                        );
                                        let req = pin.required.unwrap_or(true);
                                        ui.label(if req {
                                            RichText::new("必接")
                                                .color(Color32::from_rgb(255, 120, 120))
                                        } else {
                                            RichText::new("選接").color(Color32::GRAY)
                                        });
                                        let mut extra = pin.label.clone().unwrap_or_default();
                                        if let Some(ref u) = pin.unit {
                                            extra.push_str(&format!(" ({})", u));
                                        }
                                        ui.label(RichText::new(extra).color(Color32::LIGHT_GRAY));
                                        ui.end_row();
                                    }

                                    for (name, pin) in &ut.field.output.0 {
                                        ui.label(
                                            RichText::new("輸出 (DO)")
                                                .color(Color32::from_rgb(220, 140, 255))
                                                .monospace(),
                                        );
                                        ui.label(RichText::new(name).strong());
                                        ui.label(
                                            RichText::new(&pin.ty)
                                                .color(Color32::KHAKI)
                                                .monospace(),
                                        );
                                        let req = pin.required.unwrap_or(true);
                                        ui.label(if req {
                                            RichText::new("必接")
                                                .color(Color32::from_rgb(255, 120, 120))
                                        } else {
                                            RichText::new("選接").color(Color32::GRAY)
                                        });
                                        let mut extra = pin.label.clone().unwrap_or_default();
                                        if let Some(ref u) = pin.unit {
                                            extra.push_str(&format!(" ({})", u));
                                        }
                                        ui.label(RichText::new(extra).color(Color32::LIGHT_GRAY));
                                        ui.end_row();
                                    }
                                });
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 2. 控制介面與警報
                        ui.label(
                            RichText::new("⚙️ 控制與警報介面 (ctrl)")
                                .strong()
                                .size(15.0),
                        );
                        if ut.ctrl.is_empty() {
                            ui.label(
                                RichText::new("  (無 ctrl 接腳)")
                                    .color(Color32::GRAY)
                                    .small(),
                            );
                        } else {
                            egui::Grid::new("ctrl_pins_grid")
                                .num_columns(5)
                                .spacing([12.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label(RichText::new("前綴 / 語意").strong().small());
                                    ui.label(RichText::new("完整名稱").strong().small());
                                    ui.label(RichText::new("型別").strong().small());
                                    ui.label(RichText::new("標籤 / 警報說明").strong().small());
                                    ui.label(RichText::new("參數範圍 / 預設值").strong().small());
                                    ui.end_row();

                                    for (name, ctrl) in &ut.ctrl.0 {
                                        let prefix_opt = naming::split(name).map(|(p, _)| p);
                                        let (pref_name, pref_col) = match prefix_opt {
                                            Some(Prefix::Cmd) => ("cmd (命令)", Color32::GRAY),
                                            Some(Prefix::Man) => {
                                                ("man (手動)", Color32::from_rgb(100, 220, 120))
                                            }
                                            Some(Prefix::Ena) => {
                                                ("ena (允許)", Color32::from_rgb(255, 180, 80))
                                            }
                                            Some(Prefix::Par) => {
                                                ("par (參數)", Color32::from_rgb(100, 200, 255))
                                            }
                                            Some(Prefix::At) => {
                                                ("at (到位)", Color32::from_rgb(120, 180, 255))
                                            }
                                            Some(Prefix::Sts) => {
                                                ("sts (狀態)", Color32::from_rgb(100, 220, 200))
                                            }
                                            Some(Prefix::Error) => {
                                                ("error (警報)", Color32::from_rgb(255, 100, 100))
                                            }
                                            Some(Prefix::Count) => {
                                                ("count (計數)", Color32::from_rgb(200, 150, 255))
                                            }
                                            Some(Prefix::Pv) => {
                                                ("pv (製程值)", Color32::from_rgb(150, 200, 255))
                                            }
                                            Some(Prefix::Msg) => {
                                                ("msg (訊息)", Color32::from_rgb(255, 220, 100))
                                            }
                                            None => ("未知", Color32::GRAY),
                                        };

                                        ui.label(RichText::new(pref_name).color(pref_col).small());
                                        ui.label(RichText::new(name).monospace().strong());

                                        let ty_str = prefix_opt
                                            .and_then(|p| ctrl.type_text(p))
                                            .unwrap_or("bool");
                                        ui.label(
                                            RichText::new(ty_str).color(Color32::KHAKI).monospace(),
                                        );

                                        let label_or_msg = if let Some(ref m) = ctrl.message {
                                            format!("警報: {}", m)
                                        } else {
                                            ctrl.label.clone().unwrap_or_default()
                                        };
                                        ui.label(RichText::new(label_or_msg).color(Color32::WHITE));

                                        let mut range_info = String::new();
                                        if let (Some(min), Some(max)) = (ctrl.min, ctrl.max) {
                                            range_info.push_str(&format!("{}..={}", min, max));
                                        }
                                        if let Some(ref def) = ctrl.default {
                                            if !range_info.is_empty() {
                                                range_info.push_str(", ");
                                            }
                                            let def_str = match def {
                                                Scalar::Bool(b) => b.to_string(),
                                                Scalar::Int(i) => i.to_string(),
                                                Scalar::Float(f) => f.to_string(),
                                                Scalar::Text(s) => s.clone(),
                                            };
                                            range_info.push_str(&format!("預設: {}", def_str));
                                        }
                                        if let Some(ref u) = ctrl.unit {
                                            range_info.push_str(&format!(" {}", u));
                                        }
                                        ui.label(
                                            RichText::new(range_info).color(Color32::GRAY).small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }

                        if !ut.parts.is_empty() {
                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(6.0);
                            ui.label(RichText::new("🧩 子組件插槽 (parts)").strong().size(15.0));
                            for (pname, ptype) in &ut.parts.0 {
                                ui.label(
                                    RichText::new(format!(
                                        "  • 插槽 `{}` -> 目標單元類型 `{}`",
                                        pname, ptype
                                    ))
                                    .color(Color32::LIGHT_GREEN),
                                );
                            }
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 3. YAML 定義預覽
                        ui.collapsing("📝 檢視標準 YAML 定義文字", |ui| {
                            let mut yaml_text = self
                                .project
                                .get_custom_type_yaml(&ut.id)
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| {
                                    crate::type_editor::UnitTypeDraft::from_unit_type(&ut)
                                        .to_yaml_string()
                                });
                            egui::Frame::new()
                                .fill(Color32::from_rgb(18, 20, 24))
                                .inner_margin(egui::Margin::same(8))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(40, 48, 60)))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut yaml_text)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY),
                                    );
                                });
                        });
                    });
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Tab 4: YAML Preview & Export
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_yaml_preview_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("📝 YAML 規格預覽與推導結果").size(16.0));

            ui.separator();
            ui.selectable_value(
                &mut self.yaml_view_mode,
                YamlViewMode::RawSource,
                "machine.yaml 規格檔",
            );
            ui.selectable_value(
                &mut self.yaml_view_mode,
                YamlViewMode::ResolvedDump,
                "推導結果傾印 (Resolved Dump)",
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("📋 複製到剪貼簿").color(Color32::LIGHT_GREEN))
                    .clicked()
                {
                    let text = match self.yaml_view_mode {
                        YamlViewMode::RawSource => &self.yaml_text_cache,
                        YamlViewMode::ResolvedDump => &self.yaml_dump_cache,
                    };
                    ui.ctx().copy_text(text.clone());
                    self.set_toast("已複製 YAML 至剪貼簿", Color32::LIGHT_GREEN);
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let content = match self.yaml_view_mode {
            YamlViewMode::RawSource => &self.yaml_text_cache,
            YamlViewMode::ResolvedDump => {
                if self.yaml_dump_cache.is_empty() {
                    "（模型規格檢查未通過，無法產生推導傾印。請先修正底層錯誤。）"
                } else {
                    &self.yaml_dump_cache
                }
            }
        };

        egui::ScrollArea::vertical()
            .id_salt("yaml_preview_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut text_buf = content.to_string();
                ui.add(
                    egui::TextEdit::multiline(&mut text_buf)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
    }
}

// ---------------------------------------------------------------------------
// Modals & Popups
// ---------------------------------------------------------------------------

impl ModelDebuggerApp {
    fn render_all_modals(&mut self, ctx: &egui::Context) {
        self.render_new_project_modal(ctx);
        self.render_open_file_modal(ctx);
        self.render_save_as_modal(ctx);
        self.render_add_unit_modal(ctx);
        self.render_rename_unit_modal(ctx);
        self.render_quick_tag_modal(ctx);
        self.render_import_modal(ctx);
        self.render_custom_type_modal(ctx);
        self.render_tag_edit_modal(ctx);
        self.render_type_editor_modal(ctx);
        self.render_delete_type_confirm_modal(ctx);
        self.render_export_type_modal(ctx);
    }

    fn render_new_project_modal(&mut self, ctx: &egui::Context) {
        if !self.show_new_project_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("📄 新增專案")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("機台 ID (Machine ID，必填識別碼):").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_project_machine_id).return_key(None),
                );

                ui.add_space(4.0);
                ui.label(RichText::new("機台名稱 / 標題 (Title):").strong());
                ui.add(egui::TextEdit::singleline(&mut self.new_project_title).return_key(None));

                ui.add_space(6.0);
                ui.label(RichText::new("主 PLC 系列 (PLC Family):").strong());
                egui::ComboBox::from_id_salt("new_project_family_combo")
                    .selected_text(self.new_project_family.as_str())
                    .show_ui(ui, |ui| {
                        for &f in Family::ALL {
                            ui.selectable_value(&mut self.new_project_family, f, f.as_str());
                        }
                    });

                ui.add_space(8.0);
                ui.label(RichText::new("起始專案範本:").strong());
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.new_project_template_is_demo,
                        false,
                        "全新空白專案 (自行新增 Unit 與 Tag)",
                    );
                    ui.radio_value(
                        &mut self.new_project_template_is_demo,
                        true,
                        "標準示範機台範本",
                    );
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_new_project_modal = false;
                    }

                    if ui
                        .button(
                            RichText::new("✨ 立即建立專案")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        let mid = self.new_project_machine_id.trim().to_string();
                        if mid.is_empty() {
                            self.set_toast("機台 ID 不得為空白", Color32::LIGHT_RED);
                        } else {
                            if self.new_project_template_is_demo {
                                let mut p = ProjectModel::demo();
                                p.machine_id = mid.clone();
                                p.machine_title = if self.new_project_title.trim().is_empty() {
                                    None
                                } else {
                                    Some(self.new_project_title.trim().to_string())
                                };
                                if let Some(plc) = p.plcs.first_mut() {
                                    plc.family = self.new_project_family;
                                }
                                self.project = p;
                            } else {
                                let title = if self.new_project_title.trim().is_empty() {
                                    None
                                } else {
                                    Some(self.new_project_title.trim().to_string())
                                };
                                self.project =
                                    ProjectModel::new_empty(&mid, title, self.new_project_family);
                            }
                            self.current_file_path = None;
                            self.selected_unit_id =
                                self.project.units.first().map(|u| u.id.clone());
                            self.trigger_validation();
                            self.show_new_project_modal = false;
                            self.set_toast(
                                format!("已成功建立專案 `{}`", mid),
                                Color32::LIGHT_GREEN,
                            );
                        }
                    }
                });
            });

        if !open {
            self.show_new_project_modal = false;
        }
    }

    fn render_open_file_modal(&mut self, ctx: &egui::Context) {
        if !self.show_open_file_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("📂 開啟 machine.yaml 檔案")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(540.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("請輸入或選擇要讀取的 YAML 檔案路徑：").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("檔案路徑:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.open_file_path_input)
                            .return_key(None)
                            .desired_width(ui.available_width() - 8.0)
                            .hint_text("machine.yaml 或 /path/to/machine.yaml"),
                    );
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new("快速捷徑 / 範本檔案：")
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.horizontal_wrapped(|ui| {
                    let quick_paths = [
                        "machine.yaml",
                        "machine-model/tests/fixtures/structural_full.yaml",
                        "machine-model/tests/fixtures/valid_units.yaml",
                    ];
                    for qp in quick_paths {
                        if ui.button(RichText::new(qp).small()).clicked() {
                            self.open_file_path_input = qp.to_string();
                        }
                    }
                });

                if let Some(ref err) = self.open_file_error {
                    ui.add_space(8.0);
                    ui.colored_label(Color32::RED, format!("❌ 讀取失敗：{}", err));
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_open_file_modal = false;
                        self.open_file_error = None;
                    }

                    if ui
                        .button(
                            RichText::new("📂 確認讀取並載入")
                                .strong()
                                .color(Color32::LIGHT_BLUE),
                        )
                        .clicked()
                    {
                        let path_str = self.open_file_path_input.trim().to_string();
                        if path_str.is_empty() {
                            self.open_file_error = Some("請提供檔案路徑".to_string());
                        } else {
                            match std::fs::read_to_string(&path_str) {
                                Ok(content) => {
                                    match ProjectModel::from_yaml_str(
                                        &content,
                                        self.project.catalog.clone(),
                                    ) {
                                        Ok(loaded) => {
                                            self.project = loaded;
                                            self.current_file_path = Some(path_str.clone());
                                            self.selected_unit_id =
                                                self.project.units.first().map(|u| u.id.clone());
                                            self.trigger_validation();
                                            self.show_open_file_modal = false;
                                            self.open_file_error = None;
                                            self.set_toast(
                                                format!("已成功載入專案：{}", path_str),
                                                Color32::LIGHT_GREEN,
                                            );
                                        }
                                        Err(diags) => {
                                            let msgs: Vec<String> = diags
                                                .iter()
                                                .map(|d| format!("[{}] {}", d.code, d.message))
                                                .collect();
                                            self.open_file_error = Some(format!(
                                                "YAML 模型語意錯誤：\n{}",
                                                msgs.join("\n")
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.open_file_error = Some(format!("讀取實體檔案失敗：{}", e));
                                }
                            }
                        }
                    }
                });
            });

        if !open {
            self.show_open_file_modal = false;
            self.open_file_error = None;
        }
    }

    fn render_save_as_modal(&mut self, ctx: &egui::Context) {
        if !self.show_save_as_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("💾 另存 machine.yaml 檔案")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("請輸入另存檔案路徑：").strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("目標路徑:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.save_as_path_input)
                            .return_key(None)
                            .desired_width(ui.available_width() - 8.0)
                            .hint_text("machine.yaml"),
                    );
                });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "專案資訊：機台 ID `{}` | Unit 數量: {} | Tag 數量: {}",
                        self.project.machine_id,
                        self.project.units.len(),
                        self.project.tags.len(),
                    ))
                    .small()
                    .color(Color32::GRAY),
                );

                if let Some(ref err) = self.save_as_error {
                    ui.add_space(8.0);
                    ui.colored_label(Color32::RED, format!("❌ 寫入失敗：{}", err));
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_save_as_modal = false;
                        self.save_as_error = None;
                    }

                    if ui
                        .button(
                            RichText::new("💾 確認儲存")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        let path_str = self.save_as_path_input.trim().to_string();
                        if path_str.is_empty() {
                            self.save_as_error = Some("檔案路徑不得為空".to_string());
                        } else {
                            if let Some(parent) = std::path::Path::new(&path_str)
                                .parent()
                                .filter(|p| !p.as_os_str().is_empty())
                            {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let yaml_content = self.project.to_yaml_string();
                            match std::fs::write(&path_str, yaml_content) {
                                Ok(()) => {
                                    self.current_file_path = Some(path_str.clone());
                                    self.show_save_as_modal = false;
                                    self.save_as_error = None;
                                    self.set_toast(
                                        format!("專案已成功儲存至：{}", path_str),
                                        Color32::LIGHT_GREEN,
                                    );
                                }
                                Err(e) => {
                                    self.save_as_error = Some(format!("寫入失敗：{}", e));
                                }
                            }
                        }
                    }
                });
            });

        if !open {
            self.show_save_as_modal = false;
            self.save_as_error = None;
        }
    }

    fn render_add_unit_modal(&mut self, ctx: &egui::Context) {
        if !self.show_add_unit_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("➕ 新增裝配單元 (Add Unit)")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(360.0);

                ui.label(RichText::new("單元識別碼 (Unit ID):").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_unit_id)
                        .return_key(None)
                        .hint_text("例如 cyl2、station_load_2"),
                );
                ui.label(
                    RichText::new("ℹ️ 識別碼依規範限用英文字母開頭之識別碼（如 cyl2）")
                        .color(Color32::GRAY)
                        .small(),
                );

                ui.add_space(6.0);
                ui.label(RichText::new("選擇 Unit 類型模板:").strong());

                let types: Vec<String> =
                    self.project.catalog.iter().map(|t| t.id.clone()).collect();
                egui::ComboBox::from_id_salt("modal_add_unit_type")
                    .selected_text(&self.new_unit_type)
                    .show_ui(ui, |ui| {
                        for t in types {
                            ui.selectable_value(&mut self.new_unit_type, t.clone(), t);
                        }
                    });

                let selected_ver = self
                    .project
                    .catalog
                    .get(&self.new_unit_type)
                    .map(|t| t.version)
                    .unwrap_or(1);

                ui.label(
                    RichText::new(format!("模板版本: v{}", selected_ver))
                        .color(Color32::GRAY)
                        .small(),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("💡 說明：Unit 實例由 ID 與類型模板組成；若需自訂各腳位的中文標籤與警報說明，請至「Unit 類型庫」分頁中新建或修改類型。")
                        .color(Color32::from_rgb(160, 200, 220))
                        .small(),
                );

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_add_unit_modal = false;
                    }

                    if ui
                        .button(
                            RichText::new("確定新增")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        let id = self.new_unit_id.trim().to_string();
                        if id.is_empty() {
                            self.set_toast("Unit ID 不能為空", Color32::LIGHT_RED);
                        } else if self.project.units.iter().any(|u| u.id == id) {
                            self.set_toast("Unit ID 已存在，不可重複", Color32::LIGHT_RED);
                        } else {
                            self.project.add_unit(
                                id.clone(),
                                self.new_unit_type.clone(),
                                selected_ver,
                            );
                            self.selected_unit_id = Some(id);
                            self.trigger_validation();
                            self.show_add_unit_modal = false;
                            self.set_toast("已成功新增 Unit", Color32::LIGHT_GREEN);
                        }
                    }
                });
            });

        if !open {
            self.show_add_unit_modal = false;
        }
    }

    fn render_rename_unit_modal(&mut self, ctx: &egui::Context) {
        if !self.show_rename_unit_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("✏️ 重新命名單元")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.label(RichText::new("原 ID:").strong());
                ui.label(RichText::new(&self.rename_unit_old).monospace());

                ui.add_space(6.0);
                ui.label(RichText::new("新 ID:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.rename_unit_new).return_key(None));

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_rename_unit_modal = false;
                    }
                    if ui.button(RichText::new("確定變更").strong()).clicked() {
                        let new_id = self.rename_unit_new.trim().to_string();
                        if new_id.is_empty() {
                            self.set_toast("新 ID 不能為空", Color32::LIGHT_RED);
                        } else if new_id != self.rename_unit_old
                            && self.project.units.iter().any(|u| u.id == new_id)
                        {
                            self.set_toast("新 ID 與現有單元衝突", Color32::LIGHT_RED);
                        } else {
                            self.project.rename_unit(&self.rename_unit_old, &new_id);
                            self.selected_unit_id = Some(new_id);
                            self.trigger_validation();
                            self.show_rename_unit_modal = false;
                            self.set_toast("已成功重新命名 Unit", Color32::LIGHT_GREEN);
                        }
                    }
                });
            });

        if !open {
            self.show_rename_unit_modal = false;
        }
    }

    fn render_quick_tag_modal(&mut self, ctx: &egui::Context) {
        if !self.show_quick_tag_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("⚡ 快速建立並綁定 Tag")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label(
                    RichText::new(format!(
                        "目標插槽：{}.{}",
                        self.quick_tag_unit_id, self.quick_tag_slot_name
                    ))
                    .color(Color32::LIGHT_BLUE)
                    .strong(),
                );
                ui.add_space(6.0);

                ui.label(RichText::new("Tag ID:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.quick_tag_id).return_key(None));

                ui.label(RichText::new("PLC 位址:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.quick_tag_address).return_key(None));

                ui.label(RichText::new("資料型別:").strong());
                egui::ComboBox::from_id_salt("quick_tag_ty_combo")
                    .selected_text(&self.quick_tag_type)
                    .show_ui(ui, |ui| {
                        for t in ["bool", "int16", "uint16", "int32", "float32"] {
                            ui.selectable_value(&mut self.quick_tag_type, t.to_string(), t);
                        }
                    });

                ui.label(RichText::new("標籤名稱 / 說明:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.quick_tag_label).return_key(None));

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_quick_tag_modal = false;
                    }

                    if ui
                        .button(
                            RichText::new("建立並立即綁定")
                                .color(Color32::LIGHT_GREEN)
                                .strong(),
                        )
                        .clicked()
                    {
                        let tag_id = self.quick_tag_id.trim().to_string();
                        let address = self.quick_tag_address.trim().to_uppercase();

                        if tag_id.is_empty() || address.is_empty() {
                            self.set_toast("ID 與位址不可為空", Color32::LIGHT_RED);
                        } else {
                            // 建立 Tag
                            let new_tag = TagConfig {
                                id: tag_id.clone(),
                                address,
                                ty: self.quick_tag_type.clone(),
                                label: if self.quick_tag_label.trim().is_empty() {
                                    None
                                } else {
                                    Some(self.quick_tag_label.trim().to_string())
                                },
                                description: None,
                                polarity: None,
                                safety_monitor: None,
                                scan: None,
                                unit: None,
                                min: None,
                                max: None,
                                default: None,
                                spare: None,
                            };

                            // 若 ID 已存在則更新，否則加入
                            if let Some(pos) = self.project.tags.iter().position(|t| t.id == tag_id)
                            {
                                self.project.tags[pos] = new_tag;
                            } else {
                                self.project.tags.push(new_tag);
                            }

                            // 綁定至目標插槽
                            if let Some(u) = self
                                .project
                                .units
                                .iter_mut()
                                .find(|u| u.id == self.quick_tag_unit_id)
                            {
                                u.bind
                                    .insert(self.quick_tag_slot_name.clone(), Some(tag_id));
                            }

                            self.trigger_validation();
                            self.show_quick_tag_modal = false;
                            self.set_toast("已建立 Tag 並成功綁定", Color32::LIGHT_GREEN);
                        }
                    }
                });
            });

        if !open {
            self.show_quick_tag_modal = false;
        }
    }

    fn render_import_modal(&mut self, ctx: &egui::Context) {
        if !self.show_import_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("📥 匯入 PLC I/O 點位")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 600.0])
            .min_width(680.0)
            .min_height(480.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.import_method,
                        ImportMethod::Delimited,
                        "📋 貼上 CSV / TSV / 軟體符號表",
                    );
                    ui.selectable_value(
                        &mut self.import_method,
                        ImportMethod::YamlBlock,
                        "📄 貼上 YAML 點位區塊",
                    );
                    ui.selectable_value(
                        &mut self.import_method,
                        ImportMethod::Preset,
                        "⚡ 快速套用機型範本",
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                let default_plc = self
                    .project
                    .plcs
                    .first()
                    .map(|p| p.id.as_str())
                    .unwrap_or("plc1");

                match self.import_method {
                    ImportMethod::Delimited => {
                        ui.label(RichText::new("請由 Excel、記事本或 PLC 軟體（匯川 AutoShop、三菱 GX Works 等）複製貼上表格：").small());
                        ui.label(RichText::new("支援自動識別：[序號, 變量名, 類型, 保持, 範圍, 軟元件位址, 註釋] 等多欄結構，或簡化 [名稱, 位址, 類型, 說明] / [位址, 說明]").color(Color32::GRAY).small());
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .id_salt("import_delimited_input_scroll")
                            .max_height(140.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.import_text)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(6)
                                        .hint_text("9\tI_RightGripper_OpenLimit\tBOOL\t不保持\t私有\tX010\t右夾爪開極限\n10\tI_GripperCylinder_ForwardLimit\tBOOL\t不保持\t私有\tX011\t前進檢知"),
                                );
                            });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("🔍 解析並預覽").strong()).clicked() {
                                match parse_delimited(&self.import_text, default_plc) {
                                    Ok(tags) => {
                                        self.import_preview_tags = tags;
                                        self.import_error = None;
                                    }
                                    Err(e) => {
                                        self.import_error = Some(e);
                                        self.import_preview_tags.clear();
                                    }
                                }
                            }
                            if ui.button("清空輸入內容").clicked() {
                                self.import_text.clear();
                                self.import_preview_tags.clear();
                                self.import_error = None;
                            }
                        });
                    }
                    ImportMethod::YamlBlock => {
                        ui.label(RichText::new("貼上 YAML tags 陣列片段：").small());
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt("import_yaml_input_scroll")
                            .max_height(140.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.import_text)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(6)
                                        .hint_text("- { id: plc1.X10, address: X10, type: bool, label: 檢知 }\n- { id: plc1.Y10, address: Y10, type: bool }"),
                                );
                            });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("🔍 解析並預覽").strong()).clicked() {
                                match parse_yaml_tags(&self.import_text) {
                                    Ok(tags) => {
                                        self.import_preview_tags = tags;
                                        self.import_error = None;
                                    }
                                    Err(e) => {
                                        self.import_error = Some(e);
                                        self.import_preview_tags.clear();
                                    }
                                }
                            }
                            if ui.button("清空輸入內容").clicked() {
                                self.import_text.clear();
                                self.import_preview_tags.clear();
                                self.import_error = None;
                            }
                        });
                    }
                    ImportMethod::Preset => {
                        ui.label(RichText::new("選擇常用的 PLC 機型標準點表範本：").strong());
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui.button("⚡ 匯川 Inovance Easy 常用 I/O 集合 (X0~X17, Y0~Y17, M100~M115, D100~D109)").clicked() {
                                self.import_preview_tags = generate_inovance_easy_preset(default_plc);
                                self.import_error = None;
                            }
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button("⚡ 三菱 Mitsubishi FX 常用 I/O 集合 (X0~X17, Y0~Y17, M0~M15)").clicked() {
                                self.import_preview_tags = generate_mitsubishi_preset(default_plc);
                                self.import_error = None;
                            }
                        });
                    }
                }

                if let Some(ref err) = self.import_error {
                    ui.add_space(6.0);
                    ui.colored_label(Color32::RED, format!("❌ {}", err));
                }

                if !self.import_preview_tags.is_empty() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("預覽待匯入點位（共 {} 個）：", self.import_preview_tags.len()))
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        );
                        if self.import_preview_tags.len() > 100 {
                            ui.label(RichText::new("(預覽僅顯示前 100 筆)").small().color(Color32::GRAY));
                        }
                    });
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("import_preview_tags_scroll")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            egui::Grid::new("import_preview_grid")
                                .striped(true)
                                .spacing([14.0, 6.0])
                                .min_col_width(40.0)
                                .show(ui, |ui| {
                                    ui.label(RichText::new("#").strong().color(Color32::GRAY));
                                    ui.label(RichText::new("Tag ID").strong().color(Color32::LIGHT_BLUE));
                                    ui.label(RichText::new("PLC 位址").strong().color(Color32::KHAKI));
                                    ui.label(RichText::new("型別").strong().color(Color32::WHITE));
                                    ui.label(RichText::new("說明 (Label)").strong().color(Color32::LIGHT_GRAY));
                                    ui.end_row();

                                    for (i, t) in self.import_preview_tags.iter().take(100).enumerate() {
                                        ui.label(RichText::new(format!("{}", i + 1)).small().color(Color32::GRAY));
                                        ui.label(RichText::new(&t.id).monospace().strong());
                                        ui.label(
                                            RichText::new(&t.address)
                                                .monospace()
                                                .color(Color32::from_rgb(255, 215, 0)),
                                        );
                                        ui.label(RichText::new(&t.ty).monospace().color(Color32::from_rgb(100, 220, 220)));
                                        ui.label(
                                            RichText::new(t.label.as_deref().unwrap_or("-"))
                                                .color(Color32::LIGHT_GRAY),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new(format!("📥 全部加入專案（合併 {} 筆）", self.import_preview_tags.len()))
                                    .strong()
                                    .color(Color32::LIGHT_GREEN),
                            )
                            .clicked()
                        {
                            let mut added_count = 0;
                            for t in self.import_preview_tags.drain(..) {
                                if let Some(pos) = self.project.tags.iter().position(|x| x.id == t.id) {
                                    self.project.tags[pos] = t;
                                } else {
                                    self.project.tags.push(t);
                                }
                                added_count += 1;
                            }
                            self.trigger_validation();
                            self.show_import_modal = false;
                            self.set_toast(format!("已成功匯入 {} 個點位", added_count), Color32::LIGHT_GREEN);
                        }

                        if ui
                            .button(
                                RichText::new(format!("⚠️ 覆蓋現有全部 Tag（共 {} 筆）", self.import_preview_tags.len()))
                                    .color(Color32::LIGHT_RED),
                            )
                            .clicked()
                        {
                            let count = self.import_preview_tags.len();
                            self.project.tags = std::mem::take(&mut self.import_preview_tags);
                            self.trigger_validation();
                            self.show_import_modal = false;
                            self.set_toast(format!("已重設並套用 {} 個點位", count), Color32::LIGHT_RED);
                        }

                        if ui.button("取消").clicked() {
                            self.show_import_modal = false;
                        }
                    });
                }
            });

        if !open {
            self.show_import_modal = false;
        }
    }

    fn render_custom_type_modal(&mut self, ctx: &egui::Context) {
        if !self.show_custom_type_modal {
            return;
        }

        let mut open = true;
        egui::Window::new("📂 載入自訂 Unit 類型檔 (YAML)")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("來源名稱 / 檔名:").strong());
                ui.add(egui::TextEdit::singleline(&mut self.custom_type_source_name).return_key(None));

                ui.add_space(6.0);
                ui.label(RichText::new("UnitType YAML 內容:").strong());
                egui::ScrollArea::vertical()
                    .id_salt("custom_type_yaml_scroll")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.custom_type_yaml)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .hint_text("id: custom_feeder\nversion: 1\ntitle: 震動送料機\nfield:\n  in:\n    sensor: { type: bool }\nctrl:\n  cmd_start: {}\nparts: {}"),
                        );
                    });

                if let Some(ref err) = self.custom_type_error {
                    ui.add_space(6.0);
                    ui.colored_label(Color32::RED, format!("❌ 驗證失敗：{}", err));
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.show_custom_type_modal = false;
                    }

                    if ui
                        .button(RichText::new("載入並註冊至類型目錄").strong().color(Color32::LIGHT_GREEN))
                        .clicked()
                    {
                        match self.project.load_custom_type(
                            &self.custom_type_source_name,
                            &self.custom_type_yaml,
                        ) {
                            Ok(_) => {
                                self.trigger_validation();
                                self.show_custom_type_modal = false;
                                self.set_toast("已成功註冊自訂 Unit 類型！", Color32::LIGHT_GREEN);
                            }
                            Err(diags) => {
                                let msg = diags
                                    .iter()
                                    .map(|d| d.to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                self.custom_type_error = Some(msg);
                            }
                        }
                    }
                });
            });

        if !open {
            self.show_custom_type_modal = false;
        }
    }

    fn render_tag_edit_modal(&mut self, ctx: &egui::Context) {
        if !self.show_tag_edit_modal {
            return;
        }

        let Some(mut tag) = self.editing_tag.clone() else {
            return;
        };

        let mut open = true;
        let mut save_clicked = false;
        let mut cancel_clicked = false;

        egui::Window::new("✏️ 編輯 Tag 點位")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(380.0);

                ui.label(RichText::new("Tag ID:").strong());
                ui.add(egui::TextEdit::singleline(&mut tag.id).return_key(None));

                ui.label(RichText::new("PLC 位址:").strong());
                ui.add(egui::TextEdit::singleline(&mut tag.address).return_key(None));

                ui.label(RichText::new("資料型別:").strong());
                egui::ComboBox::from_id_salt("edit_tag_ty_combo")
                    .selected_text(&tag.ty)
                    .show_ui(ui, |ui| {
                        for t in [
                            "bool", "int16", "uint16", "int32", "uint32", "float32", "float64",
                        ] {
                            ui.selectable_value(&mut tag.ty, t.to_string(), t);
                        }
                    });

                ui.label(RichText::new("標籤 (Label):").strong());
                let mut label_str = tag.label.clone().unwrap_or_default();
                if ui
                    .add(egui::TextEdit::singleline(&mut label_str).return_key(None))
                    .changed()
                {
                    tag.label = if label_str.trim().is_empty() {
                        None
                    } else {
                        Some(label_str)
                    };
                }

                ui.label(RichText::new("工程單位 (Unit):").strong());
                let mut unit_str = tag.unit.clone().unwrap_or_default();
                if ui
                    .add(egui::TextEdit::singleline(&mut unit_str).return_key(None))
                    .changed()
                {
                    tag.unit = if unit_str.trim().is_empty() {
                        None
                    } else {
                        Some(unit_str)
                    };
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut is_safety = tag.safety_monitor.unwrap_or(false);
                    if ui
                        .checkbox(&mut is_safety, "安全監控點 (Safety Monitor)")
                        .changed()
                    {
                        tag.safety_monitor = if is_safety { Some(true) } else { None };
                    }
                });

                ui.horizontal(|ui| {
                    let mut is_spare = tag.spare.unwrap_or(false);
                    if ui.checkbox(&mut is_spare, "備用點 (Spare)").changed() {
                        tag.spare = if is_spare { Some(true) } else { None };
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        cancel_clicked = true;
                    }

                    if ui
                        .button(
                            RichText::new("儲存變更")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        save_clicked = true;
                    }
                });
            });

        if save_clicked {
            if let Some(pos) = self.project.tags.iter().position(|t| t.id == tag.id) {
                self.project.tags[pos] = tag;
            } else {
                self.project.tags.push(tag);
            }
            self.trigger_validation();
            self.show_tag_edit_modal = false;
            self.editing_tag = None;
            self.set_toast("已儲存 Tag 變更", Color32::LIGHT_GREEN);
        } else if cancel_clicked || !open {
            self.show_tag_edit_modal = false;
            self.editing_tag = None;
        } else {
            self.editing_tag = Some(tag);
        }
    }

    fn render_type_editor_modal(&mut self, ctx: &egui::Context) {
        if !self.type_editor_state.is_open {
            return;
        }

        let mut open = true;
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        let available_types: Vec<String> = self
            .project
            .catalog
            .iter()
            .map(|ut| ut.id.clone())
            .collect();

        let window_title = if self.type_editor_state.is_new {
            "➕ 新增自訂 Unit 類型"
        } else {
            "✏️ 編輯自訂 Unit 類型"
        };

        egui::Window::new(window_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([1040.0, 740.0])
            .min_width(880.0)
            .min_height(520.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 基本資訊列
                ui.horizontal(|ui| {
                    ui.label(RichText::new("類型識別碼 (ID):").strong());
                    let id_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.type_editor_state.draft.id)
                            .return_key(None)
                            .desired_width(180.0)
                            .hint_text("例如: custom_feeder"),
                    );
                    if id_edit.changed()
                        && self.type_editor_state.current_tab == TypeEditTab::VisualForm
                    {
                        self.type_editor_state.validate();
                    }

                    ui.separator();
                    ui.label(RichText::new("版本 (version):").strong());
                    let ver_edit = ui.add(
                        egui::DragValue::new(&mut self.type_editor_state.draft.version)
                            .range(1..=9999),
                    );
                    if ver_edit.changed()
                        && self.type_editor_state.current_tab == TypeEditTab::VisualForm
                    {
                        self.type_editor_state.validate();
                    }

                    ui.separator();
                    ui.label(RichText::new("顯示名稱 (title):").strong());
                    let title_edit = ui.add(
                        egui::TextEdit::singleline(&mut self.type_editor_state.draft.title)
                            .return_key(None)
                            .desired_width(220.0)
                            .hint_text("例如: 震動送料機單元"),
                    );
                    if title_edit.changed()
                        && self.type_editor_state.current_tab == TypeEditTab::VisualForm
                    {
                        self.type_editor_state.validate();
                    }
                });

                ui.add_space(8.0);

                // 標籤切換（視覺表單 / YAML 原始碼）
                let prev_tab = self.type_editor_state.current_tab;
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.type_editor_state.current_tab,
                        TypeEditTab::VisualForm,
                        RichText::new("📊 視覺化表單 (Visual Form)").strong(),
                    );
                    ui.selectable_value(
                        &mut self.type_editor_state.current_tab,
                        TypeEditTab::RawYaml,
                        RichText::new("📝 YAML 原始碼 (Raw YAML)").strong(),
                    );
                });

                if prev_tab != self.type_editor_state.current_tab {
                    if self.type_editor_state.current_tab == TypeEditTab::RawYaml {
                        self.type_editor_state.sync_form_to_yaml();
                    } else {
                        let _ = self.type_editor_state.sync_yaml_to_form();
                    }
                }

                ui.separator();

                match self.type_editor_state.current_tab {
                    TypeEditTab::VisualForm => {
                        let mut form_changed = false;
                        let mut remove_in = None;
                        let mut remove_out = None;
                        let mut remove_ctrl = None;
                        let mut remove_part = None;

                        egui::ScrollArea::both()
                            .id_salt("type_editor_visual_scroll")
                            .auto_shrink([false, false])
                            .max_height(480.0)
                            .show(ui, |ui| {
                                // 1. Field In 區塊
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("📥 輸入腳位 (field.in)")
                                                .strong()
                                                .color(Color32::LIGHT_BLUE),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        RichText::new("➕ 新增輸入腳位")
                                                            .color(Color32::LIGHT_GREEN),
                                                    )
                                                    .clicked()
                                                {
                                                    let next_idx =
                                                        self.type_editor_state.draft.field_in.len()
                                                            + 1;
                                                    self.type_editor_state.draft.field_in.push(
                                                        crate::type_editor::FieldPinDraft {
                                                            name: format!("sens_{}", next_idx),
                                                            ty: "bool".to_string(),
                                                            required: true,
                                                            label: String::new(),
                                                            unit: String::new(),
                                                        },
                                                    );
                                                    form_changed = true;
                                                }
                                            },
                                        );
                                    });

                                    if self.type_editor_state.draft.field_in.is_empty() {
                                        ui.label(
                                            RichText::new("（無輸入腳位）")
                                                .italics()
                                                .color(Color32::GRAY),
                                        );
                                    } else {
                                        egui::Grid::new("field_in_grid")
                                            .num_columns(6)
                                            .spacing([10.0, 6.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new("腳位名稱").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("資料型別").strong().small(),
                                                );
                                                ui.label(RichText::new("必要").strong().small());
                                                ui.label(
                                                    RichText::new("中文標籤 (Label)").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("工程單位 (Unit)").strong().small(),
                                                );
                                                ui.label(RichText::new("操作").strong().small());
                                                ui.end_row();

                                                for (idx, pin) in self
                                                    .type_editor_state
                                                    .draft
                                                    .field_in
                                                    .iter_mut()
                                                    .enumerate()
                                                {
                                                    if ui
                                                        .add_sized(
                                                            [130.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.name,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fin_name_{}", idx))
                                                            .hint_text("例如: sens1"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    egui::ComboBox::from_id_salt(format!(
                                                        "fin_ty_{}",
                                                        idx
                                                    ))
                                                    .selected_text(&pin.ty)
                                                    .width(90.0)
                                                    .show_ui(ui, |ui| {
                                                        for t in &[
                                                            "bool",
                                                            "int16",
                                                            "int32",
                                                            "float32",
                                                            "string:32",
                                                        ] {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut pin.ty,
                                                                    t.to_string(),
                                                                    *t,
                                                                )
                                                                .clicked()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        }
                                                    });

                                                    if ui.checkbox(&mut pin.required, "").changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .add_sized(
                                                            [240.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.label,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fin_lbl_{}", idx))
                                                            .hint_text("例如: 氣缸伸出到位檢測開關"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .add_sized(
                                                            [110.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.unit,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fin_unit_{}", idx))
                                                            .hint_text("例如: mm、kPa"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .button(
                                                            RichText::new("❌").color(Color32::RED),
                                                        )
                                                        .clicked()
                                                    {
                                                        remove_in = Some(idx);
                                                        form_changed = true;
                                                    }
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                });

                                ui.add_space(8.0);

                                // 2. Field Out 區塊
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("📤 輸出腳位 (field.out)")
                                                .strong()
                                                .color(Color32::from_rgb(255, 180, 80)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        RichText::new("➕ 新增輸出腳位")
                                                            .color(Color32::LIGHT_GREEN),
                                                    )
                                                    .clicked()
                                                {
                                                    let next_idx = self
                                                        .type_editor_state
                                                        .draft
                                                        .field_out
                                                        .len()
                                                        + 1;
                                                    self.type_editor_state.draft.field_out.push(
                                                        crate::type_editor::FieldPinDraft {
                                                            name: format!("act_{}", next_idx),
                                                            ty: "bool".to_string(),
                                                            required: true,
                                                            label: String::new(),
                                                            unit: String::new(),
                                                        },
                                                    );
                                                    form_changed = true;
                                                }
                                            },
                                        );
                                    });

                                    if self.type_editor_state.draft.field_out.is_empty() {
                                        ui.label(
                                            RichText::new("（無輸出腳位）")
                                                .italics()
                                                .color(Color32::GRAY),
                                        );
                                    } else {
                                        egui::Grid::new("field_out_grid")
                                            .num_columns(6)
                                            .spacing([10.0, 6.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new("腳位名稱").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("資料型別").strong().small(),
                                                );
                                                ui.label(RichText::new("必要").strong().small());
                                                ui.label(
                                                    RichText::new("中文標籤 (Label)").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("工程單位 (Unit)").strong().small(),
                                                );
                                                ui.label(RichText::new("操作").strong().small());
                                                ui.end_row();

                                                for (idx, pin) in self
                                                    .type_editor_state
                                                    .draft
                                                    .field_out
                                                    .iter_mut()
                                                    .enumerate()
                                                {
                                                    if ui
                                                        .add_sized(
                                                            [130.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.name,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fout_name_{}", idx))
                                                            .hint_text("例如: act1"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    egui::ComboBox::from_id_salt(format!(
                                                        "fout_ty_{}",
                                                        idx
                                                    ))
                                                    .selected_text(&pin.ty)
                                                    .width(90.0)
                                                    .show_ui(ui, |ui| {
                                                        for t in &[
                                                            "bool",
                                                            "int16",
                                                            "int32",
                                                            "float32",
                                                            "string:32",
                                                        ] {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut pin.ty,
                                                                    t.to_string(),
                                                                    *t,
                                                                )
                                                                .clicked()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        }
                                                    });

                                                    if ui.checkbox(&mut pin.required, "").changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .add_sized(
                                                            [240.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.label,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fout_lbl_{}", idx))
                                                            .hint_text("例如: 氣缸伸出電磁閥線圈"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .add_sized(
                                                            [110.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.unit,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("fout_unit_{}", idx))
                                                            .hint_text("例如: mm、kPa"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .button(
                                                            RichText::new("❌").color(Color32::RED),
                                                        )
                                                        .clicked()
                                                    {
                                                        remove_out = Some(idx);
                                                        form_changed = true;
                                                    }
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                });

                                ui.add_space(8.0);

                                // 3. Ctrl Pins 區塊
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("⚙️ 控制與警報腳位 (ctrl)")
                                                .strong()
                                                .color(Color32::from_rgb(100, 220, 180)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        RichText::new("➕ 新增控制腳位")
                                                            .color(Color32::LIGHT_GREEN),
                                                    )
                                                    .clicked()
                                                {
                                                    let next_idx =
                                                        self.type_editor_state.draft.ctrl.len() + 1;
                                                    self.type_editor_state.draft.ctrl.push(
                                                        crate::type_editor::CtrlPinDraft {
                                                            prefix: "cmd".to_string(),
                                                            suffix: format!("action_{}", next_idx),
                                                            ty: "bool".to_string(),
                                                            label: String::new(),
                                                            unit: String::new(),
                                                            min: String::new(),
                                                            max: String::new(),
                                                            default_val: String::new(),
                                                            message: String::new(),
                                                        },
                                                    );
                                                    form_changed = true;
                                                }
                                            },
                                        );
                                    });

                                    let prefixes = [
                                        ("cmd", "命令 (bool)"),
                                        ("man", "手動 (bool)"),
                                        ("ena", "允許 (bool)"),
                                        ("par", "參數 (min..max, def)"),
                                        ("at", "到位 (bool)"),
                                        ("sts", "狀態 (bool)"),
                                        ("error", "警報 (message)"),
                                        ("count", "計數 (整數)"),
                                        ("pv", "製程值 (數值)"),
                                        ("msg", "訊息 (string)"),
                                    ];

                                    if self.type_editor_state.draft.ctrl.is_empty() {
                                        ui.label(
                                            RichText::new("（無控制腳位）")
                                                .italics()
                                                .color(Color32::GRAY),
                                        );
                                    } else {
                                        egui::Grid::new("ctrl_pins_grid")
                                            .num_columns(6)
                                            .spacing([10.0, 6.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new("前綴 (語意)").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("腳位後綴").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("完整識別名稱").strong().small(),
                                                );
                                                ui.label(
                                                    RichText::new("專屬設定 (參數範圍 / 警報說明)")
                                                        .strong()
                                                        .small(),
                                                );
                                                ui.label(
                                                    RichText::new("中文標籤 (Label)").strong().small(),
                                                );
                                                ui.label(RichText::new("操作").strong().small());
                                                ui.end_row();

                                                for (idx, pin) in self
                                                    .type_editor_state
                                                    .draft
                                                    .ctrl
                                                    .iter_mut()
                                                    .enumerate()
                                                {
                                                    egui::ComboBox::from_id_salt(format!(
                                                        "ctrl_prefix_{}",
                                                        idx
                                                    ))
                                                    .selected_text(&pin.prefix)
                                                    .width(80.0)
                                                    .show_ui(ui, |ui| {
                                                        for (p, desc) in &prefixes {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut pin.prefix,
                                                                    p.to_string(),
                                                                    *desc,
                                                                )
                                                                .clicked()
                                                            {
                                                                if *p == "error"
                                                                    && pin.message.is_empty()
                                                                {
                                                                    pin.message =
                                                                        "異常警報說明".to_string();
                                                                } else if *p == "par"
                                                                    && pin.ty.is_empty()
                                                                {
                                                                    pin.ty = "int16".to_string();
                                                                } else if *p == "pv"
                                                                    && pin.ty.is_empty()
                                                                {
                                                                    pin.ty = "float32".to_string();
                                                                }
                                                                form_changed = true;
                                                            }
                                                        }
                                                    });

                                                    if ui
                                                        .add_sized(
                                                            [120.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.suffix,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("ctrl_suffix_{}", idx))
                                                            .hint_text("例如: start"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    let full_name =
                                                        format!("{}_{}", pin.prefix, pin.suffix);
                                                    ui.label(
                                                        RichText::new(full_name)
                                                            .monospace()
                                                            .strong(),
                                                    );

                                                    // 專屬屬性輸入區
                                                    ui.horizontal(|ui| {
                                                        if pin.prefix == "par" {
                                                            ui.label("型別:");
                                                            if ui
                                                                .add_sized(
                                                                    [60.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.ty,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_par_ty_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                            ui.label("Min:");
                                                            if ui
                                                                .add_sized(
                                                                    [50.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.min,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_par_min_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                            ui.label("Max:");
                                                            if ui
                                                                .add_sized(
                                                                    [50.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.max,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_par_max_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                            ui.label("預設:");
                                                            if ui
                                                                .add_sized(
                                                                    [50.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.default_val,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_par_def_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                            ui.label("工程單位:");
                                                            if ui
                                                                .add_sized(
                                                                    [80.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.unit,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_par_unit_{}", idx))
                                                                    .hint_text("單位"),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        } else if pin.prefix == "error" {
                                                            ui.label("警報說明*:");
                                                            if ui
                                                                .add_sized(
                                                                    [240.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.message,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_err_msg_{}", idx))
                                                                    .hint_text("必填警報文字"),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        } else if pin.prefix == "pv"
                                                            || pin.prefix == "msg"
                                                        {
                                                            ui.label("型別:");
                                                            if ui
                                                                .add_sized(
                                                                    [65.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.ty,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_pv_ty_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                            ui.label("工程單位:");
                                                            if ui
                                                                .add_sized(
                                                                    [80.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.unit,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_pv_unit_{}", idx))
                                                                    .hint_text("單位"),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        } else {
                                                            ui.label("型別:");
                                                            if ui
                                                                .add_sized(
                                                                    [60.0, 22.0],
                                                                    egui::TextEdit::singleline(
                                                                        &mut pin.ty,
                                                                    )
                                                                    .return_key(None)
                                                                    .id_salt(format!("ctrl_gen_ty_{}", idx)),
                                                                )
                                                                .changed()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        }
                                                    });

                                                    if ui
                                                        .add_sized(
                                                            [220.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut pin.label,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("ctrl_lbl_{}", idx))
                                                            .hint_text("例如: 手動觸發動作"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    if ui
                                                        .button(
                                                            RichText::new("❌").color(Color32::RED),
                                                        )
                                                        .clicked()
                                                    {
                                                        remove_ctrl = Some(idx);
                                                        form_changed = true;
                                                    }
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                });

                                ui.add_space(8.0);

                                // 4. Parts 區塊
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("🧩 次級單元插槽 (parts)")
                                                .strong()
                                                .color(Color32::from_rgb(220, 150, 255)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        RichText::new("➕ 新增次級插槽")
                                                            .color(Color32::LIGHT_GREEN),
                                                    )
                                                    .clicked()
                                                {
                                                    let next_idx =
                                                        self.type_editor_state.draft.parts.len()
                                                            + 1;
                                                    let target = available_types
                                                        .first()
                                                        .cloned()
                                                        .unwrap_or_else(|| "cylinder".to_string());
                                                    self.type_editor_state.draft.parts.push(
                                                        crate::type_editor::PartSlotDraft {
                                                            slot_name: format!("part_{}", next_idx),
                                                            target_type: target,
                                                        },
                                                    );
                                                    form_changed = true;
                                                }
                                            },
                                        );
                                    });

                                    if self.type_editor_state.draft.parts.is_empty() {
                                        ui.label(
                                            RichText::new("（無次級插槽）")
                                                .italics()
                                                .color(Color32::GRAY),
                                        );
                                    } else {
                                        egui::Grid::new("parts_grid")
                                            .num_columns(3)
                                            .spacing([12.0, 4.0])
                                            .striped(true)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new("插槽名稱 (Slot Name)")
                                                        .strong()
                                                        .small(),
                                                );
                                                ui.label(
                                                    RichText::new(
                                                        "目標 Unit 類型 (Target Unit Type)",
                                                    )
                                                    .strong()
                                                    .small(),
                                                );
                                                ui.label(RichText::new("操作").strong().small());
                                                ui.end_row();

                                                for (idx, slot) in self
                                                    .type_editor_state
                                                    .draft
                                                    .parts
                                                    .iter_mut()
                                                    .enumerate()
                                                {
                                                    if ui
                                                        .add_sized(
                                                            [180.0, 22.0],
                                                            egui::TextEdit::singleline(
                                                                &mut slot.slot_name,
                                                            )
                                                            .return_key(None)
                                                            .id_salt(format!("part_slot_name_{}", idx))
                                                            .hint_text("例如: clamp_cyl"),
                                                        )
                                                        .changed()
                                                    {
                                                        form_changed = true;
                                                    }

                                                    egui::ComboBox::from_id_salt(format!(
                                                        "part_target_{}",
                                                        idx
                                                    ))
                                                    .selected_text(&slot.target_type)
                                                    .width(200.0)
                                                    .show_ui(ui, |ui| {
                                                        for t in &available_types {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut slot.target_type,
                                                                    t.clone(),
                                                                    t,
                                                                )
                                                                .clicked()
                                                            {
                                                                form_changed = true;
                                                            }
                                                        }
                                                    });

                                                    if ui
                                                        .button(
                                                            RichText::new("❌").color(Color32::RED),
                                                        )
                                                        .clicked()
                                                    {
                                                        remove_part = Some(idx);
                                                        form_changed = true;
                                                    }
                                                    ui.end_row();
                                                }
                                            });
                                    }
                                });
                            });

                        if let Some(idx) = remove_in {
                            self.type_editor_state.draft.field_in.remove(idx);
                        }
                        if let Some(idx) = remove_out {
                            self.type_editor_state.draft.field_out.remove(idx);
                        }
                        if let Some(idx) = remove_ctrl {
                            self.type_editor_state.draft.ctrl.remove(idx);
                        }
                        if let Some(idx) = remove_part {
                            self.type_editor_state.draft.parts.remove(idx);
                        }

                        if form_changed {
                            self.type_editor_state.validate();
                        }
                    }

                    TypeEditTab::RawYaml => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("YAML 原始碼編輯區（支援完整欄位與手動調整）")
                                    .color(Color32::LIGHT_BLUE),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(
                                            RichText::new("🔄 重新解析並套用至視覺表單")
                                                .color(Color32::LIGHT_GREEN),
                                        )
                                        .clicked()
                                    {
                                        match self.type_editor_state.sync_yaml_to_form() {
                                            Ok(_) => {
                                                self.set_toast(
                                                    "已成功套用至視覺表單！",
                                                    Color32::LIGHT_GREEN,
                                                );
                                            }
                                            Err(_) => {
                                                self.set_toast(
                                                    "YAML 解析失敗，請檢查錯誤",
                                                    Color32::RED,
                                                );
                                            }
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt("raw_yaml_scroll")
                            .max_height(420.0)
                            .show(ui, |ui| {
                                if ui
                                    .add(
                                        egui::TextEdit::multiline(
                                            &mut self.type_editor_state.raw_yaml,
                                        )
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(24),
                                    )
                                    .changed()
                                {
                                    match crate::type_editor::UnitTypeDraft::from_yaml_str(
                                        &self.type_editor_state.raw_yaml,
                                    ) {
                                        Ok(_) => {
                                            self.type_editor_state.validation_errors.clear();
                                        }
                                        Err(diags) => {
                                            self.type_editor_state.validation_errors = diags;
                                        }
                                    }
                                }
                            });
                    }
                }

                ui.separator();

                // 即時規格驗證回饋區 (SC2 / SC3 檢核)
                if self.type_editor_state.validation_errors.is_empty() {
                    ui.colored_label(
                        Color32::LIGHT_GREEN,
                        "✅ 規格檢核通過（完全符合 SC2 結構與 SC3 命名標準）",
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt("type_editor_diags_scroll")
                        .max_height(70.0)
                        .show(ui, |ui| {
                            for d in &self.type_editor_state.validation_errors {
                                ui.colored_label(
                                    Color32::RED,
                                    format!("❌ [{}] {}", d.code, d.message),
                                );
                            }
                        });
                }

                if let Some(ref err) = self.type_editor_state.save_error {
                    ui.colored_label(Color32::RED, format!("❌ 儲存錯誤：{}", err));
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        cancel_clicked = true;
                    }

                    if ui
                        .button(
                            RichText::new("💾 儲存並註冊至類型目錄")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        save_clicked = true;
                    }
                });
            });

        if save_clicked {
            if self.type_editor_state.current_tab == TypeEditTab::RawYaml {
                let _ = self.type_editor_state.sync_yaml_to_form();
            } else {
                self.type_editor_state.sync_form_to_yaml();
            }

            let type_id = self.type_editor_state.draft.id.trim().to_string();
            let yaml = self.type_editor_state.raw_yaml.clone();

            if type_id.is_empty() {
                self.type_editor_state.save_error = Some("類型 ID 不可為空".to_string());
                return;
            }

            match self.project.save_custom_type(&type_id, &yaml) {
                Ok(_) => {
                    self.trigger_validation();
                    self.type_editor_state.is_open = false;
                    self.set_toast(
                        format!("已成功儲存自訂 Unit 類型「{}」！", type_id),
                        Color32::LIGHT_GREEN,
                    );
                }
                Err(diags) => {
                    let msg = diags
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.type_editor_state.validation_errors = diags;
                    self.type_editor_state.save_error = Some(msg);
                    self.set_toast("儲存失敗：規格檢核未通過", Color32::RED);
                }
            }
        } else if cancel_clicked || !open {
            self.type_editor_state.is_open = false;
        }
    }

    fn render_delete_type_confirm_modal(&mut self, ctx: &egui::Context) {
        let Some(type_id) = self.show_delete_type_confirm.clone() else {
            return;
        };

        let in_use = self.project.units_using_type(&type_id);
        let is_standard = self.project.is_standard_type(&type_id);

        let mut open = true;
        let mut confirm_delete = false;
        let mut close_modal = false;

        egui::Window::new("⚠️ 刪除 Unit 類型確認")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if is_standard {
                    ui.colored_label(
                        Color32::RED,
                        format!("❌ 類型「{}」為核心標準類型，不可刪除！", type_id),
                    );
                    ui.add_space(8.0);
                    if ui.button("關閉").clicked() {
                        close_modal = true;
                    }
                } else if !in_use.is_empty() {
                    ui.colored_label(Color32::RED, format!("❌ 無法刪除類型「{}」！", type_id));
                    ui.label(RichText::new("專案中目前有以下 Unit 正在使用此類型：").strong());
                    for u in &in_use {
                        ui.label(format!("  • Unit: {}", u));
                    }
                    ui.add_space(4.0);
                    ui.label("請先修改或刪除上述 Unit 後，才能刪除此類型。");
                    ui.add_space(8.0);
                    if ui.button("關閉").clicked() {
                        close_modal = true;
                    }
                } else {
                    ui.label(format!("您確定要刪除自訂 Unit 類型「{}」嗎？", type_id));
                    ui.colored_label(
                        Color32::YELLOW,
                        "⚠️ 此動作將從自訂庫中移除該類型定義，無法復原。",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("取消").clicked() {
                            close_modal = true;
                        }
                        if ui
                            .button(
                                RichText::new("確定刪除")
                                    .strong()
                                    .color(Color32::from_rgb(255, 100, 100)),
                            )
                            .clicked()
                        {
                            confirm_delete = true;
                        }
                    });
                }
            });

        if confirm_delete {
            match self.project.delete_custom_type(&type_id) {
                Ok(_) => {
                    self.trigger_validation();
                    self.show_delete_type_confirm = None;
                    self.set_toast(
                        format!("已成功刪除 Unit 類型「{}」", type_id),
                        Color32::LIGHT_GREEN,
                    );
                }
                Err(e) => {
                    self.set_toast(format!("刪除失敗：{}", e), Color32::RED);
                }
            }
        } else if close_modal || !open {
            self.show_delete_type_confirm = None;
        }
    }

    fn render_export_type_modal(&mut self, ctx: &egui::Context) {
        if !self.show_export_type_modal {
            return;
        }

        let mut open = true;
        let mut do_export = false;
        let mut close_modal = false;

        egui::Window::new("💾 匯出 Unit 類型至 .yaml 檔案")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(550.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Unit 類型 ID:").strong());
                    ui.label(
                        RichText::new(&self.export_type_id)
                            .color(Color32::LIGHT_BLUE)
                            .strong(),
                    );
                });

                ui.add_space(6.0);
                ui.label(RichText::new("儲存檔案路徑:").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut self.export_type_path)
                        .return_key(None)
                        .desired_width(f32::INFINITY)
                        .hint_text("例如: custom_types/my_unit.yaml"),
                );

                ui.add_space(8.0);
                ui.label(RichText::new("YAML 內容預覽:").strong());
                egui::ScrollArea::vertical()
                    .id_salt("export_type_yaml_scroll")
                    .max_height(250.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.export_type_yaml)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        close_modal = true;
                    }

                    if ui
                        .button(
                            RichText::new("寫入磁碟並匯出")
                                .strong()
                                .color(Color32::LIGHT_GREEN),
                        )
                        .clicked()
                    {
                        do_export = true;
                    }
                });
            });

        if do_export {
            let path = std::path::Path::new(&self.export_type_path);
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&self.export_type_path, &self.export_type_yaml) {
                Ok(_) => {
                    self.show_export_type_modal = false;
                    self.set_toast(
                        format!("已成功匯出至 {}", self.export_type_path),
                        Color32::LIGHT_GREEN,
                    );
                }
                Err(e) => {
                    self.set_toast(format!("匯出失敗：{}", e), Color32::RED);
                }
            }
        } else if close_modal || !open {
            self.show_export_type_modal = false;
        }
    }

    fn render_toast_overlay(&mut self, ctx: &egui::Context) {
        if let Some((msg, color, start_time)) = &self.toast {
            if start_time.elapsed().as_secs_f32() > 3.0 {
                self.toast = None;
                return;
            }

            egui::Area::new(egui::Id::new("toast_overlay"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -40.0])
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(30, 36, 46))
                        .stroke(Stroke::new(1.0_f32, *color))
                        .inner_margin(egui::Margin::symmetric(14, 8))
                        .show(ui, |ui| {
                            ui.label(RichText::new(msg).color(*color).strong());
                        });
                });
        }
    }
}
