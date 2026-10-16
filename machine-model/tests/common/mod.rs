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
pub const TREE: &str = "  - {id: machine, type: root, type_version: 1, bind: {m: mid1} }
  - {id: mid1, type: mid, type_version: 1, bind: {a: leaf1, b: leaf2} }
  - {id: leaf1, type: leaf, type_version: 1}
  - {id: leaf2, type: leaf, type_version: 1}
";

/// 只跑圖規則（不跑載入安全規則與接腳規則），回傳規則編號。
pub fn graph_codes(units: &str) -> Vec<&'static str> {
    let text = doc(&format!("units:\n{units}"));
    let model = match parse(&text) {
        Ok(m) => m,
        Err(e) => {
            panic!("測試資料必須能解析: {e}");
        }
    };
    analyze::check_graph(&model, &catalog())
        .iter()
        .map(|d| d.code)
        .collect()
}

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
        "  - {{id: machine, type: root, type_version: 1, bind: {{m: mid1}}}}\n  - {{id: mid1, type: mid, type_version: 1, bind: {{a: leaf1, b: leaf2}}}}\n  - {{id: leaf1, type: leaf, type_version: 1, bind: {{{leaf1_bind}}}}}\n  - {{id: leaf2, type: leaf, type_version: 1, bind: {{i1: plc1.L2_in, o1: plc1.L2_out, error_e: plc1.L2_err}}}}\n"
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
