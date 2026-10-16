use crate::model::TagConfig;

/// 解析 CSV、TSV 或多欄 PLC 符號表文字為 Tag 清單。
pub fn parse_delimited(text: &str, default_plc: &str) -> Result<Vec<TagConfig>, String> {
    let mut tags = Vec::new();
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err("輸入內容為空".to_string());
    }

    for (line_idx, line) in lines.iter().enumerate() {
        let cols = split_line_columns(line);
        if cols.is_empty() {
            continue;
        }

        // 檢查是否為表頭（第一行）
        if line_idx == 0 && is_header_row(&cols) {
            continue;
        }

        let tag = parse_row(&cols, default_plc, line_idx + 1)?;
        tags.push(tag);
    }

    if tags.is_empty() {
        return Err("未解析到任何有效 Tag 點位".to_string());
    }

    Ok(tags)
}

fn split_line_columns(line: &str) -> Vec<String> {
    let raw_cols: Vec<String> = if line.contains('\t') {
        line.split('\t')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect()
    } else if line.contains(',') {
        line.split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .collect()
    } else if line.contains("  ") {
        // 連續 2 個或以上空格分隔（例如自終端或對齊文字複製）
        let mut cols = Vec::new();
        let mut cur = String::new();
        let mut space_count = 0;
        for ch in line.chars() {
            if ch == ' ' || ch == '\u{3000}' {
                space_count += 1;
            } else {
                if space_count >= 2 {
                    if !cur.is_empty() {
                        cols.push(cur.trim().trim_matches('"').trim_matches('\'').to_string());
                        cur.clear();
                    }
                } else if space_count == 1 {
                    cur.push(' ');
                }
                space_count = 0;
                cur.push(ch);
            }
        }
        if !cur.is_empty() {
            cols.push(cur.trim().trim_matches('"').trim_matches('\'').to_string());
        }
        cols
    } else {
        line.split_whitespace()
            .map(|s| s.trim_matches('"').trim_matches('\'').to_string())
            .collect()
    };

    let mut cols = raw_cols;
    while cols.last().map(|c| c.is_empty()).unwrap_or(false) {
        cols.pop();
    }
    cols
}

fn is_header_row(cols: &[String]) -> bool {
    let header_keywords = [
        "id",
        "tag_id",
        "address",
        "addr",
        "位址",
        "点位",
        "點位",
        "名稱",
        "名称",
        "序号",
        "序號",
        "no",
        "no.",
        "index",
        "变量名",
        "變數名",
        "變量名",
        "数据类型",
        "資料型別",
        "数据型态",
        "資料型態",
        "datatype",
        "data_type",
        "type",
        "掉电保持",
        "掉電保持",
        "保持",
        "retain",
        "范围",
        "範圍",
        "scope",
        "access",
        "软元件",
        "軟元件",
        "元件",
        "物理地址",
        "物理位址",
        "注释",
        "註釋",
        "說明",
        "说明",
        "comment",
        "remark",
        "description",
    ];
    let matched_count = cols
        .iter()
        .filter(|c| {
            let lower = c.trim().to_ascii_lowercase();
            header_keywords.iter().any(|k| lower == *k)
        })
        .count();

    if matched_count >= 2 {
        return true;
    }
    if let Some(first) = cols.first() {
        let lower = first.trim().to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "id" | "tag_id"
                | "序号"
                | "序號"
                | "no"
                | "no."
                | "index"
                | "位址"
                | "address"
                | "addr"
                | "名稱"
                | "名称"
                | "变量名"
                | "變數名"
        )
    } else {
        false
    }
}

fn parse_row(cols: &[String], default_plc: &str, row_num: usize) -> Result<TagConfig, String> {
    let non_empty_cols: Vec<(usize, &String)> = cols
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.trim().is_empty())
        .collect();

    if non_empty_cols.is_empty() {
        return Err(format!("第 {} 行為空白", row_num));
    }

    // 1. 尋找位址 (address) 候選欄位
    let addr_found = non_empty_cols
        .iter()
        .find(|(_, s)| is_address_like(s))
        .map(|(idx, s)| (*idx, (*s).clone()));

    // 2. 尋找型別 (type) 候選欄位
    let type_found = non_empty_cols
        .iter()
        .find(|(idx, s)| {
            if matches!(&addr_found, Some((addr_idx, _)) if addr_idx == idx) {
                return false;
            }
            parse_plc_type(s).is_some()
        })
        .and_then(|(idx, s)| parse_plc_type(s).map(|ty| (*idx, ty)));

    // 3. 識別列序號 (若首欄為純數字且總欄數 >= 3)
    let has_row_no = non_empty_cols.len() >= 3
        && non_empty_cols[0].0 == 0
        && non_empty_cols[0].1.chars().all(|c| c.is_ascii_digit());
    let row_no_idx = if has_row_no { Some(0) } else { None };

    // 4. 識別元資料欄位（掉電保持、作用域等）
    let meta_indices: Vec<usize> = non_empty_cols
        .iter()
        .filter_map(|(idx, s)| if is_meta_column(s) { Some(*idx) } else { None })
        .collect();

    // 5. 尋找名稱／標籤 (name/id)
    // 排除：位址欄位、型別欄位、序號欄位、元資料欄位
    let name_candidate = non_empty_cols.iter().find(|(idx, s)| {
        if Some(*idx) == row_no_idx {
            return false;
        }
        if matches!(&addr_found, Some((addr_idx, _)) if addr_idx == idx) {
            return false;
        }
        if matches!(&type_found, Some((type_idx, _)) if type_idx == idx) {
            return false;
        }
        if meta_indices.contains(idx) {
            return false;
        }
        is_valid_identifier(s)
    });

    // 6. 尋找註釋／說明 (label)
    // 排除：位址欄位、型別欄位、序號欄位、名稱欄位、元資料欄位
    let label_candidate = non_empty_cols.iter().rfind(|(idx, s)| {
        if Some(*idx) == row_no_idx {
            return false;
        }
        if matches!(&addr_found, Some((addr_idx, _)) if addr_idx == idx) {
            return false;
        }
        if matches!(&type_found, Some((type_idx, _)) if type_idx == idx) {
            return false;
        }
        if matches!(&name_candidate, Some((name_idx, _)) if name_idx == idx) {
            return false;
        }
        if meta_indices.contains(idx) {
            return false;
        }
        !s.trim().is_empty()
    });

    let (raw_id, raw_addr, parsed_type, raw_label) = match (addr_found, name_candidate) {
        (Some((_, addr)), Some((_, name))) => {
            let ty = type_found.as_ref().map(|(_, t)| t.clone());
            let lbl = label_candidate.map(|(_, l)| (*l).clone());
            ((*name).clone(), addr, ty, lbl)
        }
        (Some((_, addr)), None) => {
            // 無專門名稱，檢查剩餘非元資料欄是否可作為說明或名稱
            let ty = type_found.as_ref().map(|(_, t)| t.clone());
            let lbl = label_candidate.map(|(_, l)| (*l).clone());
            (suggest_id_from_addr(&addr), addr, ty, lbl)
        }
        (None, Some((_, name))) => {
            // 未偵測到位址特徵，嘗試從第二欄或最後一欄作為位址 fallback
            let ty = type_found.as_ref().map(|(_, t)| t.clone());
            let fallback_addr = non_empty_cols
                .iter()
                .find(|(idx, _)| {
                    Some(*idx) != row_no_idx
                        && !meta_indices.contains(idx)
                        && Some(*idx) != type_found.as_ref().map(|(i, _)| *i)
                })
                .map(|(_, s)| (*s).clone())
                .unwrap_or_default();

            if fallback_addr.is_empty() {
                return Err(format!("第 {} 行未提供有效 PLC 位址", row_num));
            }
            (
                (*name).clone(),
                fallback_addr,
                ty,
                label_candidate.map(|(_, l)| (*l).clone()),
            )
        }
        (None, None) => {
            // 兩欄或一欄的傳統 fallback: [id, addr] 或 [addr]
            if non_empty_cols.len() == 1 {
                let s = non_empty_cols[0].1;
                (suggest_id_from_addr(s), s.clone(), None, None)
            } else if non_empty_cols.len() >= 2 {
                let id_s = non_empty_cols[0].1;
                let addr_s = non_empty_cols[1].1;
                let ty = type_found.as_ref().map(|(_, t)| t.clone());
                let lbl = label_candidate.map(|(_, l)| (*l).clone());
                (id_s.clone(), addr_s.clone(), ty, lbl)
            } else {
                return Err(format!("第 {} 行格式無法識別", row_num));
            }
        }
    };

    let address = raw_addr.trim().to_uppercase();
    if address.is_empty() {
        return Err(format!("第 {} 行未提供 PLC 位址", row_num));
    }

    let ty = parsed_type.unwrap_or_else(|| infer_type_from_address(&address));

    let sanitized_id_name = sanitize_name(&raw_id);
    let id = if sanitized_id_name.contains('.') {
        sanitized_id_name
    } else {
        format!("{}.{}", default_plc, sanitized_id_name)
    };

    let label = raw_label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(TagConfig {
        id,
        address,
        ty,
        label,
        description: None,
        polarity: None,
        safety_monitor: None,
        scan: None,
        unit: None,
        min: None,
        max: None,
        default: None,
        spare: None,
    })
}

fn is_address_like(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.contains(' ') || s.contains('_') {
        return false;
    }
    let upper = s.to_ascii_uppercase();

    // IEC 61131-3 格式: %IX0.0, %QX1.0, %MW100, %MD0 等
    if let Some(rest) = upper.strip_prefix('%') {
        if rest.starts_with(['I', 'Q', 'M']) {
            let after_prefix =
                if rest.len() >= 2 && matches!(&rest[1..2], "X" | "B" | "W" | "D" | "L") {
                    &rest[2..]
                } else {
                    &rest[1..]
                };
            return !after_prefix.is_empty()
                && after_prefix.chars().all(|c| c.is_ascii_digit() || c == '.');
        }
        return false;
    }

    // Siemens DB 格式: DB1.DBX0.0, DB1.DBD4 等
    if upper.starts_with("DB") && upper.contains('.') {
        return true;
    }

    // 常見 PLC 軟元件 / 暫存器（匯川、三菱、歐姆龍、台達等）：
    // SM, SD, X, Y, M, D, R, S, B, W, T, C, I, Q
    let prefixes = [
        "SM", "SD", "X", "Y", "M", "D", "R", "S", "B", "W", "T", "C", "I", "Q",
    ];
    for p in prefixes {
        if let Some(rest) = upper.strip_prefix(p) {
            if rest.is_empty() {
                continue;
            }
            let (main_part, bit_part) = match rest.split_once('.') {
                Some((m, b)) => (m, Some(b)),
                None => (rest, None),
            };

            let is_all_hex = !main_part.is_empty()
                && main_part.len() <= 8
                && main_part.chars().all(|c| c.is_ascii_hexdigit());

            let bit_valid = match bit_part {
                Some(b) => {
                    !b.is_empty() && b.len() <= 4 && b.chars().all(|c| c.is_ascii_hexdigit())
                }
                None => true,
            };

            if is_all_hex && bit_valid {
                if (p == "I" || p == "Q") && !main_part.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                return true;
            }
        }
    }

    false
}

fn parse_plc_type(s: &str) -> Option<String> {
    let lower = s.trim().to_ascii_lowercase();
    match lower.as_str() {
        "bool" | "boolean" | "bit" => Some("bool".to_string()),
        "int" | "int16" | "short" | "integer" | "signed" => Some("int16".to_string()),
        "uint" | "uint16" | "word" | "ushort" | "unsigned" => Some("uint16".to_string()),
        "dint" | "int32" | "long" | "dinteger" => Some("int32".to_string()),
        "udint" | "uint32" | "dword" | "ulong" => Some("uint32".to_string()),
        "lint" | "int64" => Some("int64".to_string()),
        "ulint" | "uint64" | "lword" => Some("uint64".to_string()),
        "real" | "float" | "float32" | "single" => Some("float32".to_string()),
        "lreal" | "double" | "float64" => Some("float64".to_string()),
        "byte" => Some("uint16".to_string()),
        _ if lower.starts_with("string") => Some("string".to_string()),
        _ => None,
    }
}

fn is_meta_column(s: &str) -> bool {
    let lower = s.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "不保持"
            | "保持"
            | "掉电保持"
            | "掉電保持"
            | "retain"
            | "non-retain"
            | "私有"
            | "全局"
            | "局部"
            | "public"
            | "private"
            | "local"
            | "global"
            | "var"
            | "var_global"
            | "var_input"
            | "var_output"
            | "var_in_out"
            | "base"
            | "internal"
            | "external"
            | "true"
            | "false"
            | "-"
            | "none"
            | "null"
    )
}

fn is_valid_identifier(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn infer_type_from_address(addr: &str) -> String {
    let addr = addr.trim().to_uppercase();
    if addr.starts_with('D') || addr.starts_with('R') {
        if addr.contains('.') {
            "bool".to_string()
        } else {
            "int16".to_string()
        }
    } else {
        "bool".to_string()
    }
}

fn suggest_id_from_addr(addr: &str) -> String {
    let cleaned = addr.replace('.', "_");
    format!("IO_{}", cleaned)
}

fn sanitize_name(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return "Tag".to_string();
    }
    if let Some((plc, name)) = s.split_once('.') {
        let plc_clean = plc.trim();
        let name_clean: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{}.{}", plc_clean, name_clean)
    } else {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

/// 解析 YAML 點位區塊。
pub fn parse_yaml_tags(text: &str) -> Result<Vec<TagConfig>, String> {
    // 支援直接傳入 tags 清單或帶有 tags: 鍵的片段
    let clean = text.trim();
    let to_parse = if clean.starts_with('-') {
        clean.to_string()
    } else if clean.starts_with("tags:") {
        clean
            .strip_prefix("tags:")
            .unwrap_or(clean)
            .trim()
            .to_string()
    } else {
        format!("- {}", clean)
    };

    let tags: Vec<TagConfig> =
        serde_yaml_ng::from_str(&to_parse).map_err(|e| format!("YAML 點表解析錯誤：{}", e))?;

    if tags.is_empty() {
        return Err("未解析到任何 Tag".to_string());
    }

    Ok(tags)
}

/// 產生 匯川 (Inovance Easy) 範例 I/O 集合。
pub fn generate_inovance_easy_preset(plc_id: &str) -> Vec<TagConfig> {
    let mut tags = Vec::new();

    // X 輸入（八進位 0~17）
    for i in [0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17] {
        tags.push(TagConfig {
            id: format!("{}.X_{:02}", plc_id, i),
            address: format!("X{:02}", i),
            ty: "bool".to_string(),
            label: Some(format!("數位輸入 X{:02}", i)),
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
    }

    // Y 輸出（八進位 0~17）
    for i in [0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17] {
        tags.push(TagConfig {
            id: format!("{}.Y_{:02}", plc_id, i),
            address: format!("Y{:02}", i),
            ty: "bool".to_string(),
            label: Some(format!("數位輸出 Y{:02}", i)),
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
    }

    // M 內部輔助（十進位 100~115）
    for i in 100..=115 {
        tags.push(TagConfig {
            id: format!("{}.M_{}", plc_id, i),
            address: format!("M{}", i),
            ty: "bool".to_string(),
            label: Some(format!("內部線圈 M{}", i)),
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
    }

    // D 暫存器（十進位 100~109）
    for i in 100..=109 {
        tags.push(TagConfig {
            id: format!("{}.D_{}", plc_id, i),
            address: format!("D{}", i),
            ty: "int16".to_string(),
            label: Some(format!("資料暫存器 D{}", i)),
            description: None,
            polarity: None,
            safety_monitor: None,
            scan: None,
            unit: None,
            min: Some(0.0),
            max: Some(30000.0),
            default: None,
            spare: None,
        });
    }

    tags
}

/// 產生 三菱 (Mitsubishi FX) 範例 I/O 集合。
pub fn generate_mitsubishi_preset(plc_id: &str) -> Vec<TagConfig> {
    let mut tags = Vec::new();

    // X 輸入（八進位 0~17）
    for i in [0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17] {
        tags.push(TagConfig {
            id: format!("{}.X_{:02}", plc_id, i),
            address: format!("X{:02}", i),
            ty: "bool".to_string(),
            label: Some(format!("FX 輸入 X{:02}", i)),
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
    }

    // Y 輸出（八進位 0~17）
    for i in [0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17] {
        tags.push(TagConfig {
            id: format!("{}.Y_{:02}", plc_id, i),
            address: format!("Y{:02}", i),
            ty: "bool".to_string(),
            label: Some(format!("FX 輸出 Y{:02}", i)),
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
    }

    // M 內部輔助（十進位 0~15）
    for i in 0..=15 {
        tags.push(TagConfig {
            id: format!("{}.M_{}", plc_id, i),
            address: format!("M{}", i),
            ty: "bool".to_string(),
            label: Some(format!("FX 線圈 M{}", i)),
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
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inovance_autoshop_tab_format() {
        let text = "9\tI_RightGripper_OpenLimit\tBOOL\t不保持\t私有\tX010\n10\tI_GripperCylinder_ForwardLimit\tBOOL\t不保持\t私有\tX011\t前進檢知";
        let tags = parse_delimited(text, "plc1").expect("解析應成功");
        assert_eq!(tags.len(), 2);

        assert_eq!(tags[0].id, "plc1.I_RightGripper_OpenLimit");
        assert_eq!(tags[0].address, "X010");
        assert_eq!(tags[0].ty, "bool");
        assert_eq!(tags[0].label, None);

        assert_eq!(tags[1].id, "plc1.I_GripperCylinder_ForwardLimit");
        assert_eq!(tags[1].address, "X011");
        assert_eq!(tags[1].ty, "bool");
        assert_eq!(tags[1].label.as_deref(), Some("前進檢知"));
    }

    #[test]
    fn test_inovance_autoshop_space_aligned() {
        let text = "9  I_RightGripper_OpenLimit  BOOL  不保持  私有  X010";
        let tags = parse_delimited(text, "plc1").expect("解析應成功");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, "plc1.I_RightGripper_OpenLimit");
        assert_eq!(tags[0].address, "X010");
        assert_eq!(tags[0].ty, "bool");
    }

    #[test]
    fn test_header_detection_and_skip() {
        let text = "序号\t变量名\t数据类型\t掉电保持\t范围\t软元件\t注释\n9\tI_RightGripper_OpenLimit\tBOOL\t不保持\t私有\tX010\t右夾爪開極限";
        let tags = parse_delimited(text, "plc1").expect("解析應成功");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, "plc1.I_RightGripper_OpenLimit");
        assert_eq!(tags[0].address, "X010");
        assert_eq!(tags[0].ty, "bool");
        assert_eq!(tags[0].label.as_deref(), Some("右夾爪開極限"));
    }

    #[test]
    fn test_word_register_and_bit() {
        let text = "D100, MotorSpeed, int16, 速度\nD100.2, BitAlarm, bool, 警報位元";
        let tags = parse_delimited(text, "plc1").expect("解析應成功");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].address, "D100");
        assert_eq!(tags[0].ty, "int16");
        assert_eq!(tags[0].id, "plc1.MotorSpeed");

        assert_eq!(tags[1].address, "D100.2");
        assert_eq!(tags[1].ty, "bool");
        assert_eq!(tags[1].id, "plc1.BitAlarm");
    }

    #[test]
    fn test_address_only_and_address_label() {
        let text = "X10\nX11, 測試開關";
        let tags = parse_delimited(text, "plc1").expect("解析應成功");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "plc1.IO_X10");
        assert_eq!(tags[0].address, "X10");
        assert_eq!(tags[0].ty, "bool");

        assert_eq!(tags[1].id, "plc1.IO_X11");
        assert_eq!(tags[1].address, "X11");
        assert_eq!(tags[1].label.as_deref(), Some("測試開關"));
    }
}
