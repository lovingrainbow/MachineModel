use std::collections::BTreeMap;
use std::path::PathBuf;

use machine_model::catalog::UnitCatalog;
use machine_model::diag::Severity;
use machine_model::hash::{content_hash, hex, type_hash};
use machine_model::standard::{standard_sources, standard_st_sources};
use machine_model::unit_type::parse_unit_type;

const PINS: &str = include_str!("../../standards/unit-types.pins");

/// 目前內嵌的每個標準類型：id → (version, 雜湊)。雜湊涵蓋類型檔與 `.st` 旁檔（規格 §9.1）。
fn actual() -> BTreeMap<String, (u32, String)> {
    standard_sources()
        .iter()
        .map(|(label, text)| {
            let t = parse_unit_type(label, text).expect("標準類型必須能解析");
            let st = standard_st_sources()
                .iter()
                .find(|(id, _)| *id == t.id)
                .map(|(_, st)| *st);
            (t.id, (t.version, hex(type_hash(text, st))))
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
            panic!(
                "標準類型 `{id}` 沒有釘版本；請執行 print_pins 並更新 standards/unit-types.pins"
            );
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
        .map(|e| {
            e.expect("目錄項")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| {
            std::path::Path::new(n)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml"))
        })
        .map(|n| format!("unit-types/{n}"))
        .collect();
    on_disk.sort();
    let mut embedded: Vec<String> = standard_sources()
        .iter()
        .map(|(l, _)| (*l).to_owned())
        .collect();
    embedded.sort();
    assert_eq!(
        on_disk, embedded,
        "新增標準類型檔後要加進 standard.rs 的 STANDARD"
    );
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
    assert_eq!(
        c.get("machine_ctrl").unwrap().parts.get("stn").unwrap(),
        "station_load"
    );
}

#[test]
fn every_st_file_in_the_standards_directory_is_embedded() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../standards/unit-types");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("找不到 standards/unit-types")
        .filter_map(|e| {
            let path = e.expect("目錄項").path();
            let is_st = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("st"));
            is_st.then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    on_disk.sort();
    let mut embedded: Vec<String> = standard_st_sources()
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect();
    embedded.sort();
    assert_eq!(
        on_disk, embedded,
        "新增標準旁檔後要加進 standard.rs 的 STANDARD_ST，並更新 pins"
    );
}

#[test]
fn embedded_st_files_match_the_files_on_disk() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../standards/unit-types");
    for (id, text) in standard_st_sources() {
        let on_disk = std::fs::read_to_string(dir.join(format!("{id}.st")))
            .unwrap_or_else(|e| panic!("讀不到 {id}.st：{e}"));
        assert_eq!(content_hash(text), content_hash(&on_disk), "{id}.st");
    }
}

#[test]
fn every_embedded_sidecar_belongs_to_a_standard_type() {
    let ids: Vec<String> = standard_sources()
        .iter()
        .map(|(label, text)| parse_unit_type(label, text).unwrap().id)
        .collect();
    for (id, _) in standard_st_sources() {
        assert!(
            ids.iter().any(|i| i == id),
            "旁檔 `{id}` 沒有對應的標準類型"
        );
    }
}
