//! `schema::MachineModel` 序列化後可再被讀回，且內容相同。
//! 下游編輯器（DataService 的 datasvc-demo）靠這個把修改後的模型寫回 `machine.yaml`。

use machine_model::schema::MachineModel;

fn roundtrip(text: &str) {
    let first = machine_model::parse(text).expect("fixture 必須能解析");
    let yaml = serde_yaml_ng::to_string(&first).expect("必須能序列化");
    let second = machine_model::parse(&yaml)
        .unwrap_or_else(|d| panic!("序列化結果必須能再解析：{d}\n{yaml}"));
    assert_eq!(first, second, "往返後內容必須相同\n{yaml}");
}

#[test]
fn valid_units_roundtrips() {
    roundtrip(include_str!("fixtures/valid_units.yaml"));
}

#[test]
fn structural_full_roundtrips() {
    roundtrip(include_str!("fixtures/structural_full.yaml"));
}

#[test]
fn serialized_form_omits_unset_options_but_keeps_null_binds() {
    let text = include_str!("fixtures/structural_full.yaml");
    let model: MachineModel = machine_model::parse(text).unwrap();
    let yaml = serde_yaml_ng::to_string(&model).unwrap();
    // 刻意不接的 bind 必須保留成 null（U6 區分「沒寫」與「刻意不接」）
    assert!(yaml.contains("lim_ret: null"), "{yaml}");
    // 沒設定的選填欄位不得被寫成 null
    let nulls = yaml
        .lines()
        .filter(|l| l.trim_end().ends_with(": null"))
        .count();
    assert_eq!(nulls, 1, "只有 lim_ret 可以是 null：\n{yaml}");
}
