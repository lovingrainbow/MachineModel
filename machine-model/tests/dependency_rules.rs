//! 函式庫本體的依賴限制（規格 §8.5）：未啟用 feature 時只有 `serde` 與 `serde_yaml_ng`；
//! `schema` feature 僅允許 `jsonschema`（`default-features = false`）與 `serde_json`。

use std::process::Command;

const BANNED_BASE: &[&str] = &[
    "tokio",
    "axum",
    "reqwest",
    "hyper",
    "clap",
    "clap_builder",
    "clap_derive",
];

fn tree_names(extra_args: &[&str]) -> String {
    let mut args = vec![
        "tree",
        "-p",
        "machine-model",
        "--edges",
        "normal,build",
        "--prefix",
        "none",
        "--color",
        "never",
    ];
    args.extend(extra_args);
    let output = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("無法執行 cargo tree");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree 輸出不是 UTF-8")
}

fn package_names(tree: &str) -> Vec<&str> {
    tree.lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect()
}

#[test]
fn no_features_banned() {
    let tree = tree_names(&["--no-default-features"]);
    let names = package_names(&tree);
    assert!(names.contains(&"serde"), "{tree}");
    for banned in BANNED_BASE
        .iter()
        .copied()
        .chain(["jsonschema", "serde_json"])
    {
        assert!(!names.contains(&banned), "依賴樹含有 {banned}：\n{tree}");
    }
}

#[test]
fn schema_feature_banned() {
    let tree = tree_names(&["--features", "schema"]);
    let names = package_names(&tree);
    assert!(names.contains(&"serde"), "{tree}");
    assert!(
        names.contains(&"jsonschema"),
        "schema feature 未啟用 jsonschema：\n{tree}"
    );
    assert!(
        names.contains(&"serde_json"),
        "schema feature 未啟用 serde_json：\n{tree}"
    );
    for banned in BANNED_BASE {
        assert!(!names.contains(banned), "依賴樹含有 {banned}：\n{tree}");
    }
}
