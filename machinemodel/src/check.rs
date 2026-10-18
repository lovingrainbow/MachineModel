//! `check` 子命令：載入安全規則與 U 規則檢查（規格 §9.5）。

use machine_model::catalog::UnitCatalog;

/// `machinemodel check <machine.yaml>` 引數。
#[derive(Debug, clap::Args)]
pub struct Args {
    /// 機台設定檔（`machine.yaml` v2）。
    ///
    /// 機台本地類型自動讀取其旁邊的 `types/*.yaml`（同 id 覆蓋標準類型，不提供旗標）。
    pub machine: std::path::PathBuf,
    /// 列印推導報告（每個 Tag 的 `role`、`write_candidate` 與理由）。
    #[arg(long)]
    pub report: bool,
    /// 列印 HMI 覆蓋率缺口報告（未綁定的選擇性 ctrl 腳）。
    #[arg(long)]
    pub coverage: bool,
}

/// 執行檢查：0 通過、1 規則錯誤、2 I/O 錯誤。
pub fn run(args: &Args) -> i32 {
    let text = match std::fs::read_to_string(&args.machine) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("無法讀取 {}：{e}", args.machine.display());
            return 2;
        }
    };
    let catalog = match catalog_beside(&args.machine) {
        Ok(c) => c,
        Err(code) => return code,
    };
    match machine_model::load_checked(&text, &catalog) {
        Ok(checked) => {
            for d in &checked.warnings {
                eprintln!("{d}");
            }
            if args.report {
                print_report(&checked.resolved);
            }
            if args.coverage {
                print_coverage(&checked.resolved);
            }
            0
        }
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            1
        }
    }
}

/// `machine.yaml` 旁的 `types/` 疊加到標準類型目錄上（`check`、`dump` 共用）。
pub(crate) fn catalog_beside(machine: &std::path::Path) -> Result<UnitCatalog, i32> {
    let types_dir = machine
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("types");
    load_catalog(&types_dir)
}

/// 以內嵌標準類型為基礎，疊加 `types/` 目錄的本地類型；目錄不存在視為沒有本地類型。
/// I/O 失敗（含 `types` 不是目錄）回 `Err(2)`。
fn load_catalog(dir: &std::path::Path) -> Result<UnitCatalog, i32> {
    if !dir.exists() {
        return Ok(UnitCatalog::standard());
    }
    let files = read_type_files(dir)?;
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let st_files = read_st_files(dir)?;
    let st_refs: Vec<(&str, &str)> = st_files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let built = UnitCatalog::standard()
        .with_local(&refs)
        .and_then(|(catalog, mut warnings)| {
            catalog.with_st(&st_refs).map(|(c, more)| {
                warnings.extend(more);
                (c, warnings)
            })
        });
    match built {
        Ok((catalog, warnings)) => {
            for d in &warnings {
                eprintln!("{d}");
            }
            Ok(catalog)
        }
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            Err(1)
        }
    }
}

/// 讀取目錄下的 `*.yaml`，依路徑排序。目錄或檔案讀不到回 `Err(2)`。
pub(crate) fn read_type_files(dir: &std::path::Path) -> Result<Vec<(String, String)>, i32> {
    read_files_with_extension(dir, "yaml", |path| path.display().to_string())
}

/// 讀取目錄下的 `*.st` 旁檔，回傳 `(類型 id, 內容)`；類型 id 取檔名（不含副檔名）。
pub(crate) fn read_st_files(dir: &std::path::Path) -> Result<Vec<(String, String)>, i32> {
    read_files_with_extension(dir, "st", |path| {
        path.file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
    })
}

fn read_files_with_extension(
    dir: &std::path::Path,
    extension: &str,
    name_of: impl Fn(&std::path::Path) -> String,
) -> Result<Vec<(String, String)>, i32> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("無法讀取類型目錄 {}：{e}", dir.display());
            return Err(2);
        }
    };
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("無法讀取類型目錄 {}：{e}", dir.display());
                return Err(2);
            }
        };
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case(extension))
        {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => files.push((name_of(&path), text)),
            Err(e) => {
                eprintln!("無法讀取 {}：{e}", path.display());
                return Err(2);
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn print_report(model: &machine_model::resolve::ResolvedModel) {
    println!("tag role write_candidate 綁定");
    for t in &model.tags {
        let role = t.role.map_or("-".to_owned(), |r| format!("{r:?}"));
        let reason = match t.role_source {
            machine_model::resolve::RoleSource::Explicit => "明確指定".to_owned(),
            machine_model::resolve::RoleSource::Pin(p) => {
                format!("接腳前綴 {}", p.info().name)
            }
            machine_model::resolve::RoleSource::Unknown => "未綁定".to_owned(),
        };
        let binds: Vec<String> = t
            .bindings
            .iter()
            .map(|b| format!("{}.{}", b.unit_id, b.pin))
            .collect();
        println!(
            "{} {role} {} {} [{}]",
            t.id,
            t.write_candidate,
            reason,
            binds.join(",")
        );
    }
}

fn print_coverage(model: &machine_model::resolve::ResolvedModel) {
    println!("未綁定的選擇性 ctrl 腳（HMI 覆蓋率缺口）：");
    for pin in model.coverage_gaps() {
        println!("{}/{}（{}）", pin.unit_path, pin.pin, pin.label);
    }
}
