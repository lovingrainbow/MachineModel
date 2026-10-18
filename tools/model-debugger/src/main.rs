use eframe::egui;

mod app;
mod io_import;
mod model;
mod type_editor;

use app::ModelDebuggerApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_inner_size([1360.0, 840.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("MachineModel Debugger & Studio v2"),
        ..Default::default()
    };

    eframe::run_native(
        "MachineModel Studio",
        native_options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ModelDebuggerApp::default()))
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidates = [
        // Windows system fonts
        "C:\\Windows\\Fonts\\msjh.ttc", // Microsoft JhengHei (微軟正黑體)
        "C:\\Windows\\Fonts\\msyh.ttc", // Microsoft YaHei (微軟雅黑)
        "C:\\Windows\\Fonts\\simsun.ttc", // SimSun
        // WSL mapped paths
        "/mnt/c/Windows/Fonts/msjh.ttc",
        "/mnt/c/Windows/Fonts/msyh.ttc",
        "/mnt/c/Windows/Fonts/simsun.ttc",
        // Linux system fonts
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ];

    for path in &candidates {
        if let Ok(font_data) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk_fallback".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(font_data)),
            );

            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "cjk_fallback".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "cjk_fallback".to_owned());
            }
            break;
        }
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Heading,
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Name("Subheading".into()),
            egui::FontId::new(17.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::new(14.5, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::new(12.5, egui::FontFamily::Proportional),
        ),
    ]
    .into();
    ctx.set_style(style);
    ctx.set_zoom_factor(1.15);
}

#[cfg(test)]
mod tests {
    use crate::io_import::{generate_inovance_easy_preset, parse_delimited, parse_yaml_tags};
    use crate::model::ProjectModel;

    #[test]
    fn test_cjk_font_loading() {
        let ctx = eframe::egui::Context::default();
        super::setup_custom_fonts(&ctx);
        let _ = ctx.run(eframe::egui::RawInput::default(), |ctx| {
            let font_id = eframe::egui::FontId::proportional(14.0);
            let galley = ctx.fonts(|f| {
                f.layout_no_wrap(
                    "中文標籤測試".to_string(),
                    font_id,
                    eframe::egui::Color32::WHITE,
                )
            });
            println!("galley text: {}, size: {:?}", galley.text(), galley.size());
            assert!(!galley.rows.is_empty());
        });
    }

    #[test]
    fn test_textedit_enter_retains_focus_with_return_key_none() {
        let ctx = eframe::egui::Context::default();
        let mut text = "測試標籤".to_string();

        // 1. First frame: focus the widget
        let _ = ctx.run(eframe::egui::RawInput::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let resp = ui.add(
                    eframe::egui::TextEdit::singleline(&mut text)
                        .id_salt("lbl_test")
                        .return_key(None),
                );
                resp.request_focus();
            });
        });

        // 2. Second frame: simulate pressing Enter key (as sent by IME candidate confirmation)
        let mut raw_input = eframe::egui::RawInput::default();
        raw_input.events.push(eframe::egui::Event::Key {
            key: eframe::egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: eframe::egui::Modifiers::NONE,
        });

        let _ = ctx.run(raw_input, |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let resp = ui.add(
                    eframe::egui::TextEdit::singleline(&mut text)
                        .id_salt("lbl_test")
                        .return_key(None),
                );
                // With return_key(None), it MUST NOT lose focus!
                assert!(
                    !resp.lost_focus(),
                    "return_key(None) should prevent losing focus on Enter"
                );
                assert!(resp.has_focus(), "Widget should retain focus after Enter");
            });
        });
    }

    #[test]
    fn test_demo_project_loads_and_passes_validation() {
        let p = ProjectModel::demo();
        let (res, yaml) = p.validate();
        assert!(
            res.is_ok(),
            "示範專案必須通過驗證：{:?}\nYAML:\n{}",
            res.err(),
            yaml
        );
        let checked = res.unwrap();
        assert_eq!(checked.resolved.machine_id, "demo");
        assert_eq!(checked.resolved.units.len(), 3);
        assert_eq!(checked.resolved.tags.len(), 13);
    }

    #[test]
    fn test_csv_import_and_preset() {
        let csv_data = "address,id,type,label\nX0,Sens1,bool,入口開關\nY0,Valve1,bool,主閥\nD100,Speed,int16,轉速設定";
        let tags = parse_delimited(csv_data, "plc1").expect("CSV 應可成功解析");
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].id, "plc1.Sens1");
        assert_eq!(tags[0].address, "X0");
        assert_eq!(tags[0].ty, "bool");
        assert_eq!(tags[0].label.as_deref(), Some("入口開關"));

        let preset = generate_inovance_easy_preset("plc1");
        assert!(!preset.is_empty());
        assert!(preset.iter().any(|t| t.address == "X00"));
        assert!(preset.iter().any(|t| t.address == "Y00"));
    }

    #[test]
    fn test_yaml_tags_import() {
        let yaml_data = "- { id: plc1.X20, address: X20, type: bool, label: 原點 }\n- { id: plc1.Y20, address: Y20, type: bool }";
        let tags = parse_yaml_tags(yaml_data).expect("YAML tags 應可解析");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "plc1.X20");
        assert_eq!(tags[1].address, "Y20");
    }

    #[test]
    fn test_unit_crud_and_binding() {
        let mut p = ProjectModel::demo();
        assert_eq!(p.units.len(), 3);

        // 新增 Unit
        p.add_unit("cyl2".to_string(), "cylinder_double".to_string(), 1);
        assert_eq!(p.units.len(), 4);
        let cyl2 = p.units.iter().find(|u| u.id == "cyl2").unwrap();
        assert!(cyl2.bind.contains_key("lim_ext"));
        assert!(cyl2.bind.contains_key("valve_ext"));

        // 重新命名 Unit
        p.rename_unit("cyl2", "cyl2_renamed");
        assert!(p.units.iter().any(|u| u.id == "cyl2_renamed"));
        assert!(!p.units.iter().any(|u| u.id == "cyl2"));

        // 刪除 Unit
        p.delete_unit("cyl2_renamed");
        assert_eq!(p.units.len(), 3);
    }

    #[test]
    fn test_project_new_empty_and_roundtrip() {
        use machine_model::family::Family;

        let mut p = ProjectModel::new_empty(
            "test_machine",
            Some("測試機台".to_string()),
            Family::MelsecFxSerialTcp,
        );
        assert_eq!(p.machine_id, "test_machine");
        assert_eq!(p.machine_title.as_deref(), Some("測試機台"));
        assert_eq!(p.plcs.len(), 1);
        assert_eq!(p.plcs[0].family, Family::MelsecFxSerialTcp);
        assert!(p.units.is_empty());
        assert!(p.tags.is_empty());

        // 新增一個 Unit
        p.add_unit("sensor1".to_string(), "photoelectric_sensor".to_string(), 1);
        assert_eq!(p.units.len(), 1);

        // 匯出成 YAML 字串
        let yaml = p.to_yaml_string();
        assert!(yaml.contains("id: test_machine"));
        assert!(yaml.contains("family: melsec_fx_serial_tcp"));
        assert!(yaml.contains("type: photoelectric_sensor"));

        // 讀回 YAML
        let loaded = ProjectModel::from_yaml_str(&yaml, p.catalog.clone())
            .expect("YAML 應可還原為 ProjectModel");
        assert_eq!(loaded.machine_id, "test_machine");
        assert_eq!(loaded.machine_title.as_deref(), Some("測試機台"));
        assert_eq!(loaded.plcs[0].family, Family::MelsecFxSerialTcp);
        assert_eq!(loaded.units.len(), 1);
        assert_eq!(loaded.units[0].id, "sensor1");
    }

    #[test]
    fn test_unit_type_draft_roundtrip_and_validation() {
        use crate::type_editor::{CtrlPinDraft, FieldPinDraft, UnitTypeDraft};

        let mut draft = UnitTypeDraft::new_empty("feeder_v1");
        draft.title = "震動送料機".to_string();
        draft.field_in.push(FieldPinDraft {
            name: "sens_empty".to_string(),
            ty: "bool".to_string(),
            required: true,
            label: "缺料檢知".to_string(),
            unit: String::new(),
        });
        draft.field_out.push(FieldPinDraft {
            name: "act_motor".to_string(),
            ty: "bool".to_string(),
            required: true,
            label: "震動馬達輸出".to_string(),
            unit: String::new(),
        });
        draft.ctrl.push(CtrlPinDraft {
            prefix: "cmd".to_string(),
            suffix: "feed".to_string(),
            ty: "bool".to_string(),
            label: "啟動送料".to_string(),
            unit: String::new(),
            min: String::new(),
            max: String::new(),
            default_val: String::new(),
            message: String::new(),
        });
        draft.ctrl.push(CtrlPinDraft {
            prefix: "error".to_string(),
            suffix: "jammed".to_string(),
            ty: "bool".to_string(),
            label: "卡料異常".to_string(),
            unit: String::new(),
            min: String::new(),
            max: String::new(),
            default_val: String::new(),
            message: "震動送料機卡料警報".to_string(),
        });

        // 檢核 SC2 / SC3 規範
        let val_res = draft.validate();
        assert!(val_res.is_ok(), "Draft 應通過核心驗證：{:?}", val_res.err());

        // 轉換為 YAML 並反向解析
        let yaml = draft.to_yaml_string();
        assert!(yaml.contains("id: feeder_v1"));
        assert!(yaml.contains("sens_empty: { type: bool"));
        assert!(yaml.contains("act_motor: { type: bool"));
        assert!(yaml.contains("cmd_feed: { label: 啟動送料 }"));
        assert!(yaml.contains("error_jammed: { message: 震動送料機卡料警報, label: 卡料異常 }"));

        let parsed_draft =
            UnitTypeDraft::from_yaml_str(&yaml).expect("產生的 YAML 應可正確還原為 Draft");
        assert_eq!(parsed_draft.id, "feeder_v1");
        assert_eq!(parsed_draft.title, "震動送料機");
        assert_eq!(parsed_draft.field_in.len(), 2);
        assert_eq!(parsed_draft.field_out.len(), 2);
    }

    #[test]
    fn test_custom_type_crud_and_dependency_guard() {
        use crate::type_editor::UnitTypeDraft;

        let mut p = ProjectModel::demo();

        // 1. 核心標準類型不可刪除
        assert!(p.is_standard_type("cylinder_double"));
        let del_std = p.delete_custom_type("cylinder_double");
        assert!(del_std.is_err(), "標準類型應不可被刪除");

        // 2. 複製標準類型為自訂類型
        let std_cyl = p.catalog.get("cylinder_double").unwrap();
        let mut draft = UnitTypeDraft::from_unit_type(std_cyl);
        draft.id = "custom_cylinder".to_string();
        draft.title = "自訂汽缸".to_string();

        let yaml = draft.to_yaml_string();
        let save_res = p.save_custom_type("custom_cylinder", &yaml);
        assert!(
            save_res.is_ok(),
            "自訂類型應能成功儲存：{:?}",
            save_res.err()
        );

        assert!(!p.is_standard_type("custom_cylinder"));
        assert!(p.catalog.get("custom_cylinder").is_some());

        // 3. 在專案中新增使用該自訂類型的 Unit
        p.add_unit("cyl_custom_1".to_string(), "custom_cylinder".to_string(), 1);
        let in_use = p.units_using_type("custom_cylinder");
        assert_eq!(in_use, vec!["cyl_custom_1".to_string()]);

        // 4. 正在使用中的自訂類型防刪保護
        let del_in_use = p.delete_custom_type("custom_cylinder");
        assert!(del_in_use.is_err(), "使用中的類型應受保護不可刪除");
        assert!(del_in_use.unwrap_err().contains("cyl_custom_1"));

        // 5. 移除該 Unit 後即可順利刪除
        p.delete_unit("cyl_custom_1");
        assert!(p.units_using_type("custom_cylinder").is_empty());

        let del_ok = p.delete_custom_type("custom_cylinder");
        assert!(del_ok.is_ok(), "無 Unit 引用時自訂類型應可成功刪除");
        assert!(p.catalog.get("custom_cylinder").is_none());
    }
}
