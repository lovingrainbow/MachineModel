//! 用法：`model-diff [--strict] <fixture 資料夾>...`
//!
//! 每個資料夾放 `machine.yaml`，可選 `types/*.yaml`。兩邊各自載入、驗證、推導，
//! 比對診斷（規則編號與是否為錯誤）與每個 Tag 的 role、write_candidate、唯讀位址旗標。
//! 比對的是兩邊的真實輸出，不是手寫期望值。`--strict` 時有差異就以 1 結束。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
struct Side {
    /// (規則編號, 是否為錯誤)
    diags: BTreeSet<(String, bool)>,
    /// Tag id → (role, write_candidate, 唯讀位址, 綁定數)
    tags: BTreeMap<String, (String, bool, bool, usize)>,
    /// 載入／推導成功（沒有錯誤診斷）。
    resolved: bool,
}

fn norm(dbg: &dyn std::fmt::Debug) -> String {
    format!("{dbg:?}").to_lowercase().replace('_', "")
}

fn read_dir_yaml(dir: &Path) -> Vec<(String, String)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    v.sort();
    v.into_iter()
        .filter_map(|p| Some((p.display().to_string(), std::fs::read_to_string(&p).ok()?)))
        .collect()
}

fn run_mm(dir: &Path, text: &str) -> Side {
    let mut side = Side::default();
    let locals = read_dir_yaml(&dir.join("types"));
    let refs: Vec<(&str, &str)> = locals
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let catalog = match mm::catalog::UnitCatalog::standard().with_local(&refs) {
        Ok((c, _)) => c,
        Err(ds) => {
            for d in ds {
                side.diags.insert((d.code.to_owned(), d.is_error()));
            }
            return side;
        }
    };
    match mm::load_checked(text, &catalog) {
        Ok(checked) => {
            for d in &checked.warnings {
                side.diags.insert((d.code.to_owned(), d.is_error()));
            }
            side.resolved = true;
            for t in &checked.resolved.tags {
                side.tags.insert(
                    t.id.clone(),
                    (
                        t.role.as_ref().map_or("-".into(), |r| norm(r)),
                        t.write_candidate,
                        t.class_read_only,
                        t.bindings.len(),
                    ),
                );
            }
        }
        Err(ds) => {
            for d in ds {
                side.diags.insert((d.code.to_owned(), d.is_error()));
            }
        }
    }
    side
}

fn run_ds(dir: &Path, text: &str) -> Side {
    use ds::diagnostics::has_errors;
    let mut side = Side::default();
    let machine = match ds::model::MachineFile::from_yaml_str(text) {
        Ok(m) => m,
        Err(d) => {
            if std::env::var_os("VERBOSE").is_some() {
                eprintln!("DataService 解析失敗：{d}");
            }
            side.diags.insert((d.rule.clone(), true));
            return side;
        }
    };
    let mut catalog = ds::unit_type::TypeCatalog::with_embedded_standards();
    catalog.load_local_types_from_dir(dir);
    let diags = ds::validator::validate_machine_with_catalog(&machine, &catalog);
    for d in &diags {
        side.diags.insert((d.rule.clone(), d.is_error()));
    }
    if has_errors(&diags) {
        return side;
    }
    match ds::resolver::resolve_machine_model(&machine, &catalog) {
        Ok(r) => {
            side.resolved = true;
            for t in &r.effective_tags {
                side.tags.insert(
                    t.id.clone(),
                    (
                        t.role.as_ref().map_or("-".into(), |r| norm(r)),
                        t.write_candidate,
                        t.address_read_only,
                        t.bindings.len(),
                    ),
                );
            }
        }
        Err(ds) => {
            for d in ds {
                side.diags.insert((d.rule.clone(), d.is_error()));
            }
        }
    }
    side
}

/// 已知、由設計造成的雜訊：本地類型覆蓋標準類型時，DataService 會多報 U16／U17 提示；
/// fixture 為了讓兩邊用同一份類型定義，一律用本地類型覆蓋，所以會固定出現。
fn is_known_noise(d: &(String, bool)) -> bool {
    !d.1 && (d.0 == "U16" || d.0 == "U17")
}

/// 回傳此 fixture 的差異行數。
fn compare(name: &str, mm: &Side, ds: &Side) -> usize {
    let mm = &Side {
        diags: mm
            .diags
            .iter()
            .filter(|d| !is_known_noise(d))
            .cloned()
            .collect(),
        tags: mm.tags.clone(),
        resolved: mm.resolved,
    };
    let ds = &Side {
        diags: ds
            .diags
            .iter()
            .filter(|d| !is_known_noise(d))
            .cloned()
            .collect(),
        tags: ds.tags.clone(),
        resolved: ds.resolved,
    };
    let mut lines = Vec::new();
    for d in mm.diags.difference(&ds.diags) {
        lines.push(format!(
            "  只有 MachineModel 報 {} ({})",
            d.0,
            if d.1 { "錯誤" } else { "提示" }
        ));
    }
    for d in ds.diags.difference(&mm.diags) {
        lines.push(format!(
            "  只有 DataService 報 {} ({})",
            d.0,
            if d.1 { "錯誤" } else { "提示" }
        ));
    }
    if mm.resolved != ds.resolved {
        lines.push(format!(
            "  推導結果：MachineModel={} DataService={}",
            mm.resolved, ds.resolved
        ));
    }
    if mm.resolved && ds.resolved {
        let ids: BTreeSet<&String> = mm.tags.keys().chain(ds.tags.keys()).collect();
        for id in ids {
            match (mm.tags.get(id), ds.tags.get(id)) {
                (Some(a), Some(b)) if a != b => {
                    lines.push(format!(
                        "  Tag {id}：MM={a:?} DS={b:?}  (role, write_candidate, 唯讀位址, 綁定數)"
                    ));
                }
                (Some(_), None) => lines.push(format!("  Tag {id}：只在 MachineModel")),
                (None, Some(_)) => lines.push(format!("  Tag {id}：只在 DataService")),
                _ => {}
            }
        }
    }
    if lines.is_empty() {
        let codes: Vec<String> = mm
            .diags
            .iter()
            .map(|d| format!("{}{}", d.0, if d.1 { "" } else { "(提示)" }))
            .collect();
        println!(
            "[相同] {name}  診斷 [{}]，Tag {} 個",
            codes.join(" "),
            mm.tags.len()
        );
    } else {
        let fmt = |s: &Side| {
            s.diags
                .iter()
                .map(|d| format!("{}{}", d.0, if d.1 { "" } else { "(提示)" }))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!("[差異] {name}   MM=[{}]  DS=[{}]", fmt(mm), fmt(ds));
        for l in &lines {
            println!("{l}");
        }
    }
    lines.len()
}

fn main() {
    let mut strict = false;
    let mut dirs = Vec::new();
    for a in std::env::args().skip(1) {
        if a == "--strict" {
            strict = true;
        } else {
            dirs.push(PathBuf::from(a));
        }
    }
    let mut total = 0;
    for dir in &dirs {
        let name = dir.file_name().map_or_else(
            || dir.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let Ok(text) = std::fs::read_to_string(dir.join("machine.yaml")) else {
            eprintln!("讀不到 {}/machine.yaml", dir.display());
            std::process::exit(2);
        };
        total += compare(&name, &run_mm(dir, &text), &run_ds(dir, &text));
    }
    println!("共 {total} 項差異");
    if strict && total > 0 {
        std::process::exit(1);
    }
}
