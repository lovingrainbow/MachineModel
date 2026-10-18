//! ST 旁檔的輕量檢查（規格 §4.3、U21～U24）。
//!
//! 這是掃描器，**不是** ST 解析器：只看 `FUNCTION_BLOCK` 名稱、宣告區、成對關鍵字與
//! 少數已確認的 `LiteST` 限制。型別是否相容、變數是否宣告過，交給 IDE 編譯。
//! 語法依據：H5U／Easy 編程手冊 V4.12.0.0 §8.1。

use crate::diag::Diagnostic;
use crate::types::{DataType, ScalarType};
use crate::unit_type::UnitType;

/// `LiteST` 本體的行數上限（手冊 §8.1.1）。
const MAX_BODY_LINES: usize = 1000;

/// 標量詞彙對應的 `LiteST` 型別；沒有對應（含陣列）回 `None`。
///
/// 對照依手冊 §7.3 的資料型別表：`BOOL`、`INT`、`DINT`、`REAL`、`STRING`。
pub fn lite_type(dt: DataType) -> Option<&'static str> {
    if dt.array {
        return None;
    }
    match dt.scalar {
        ScalarType::Bool => Some("BOOL"),
        ScalarType::Int16 => Some("INT"),
        ScalarType::Int32 => Some("DINT"),
        ScalarType::Float32 => Some("REAL"),
        ScalarType::Text(_) => Some("STRING"),
        ScalarType::Uint16
        | ScalarType::Uint32
        | ScalarType::Int64
        | ScalarType::Uint64
        | ScalarType::Float64 => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ident,
    Number,
    Str,
    Sym,
}

#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    text: String,
    line: usize,
}

impl Token {
    fn is_sym(&self, s: &str) -> bool {
        self.kind == Kind::Sym && self.text == s
    }

    fn is_kw(&self, kw: &str) -> bool {
        self.kind == Kind::Ident && self.text.eq_ignore_ascii_case(kw)
    }
}

/// 掃描失敗（註解或字串沒結束）。
struct ScanError {
    line: usize,
    message: &'static str,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// 把文字切成詞，略過 `(* *)`（可巢狀）與 `//` 註解；字串字面值保留為單一詞。
fn tokenize(text: &str) -> Result<Vec<Token>, ScanError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '(' && chars.get(i + 1) == Some(&'*') {
            let start = line;
            let mut depth = 1;
            i += 2;
            while depth > 0 {
                match (chars.get(i), chars.get(i + 1)) {
                    (None, _) => {
                        return Err(ScanError {
                            line: start,
                            message: "`(*` 註解沒有結束",
                        });
                    }
                    (Some('('), Some('*')) => {
                        depth += 1;
                        i += 2;
                    }
                    (Some('*'), Some(')')) => {
                        depth -= 1;
                        i += 2;
                    }
                    (Some('\n'), _) => {
                        line += 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '\'' {
            let start = line;
            let mut s = String::from("'");
            i += 1;
            loop {
                match chars.get(i) {
                    None | Some('\n') => {
                        return Err(ScanError {
                            line: start,
                            message: "字串沒有結束",
                        });
                    }
                    Some('\'') => {
                        s.push('\'');
                        i += 1;
                        break;
                    }
                    Some(ch) => {
                        s.push(*ch);
                        i += 1;
                    }
                }
            }
            out.push(Token {
                kind: Kind::Str,
                text: s,
                line: start,
            });
        } else if is_ident_start(c) {
            let begin = i;
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            out.push(Token {
                kind: Kind::Ident,
                text: chars[begin..i].iter().collect(),
                line,
            });
        } else if c.is_ascii_digit() {
            let begin = i;
            while i < chars.len() {
                let ch = chars[i];
                let dot_digit = ch == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit);
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '#' || dot_digit {
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(Token {
                kind: Kind::Number,
                text: chars[begin..i].iter().collect(),
                line,
            });
        } else {
            const TWO: [&str; 9] = [":=", "=>", "<>", "<=", ">=", "==", "!=", "&&", "||"];
            let pair: String = chars[i..chars.len().min(i + 2)].iter().collect();
            if TWO.contains(&pair.as_str()) {
                out.push(Token {
                    kind: Kind::Sym,
                    text: pair,
                    line,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: Kind::Sym,
                    text: c.to_string(),
                    line,
                });
                i += 1;
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    Input,
    Output,
    InOut,
    Local,
}

impl DeclKind {
    fn keyword(self) -> &'static str {
        match self {
            Self::Input => "VAR_INPUT",
            Self::Output => "VAR_OUTPUT",
            Self::InOut => "VAR_IN_OUT",
            Self::Local => "VAR",
        }
    }
}

#[derive(Debug)]
struct Decl {
    kind: DeclKind,
    name: String,
    ty: String,
    line: usize,
}

#[derive(Default)]
struct Parsed {
    fb_names: Vec<(String, usize)>,
    decls: Vec<Decl>,
    /// 宣告格式錯誤：`(行, 訊息)`。
    decl_errors: Vec<(usize, String)>,
    body: Vec<Token>,
    /// 本體開始行（最後一個宣告區之後）與結束行（`END_FUNCTION_BLOCK`）。
    body_start_line: usize,
    body_end_line: Option<usize>,
}

fn decl_kind(tok: &Token) -> Option<DeclKind> {
    [
        ("VAR_INPUT", DeclKind::Input),
        ("VAR_OUTPUT", DeclKind::Output),
        ("VAR_IN_OUT", DeclKind::InOut),
        ("VAR", DeclKind::Local),
    ]
    .into_iter()
    .find(|(kw, _)| tok.is_kw(kw))
    .map(|(_, k)| k)
}

/// 解析 `name[, name] : type [:= init];`，直到 `END_VAR`。回傳下一個詞的位置。
fn parse_decl_section(tokens: &[Token], mut i: usize, kind: DeclKind, p: &mut Parsed) -> usize {
    while i < tokens.len() && !tokens[i].is_kw("END_VAR") {
        let start_line = tokens[i].line;
        let mut stmt = Vec::new();
        while i < tokens.len() && !tokens[i].is_sym(";") && !tokens[i].is_kw("END_VAR") {
            stmt.push(&tokens[i]);
            i += 1;
        }
        let ended_by_semicolon = i < tokens.len() && tokens[i].is_sym(";");
        if ended_by_semicolon {
            i += 1;
        }
        if stmt.is_empty() {
            continue;
        }
        let Some(colon) = stmt.iter().position(|t| t.is_sym(":")) else {
            p.decl_errors.push((start_line, "宣告缺少 `:`".to_owned()));
            continue;
        };
        if !ended_by_semicolon {
            p.decl_errors.push((start_line, "宣告缺少 `;`".to_owned()));
        }
        let names: Vec<&Token> = stmt[..colon]
            .iter()
            .copied()
            .filter(|t| t.kind == Kind::Ident)
            .collect();
        let ty_end = stmt
            .iter()
            .position(|t| t.is_sym(":="))
            .unwrap_or(stmt.len());
        let ty: String = stmt[colon + 1..ty_end]
            .iter()
            .map(|t| t.text.as_str())
            .collect();
        if names.is_empty() || ty.is_empty() {
            p.decl_errors
                .push((start_line, "宣告缺少名稱或型別".to_owned()));
            continue;
        }
        for n in names {
            p.decls.push(Decl {
                kind,
                name: n.text.clone(),
                ty: ty.clone(),
                line: n.line,
            });
        }
    }
    i
}

fn parse(tokens: &[Token]) -> Parsed {
    let mut p = Parsed::default();
    let mut in_fb = false;
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if t.is_kw("FUNCTION_BLOCK") {
            let name = tokens
                .get(i + 1)
                .filter(|n| n.kind == Kind::Ident)
                .map_or_else(String::new, |n| n.text.clone());
            p.fb_names.push((name, t.line));
            in_fb = true;
            p.body_start_line = t.line;
            i += 2;
        } else if t.is_kw("END_FUNCTION_BLOCK") {
            p.body_end_line = Some(t.line);
            in_fb = false;
            i += 1;
        } else if let Some(kind) = decl_kind(t) {
            i += 1;
            while tokens
                .get(i)
                .is_some_and(|m| m.is_kw("RETAIN") || m.is_kw("CONSTANT") || m.is_kw("PERSISTENT"))
            {
                i += 1;
            }
            i = parse_decl_section(tokens, i, kind, &mut p);
            if let Some(end) = tokens.get(i) {
                p.body_start_line = end.line;
                i += 1;
            }
        } else {
            if in_fb {
                p.body.push(t.clone());
            }
            i += 1;
        }
    }
    p
}

/// 成對關鍵字：`(開, 關)`。
const PAIRS: [(&str, &str); 10] = [
    ("IF", "END_IF"),
    ("FOR", "END_FOR"),
    ("WHILE", "END_WHILE"),
    ("REPEAT", "END_REPEAT"),
    ("CASE", "END_CASE"),
    ("REGION", "END_REGION"),
    ("VAR", "END_VAR"),
    ("VAR_INPUT", "END_VAR"),
    ("VAR_OUTPUT", "END_VAR"),
    ("VAR_IN_OUT", "END_VAR"),
];

/// U24：成對關鍵字。只回報第一個問題，避免連鎖雜訊。
fn check_pairs(tokens: &[Token]) -> Option<(usize, String)> {
    let mut stack: Vec<(&'static str, usize)> = Vec::new();
    for t in tokens.iter().filter(|t| t.kind == Kind::Ident) {
        if t.is_kw("FUNCTION_BLOCK") {
            stack.push(("END_FUNCTION_BLOCK", t.line));
        } else if let Some((_, close)) = PAIRS.iter().find(|(open, _)| t.is_kw(open)) {
            stack.push((close, t.line));
        } else if t.is_kw("END_FUNCTION_BLOCK") || PAIRS.iter().any(|(_, close)| t.is_kw(close)) {
            let name = t.text.to_ascii_uppercase();
            match stack.pop() {
                Some((expected, _)) if expected == name => {}
                Some((expected, open_line)) => {
                    return Some((
                        t.line,
                        format!("預期 `{expected}`（對應第 {open_line} 行）卻遇到 `{name}`"),
                    ));
                }
                None => return Some((t.line, format!("`{name}` 沒有對應的開頭"))),
            }
        }
    }
    stack
        .pop()
        .map(|(expected, line)| (line, format!("缺少 `{expected}`")))
}

/// 手冊 §8.1 的語句表以外、IEC 常見但 `LiteST` 本體不會有的關鍵字。
/// 這是拒絕清單：手冊沒列的關鍵字不保證可用，但這裡只擋已知不屬於本體的。
const NOT_LITEST_KEYWORDS: [&str; 18] = [
    "ELSEIF",
    "GOTO",
    "PROGRAM",
    "END_PROGRAM",
    "FUNCTION",
    "END_FUNCTION",
    "END_TYPE",
    "STRUCT",
    "END_STRUCT",
    "VAR_GLOBAL",
    "VAR_EXTERNAL",
    "VAR_TEMP",
    "VAR_STAT",
    "VAR_ACCESS",
    "VAR_CONFIG",
    "END_ACTION",
    "END_METHOD",
    "END_CONFIGURATION",
];

/// `LiteST` 沒有的 IEC 變數型別（手冊 §7.3 只列 BOOL／INT／DINT／REAL／STRING／BYTE／IP）。
const NOT_LITEST_TYPES: [&str; 17] = [
    "UINT", "USINT", "UDINT", "ULINT", "LINT", "SINT", "LREAL", "WORD", "DWORD", "LWORD", "TIME",
    "DATE", "TOD", "DT", "WSTRING", "WCHAR", "CHAR",
];

/// FB 呼叫的輸出實參（`=>` 之後）不得是運算式：只允許空白、變數、成員與索引。
fn output_arg_is_plain(tokens: &[Token], mut i: usize) -> bool {
    let mut expect_ident = true;
    let mut seen_any = false;
    while let Some(t) = tokens.get(i) {
        if t.is_sym(",") || t.is_sym(")") {
            break;
        }
        seen_any = true;
        if expect_ident {
            if t.kind != Kind::Ident {
                return false;
            }
            expect_ident = false;
        } else if t.is_sym(".") {
            expect_ident = true;
        } else if t.is_sym("[") {
            let mut depth = 1;
            while depth > 0 {
                i += 1;
                match tokens.get(i) {
                    None => return false,
                    Some(x) if x.is_sym("[") => depth += 1,
                    Some(x) if x.is_sym("]") => depth -= 1,
                    Some(_) => {}
                }
            }
        } else {
            return false;
        }
        i += 1;
    }
    !seen_any || !expect_ident
}

/// 檢查 `<id>.st` 旁檔（U21～U24）。`text` 是旁檔內容。
pub fn check_unit_st(ty: &UnitType, text: &str) -> Vec<Diagnostic> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root = format!("types[{}].st", ty.id);
    let at = |line: usize| format!("{root} 第 {line} 行");
    let mut out = Vec::new();

    let tokens = match tokenize(text) {
        Ok(t) => t,
        Err(e) => {
            out.push(Diagnostic::error("U24", at(e.line), e.message));
            return out;
        }
    };
    let p = parse(&tokens);

    // U21
    match p.fb_names.as_slice() {
        [] => out.push(Diagnostic::error(
            "U21",
            &root,
            "找不到 `FUNCTION_BLOCK <類型 id>`",
        )),
        [(name, line)] => {
            if *name != ty.id {
                out.push(Diagnostic::error(
                    "U21",
                    at(*line),
                    format!("FUNCTION_BLOCK 名稱 `{name}` 必須等於類型 id `{}`", ty.id),
                ));
            }
        }
        [_, rest @ ..] => {
            for (_, line) in rest {
                out.push(Diagnostic::error(
                    "U21",
                    at(*line),
                    "一個旁檔只能有一個 FUNCTION_BLOCK",
                ));
            }
        }
    }

    // U24
    if let Some((line, message)) = check_pairs(&tokens) {
        out.push(Diagnostic::error("U24", at(line), message));
    }

    check_interface(ty, &p, &root, &mut out);
    check_litest(ty, &p, &root, &mut out);
    out
}

/// U22：宣告區與類型檔接腳一致；本體引用的 `ctrl.<名>` 必須存在。
fn check_interface(ty: &UnitType, p: &Parsed, root: &str, out: &mut Vec<Diagnostic>) {
    let at = |line: usize| format!("{root} 第 {line} 行");
    for (line, message) in &p.decl_errors {
        out.push(Diagnostic::error("U22", at(*line), message.clone()));
    }

    let find = |name: &str| p.decls.iter().find(|d| d.name == name);
    let expect_pins = [
        (DeclKind::Input, &ty.field.input.0, "field.in"),
        (DeclKind::Output, &ty.field.output.0, "field.out"),
    ];
    for (kind, pins, section) in expect_pins {
        for (name, pin) in pins {
            match find(name) {
                None => out.push(Diagnostic::error(
                    "U22",
                    root,
                    format!(
                        "缺少 `{name}`：`{section}` 的接腳必須在 {} 宣告",
                        kind.keyword()
                    ),
                )),
                Some(d) if d.kind != kind => out.push(Diagnostic::error(
                    "U22",
                    at(d.line),
                    format!(
                        "`{name}` 方向錯誤：`{section}` 的接腳必須在 {} 宣告，卻放在 {}",
                        kind.keyword(),
                        d.kind.keyword()
                    ),
                )),
                Some(d) => {
                    let Some(expected) = DataType::parse(&pin.ty).ok().and_then(lite_type) else {
                        continue;
                    };
                    if !type_matches(&d.ty, expected) {
                        out.push(Diagnostic::error(
                            "U22",
                            at(d.line),
                            format!(
                                "`{name}` 型別 `{}` 與類型檔的 `{}` 不一致（應為 {expected}）",
                                d.ty, pin.ty
                            ),
                        ));
                    }
                }
            }
        }
        for d in p.decls.iter().filter(|d| d.kind == kind) {
            if !pins.contains_key(&d.name) {
                out.push(Diagnostic::error(
                    "U22",
                    at(d.line),
                    format!(
                        "`{}` 不在類型檔的 `{section}`，不可宣告於 {}",
                        d.name,
                        kind.keyword()
                    ),
                ));
            }
        }
    }

    // VAR_IN_OUT：`ctrl`（有 ctrl 腳時）與各 part。
    let mut expected_inout: Vec<(String, String)> = Vec::new();
    if !ty.ctrl.0.is_empty() {
        expected_inout.push(("ctrl".to_owned(), format!("ST_{}", ty.id)));
    }
    for (part, part_ty) in &ty.parts.0 {
        expected_inout.push((part.clone(), format!("ST_{part_ty}")));
    }
    for (name, want) in &expected_inout {
        match find(name) {
            None => out.push(Diagnostic::error(
                "U22",
                root,
                format!("缺少 VAR_IN_OUT `{name} : {want}`"),
            )),
            Some(d) if d.kind != DeclKind::InOut => out.push(Diagnostic::error(
                "U22",
                at(d.line),
                format!("`{name}` 必須宣告於 VAR_IN_OUT"),
            )),
            Some(d) if d.ty != *want => out.push(Diagnostic::error(
                "U22",
                at(d.line),
                format!("`{name}` 型別 `{}` 應為 `{want}`", d.ty),
            )),
            Some(_) => {}
        }
    }
    for d in p.decls.iter().filter(|d| d.kind == DeclKind::InOut) {
        if !expected_inout.iter().any(|(n, _)| *n == d.name) {
            out.push(Diagnostic::error(
                "U22",
                at(d.line),
                format!(
                    "VAR_IN_OUT 只能是 `ctrl` 或 parts 的名稱，`{}` 不是",
                    d.name
                ),
            ));
        }
    }

    // 本體引用的 ctrl 成員。
    for w in p.body.windows(3) {
        if w[0].is_kw("ctrl")
            && w[1].is_sym(".")
            && w[2].kind == Kind::Ident
            && !ty.ctrl.0.contains_key(&w[2].text)
        {
            out.push(Diagnostic::error(
                "U22",
                at(w[2].line),
                format!("`ctrl.{}` 不是類型檔 `ctrl` 的接腳", w[2].text),
            ));
        }
    }
}

fn type_matches(declared: &str, expected: &str) -> bool {
    let d = declared.to_ascii_uppercase();
    if expected == "STRING" {
        d == "STRING" || d.starts_with("STRING[") || d.starts_with("STRING(")
    } else {
        d == expected
    }
}

/// U23：只擋已確認的 `LiteST` 限制。
fn check_litest(ty: &UnitType, p: &Parsed, root: &str, out: &mut Vec<Diagnostic>) {
    let _ = ty;
    let at = |line: usize| format!("{root} 第 {line} 行");

    for d in &p.decls {
        let head: String =
            d.ty.chars()
                .take_while(|c| is_ident_char(*c))
                .collect::<String>()
                .to_ascii_uppercase();
        if NOT_LITEST_TYPES.contains(&head.as_str()) {
            out.push(Diagnostic::error(
                "U23",
                at(d.line),
                format!(
                    "`{}` 的型別 `{}` 不是 LiteST 型別（只有 BOOL、INT、DINT、REAL、STRING、BYTE、IP）",
                    d.name, d.ty
                ),
            ));
        }
    }

    for (idx, t) in p.body.iter().enumerate() {
        if t.kind == Kind::Ident {
            let up = t.text.to_ascii_uppercase();
            if up == "ELSEIF" {
                out.push(Diagnostic::error(
                    "U23",
                    at(t.line),
                    "LiteST 的關鍵字是 `ELSIF`，不是 `ELSEIF`",
                ));
            } else if NOT_LITEST_KEYWORDS.contains(&up.as_str()) {
                out.push(Diagnostic::error(
                    "U23",
                    at(t.line),
                    format!("`{up}` 不屬於 LiteST 程式本體"),
                ));
            }
        } else if t.kind == Kind::Sym {
            let hint = match t.text.as_str() {
                "==" => Some("等於請用 `=`"),
                "!=" => Some("不等於請用 `<>`"),
                "&&" => Some("請用 `AND`"),
                "||" => Some("請用 `OR`"),
                _ => None,
            };
            if let Some(h) = hint {
                out.push(Diagnostic::error(
                    "U23",
                    at(t.line),
                    format!("LiteST 沒有運算子 `{}`：{h}", t.text),
                ));
            }
            if t.text == "=>" && !output_arg_is_plain(&p.body, idx + 1) {
                out.push(Diagnostic::error(
                    "U23",
                    at(t.line),
                    "FB 呼叫的輸出實參不得是運算式（只能是變數、成員或索引）",
                ));
            }
        }
    }

    if let Some(end) = p.body_end_line {
        let rows = end.saturating_sub(p.body_start_line + 1);
        if rows > MAX_BODY_LINES {
            out.push(Diagnostic::error(
                "U23",
                root,
                format!("本體 {rows} 行，超過 LiteST 單一程式 {MAX_BODY_LINES} 行的上限"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_type::parse_unit_type;

    const CYLINDER_YAML: &str = include_str!("../../standards/unit-types/cylinder_double.yaml");

    /// 測試夾具，不代表任何標準類型的內容。
    const GOOD: &str = "\
FUNCTION_BLOCK cylinder_double
VAR_INPUT
    lim_ext : BOOL;
    lim_ret : BOOL;
END_VAR
VAR_OUTPUT
    valve_ext : BOOL;
    valve_ret : BOOL;
END_VAR
VAR_IN_OUT
    ctrl : ST_cylinder_double;
END_VAR
VAR
    step : INT := 0;
END_VAR
(* 夾具本體 *)
IF ctrl.cmd_ext AND ctrl.ena_ext THEN
    valve_ext := TRUE;
ELSIF ctrl.cmd_ret THEN
    valve_ext := FALSE;
END_IF;
ctrl.at_ext := lim_ext;
END_FUNCTION_BLOCK
";

    fn ty() -> UnitType {
        parse_unit_type("cylinder_double.yaml", CYLINDER_YAML).unwrap()
    }

    fn codes(text: &str) -> Vec<&'static str> {
        check_unit_st(&ty(), text).iter().map(|d| d.code).collect()
    }

    fn messages(text: &str) -> String {
        check_unit_st(&ty(), text)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn fixture_passes() {
        assert_eq!(messages(GOOD), "");
    }

    #[test]
    fn bom_and_crlf_are_accepted() {
        let text = format!("\u{feff}{}", GOOD.replace('\n', "\r\n"));
        assert_eq!(messages(&text), "");
    }

    #[test]
    fn lite_type_maps_only_litest_types() {
        let p = |s: &str| lite_type(DataType::parse(s).unwrap());
        assert_eq!(p("bool"), Some("BOOL"));
        assert_eq!(p("int16"), Some("INT"));
        assert_eq!(p("int32"), Some("DINT"));
        assert_eq!(p("float32"), Some("REAL"));
        assert_eq!(p("string:20"), Some("STRING"));
        for t in ["uint16", "uint32", "int64", "uint64", "float64", "int16[4]"] {
            assert_eq!(p(t), None, "{t}");
        }
    }

    // ---- U21 ----

    #[test]
    fn u21_name_must_equal_type_id() {
        let text = GOOD.replace("FUNCTION_BLOCK cylinder_double", "FUNCTION_BLOCK cyl");
        assert_eq!(codes(&text), ["U21"]);
    }

    #[test]
    fn u21_missing_and_duplicate_function_block() {
        assert!(codes("VAR_INPUT END_VAR").contains(&"U21"));
        let twice = format!("{GOOD}{GOOD}");
        assert!(codes(&twice).contains(&"U21"));
    }

    // ---- U22 ----

    #[test]
    fn u22_missing_pin() {
        let text = GOOD.replace("    lim_ext : BOOL;\n", "");
        let m = messages(&text);
        assert!(m.contains("[U22]") && m.contains("缺少 `lim_ext`"), "{m}");
    }

    #[test]
    fn u22_optional_pin_must_still_be_declared() {
        // lim_ret 是 required: false，仍須宣告。
        let text = GOOD.replace("    lim_ret : BOOL;\n", "");
        assert!(messages(&text).contains("缺少 `lim_ret`"));
    }

    #[test]
    fn u22_wrong_direction() {
        let text = GOOD
            .replace(
                "    valve_ret : BOOL;\nEND_VAR\nVAR_IN_OUT",
                "END_VAR\nVAR_IN_OUT",
            )
            .replace(
                "    lim_ret : BOOL;",
                "    lim_ret : BOOL;\n    valve_ret : BOOL;",
            );
        let m = messages(&text);
        assert!(m.contains("`valve_ret` 方向錯誤"), "{m}");
    }

    #[test]
    fn u22_wrong_type() {
        let text = GOOD.replace("lim_ext : BOOL", "lim_ext : INT");
        let m = messages(&text);
        assert!(m.contains("`lim_ext` 型別 `INT`"), "{m}");
    }

    #[test]
    fn u22_type_is_case_insensitive() {
        let text = GOOD.replace("lim_ext : BOOL", "lim_ext : bool");
        assert_eq!(messages(&text), "");
    }

    #[test]
    fn u22_extra_input() {
        let text = GOOD.replace(
            "    lim_ret : BOOL;",
            "    lim_ret : BOOL;\n    extra : BOOL;",
        );
        assert!(messages(&text).contains("`extra` 不在類型檔的 `field.in`"));
    }

    #[test]
    fn u22_ctrl_must_be_declared_with_struct_type() {
        let none = GOOD.replace("VAR_IN_OUT\n    ctrl : ST_cylinder_double;\nEND_VAR\n", "");
        assert!(messages(&none).contains("缺少 VAR_IN_OUT `ctrl : ST_cylinder_double`"));
        let wrong = GOOD.replace("ST_cylinder_double", "ST_other");
        assert!(messages(&wrong).contains("應為 `ST_cylinder_double`"));
    }

    #[test]
    fn u22_unknown_inout_name() {
        let text = GOOD.replace(
            "    ctrl : ST_cylinder_double;",
            "    ctrl : ST_cylinder_double;\n    stray : BOOL;",
        );
        assert!(messages(&text).contains("VAR_IN_OUT 只能是"));
    }

    #[test]
    fn u22_ctrl_member_must_exist() {
        let text = GOOD.replace("ctrl.at_ext", "ctrl.at_nope");
        let m = messages(&text);
        assert!(m.contains("`ctrl.at_nope` 不是類型檔 `ctrl` 的接腳"), "{m}");
    }

    fn messages_for(t: &UnitType, text: &str) -> String {
        check_unit_st(t, text)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn u22_parts_are_declared_as_inout() {
        let yaml = "id: station_x\nversion: 1\ntitle: T\nparts:\n  cyl: cylinder_double\n";
        let t = parse_unit_type("s.yaml", yaml).unwrap();
        let ok = "FUNCTION_BLOCK station_x\nVAR_IN_OUT\n    cyl : ST_cylinder_double;\nEND_VAR\nEND_FUNCTION_BLOCK\n";
        assert_eq!(messages_for(&t, ok), "");
        let missing = "FUNCTION_BLOCK station_x\nEND_FUNCTION_BLOCK\n";
        assert!(check_unit_st(&t, missing).iter().any(|d| {
            d.message
                .contains("缺少 VAR_IN_OUT `cyl : ST_cylinder_double`")
        }));
    }

    #[test]
    fn u22_malformed_declaration() {
        let text = GOOD.replace("    step : INT := 0;", "    step INT;");
        assert!(messages(&text).contains("宣告缺少 `:`"));
    }

    // ---- U24 ----

    #[test]
    fn u24_missing_end_if() {
        let text = GOOD.replace("END_IF;", "");
        let m = messages(&text);
        assert!(m.contains("[U24]") && m.contains("END_IF"), "{m}");
    }

    #[test]
    fn u24_mismatched_close_names_the_line() {
        let text = GOOD.replace("END_IF;", "END_FOR;");
        let m = messages(&text);
        assert!(m.contains("預期 `END_IF`") && m.contains("END_FOR"), "{m}");
    }

    #[test]
    fn u24_stray_close() {
        let text = format!("END_IF;\n{GOOD}");
        assert!(messages(&text).contains("沒有對應的開頭"));
        let inside = GOOD.replace("END_FUNCTION_BLOCK", "END_IF;\nEND_FUNCTION_BLOCK");
        assert!(messages(&inside).contains("預期 `END_FUNCTION_BLOCK`"));
    }

    #[test]
    fn u24_keywords_in_comments_and_strings_are_ignored() {
        let text = GOOD.replace(
            "(* 夾具本體 *)",
            "(* IF ELSEIF (* nested END_IF *) == *)\n// IF without end\ntxt := 'IF END_FOR';",
        );
        let m = messages(&text);
        assert!(!m.contains("U24") && !m.contains("U23"), "{m}");
    }

    #[test]
    fn u24_unterminated_comment_and_string() {
        assert!(messages("FUNCTION_BLOCK x (* oops").contains("註解沒有結束"));
        assert!(messages("FUNCTION_BLOCK x\ns := 'abc;\n").contains("字串沒有結束"));
    }

    #[test]
    fn u24_all_pairs_balance() {
        let body = "\
REGION 區塊
    CASE step OF
        1: step := 2;
    ELSE
        step := 0;
    END_CASE;
    WHILE step < 3 DO step := step + 1; END_WHILE;
    REPEAT step := step + 1; UNTIL step > 5 END_REPEAT;
    FOR step := 0 TO 3 BY 1 DO CONTINUE; END_FOR;
END_REGION
";
        let text = GOOD.replace("(* 夾具本體 *)", body);
        assert_eq!(messages(&text), "");
    }

    // ---- U23 ----

    #[test]
    fn u23_elseif_is_rejected_with_hint() {
        let text = GOOD.replace("ELSIF", "ELSEIF");
        assert!(messages(&text).contains("`ELSIF`"));
    }

    #[test]
    fn u23_non_iec_operators() {
        for (bad, hint) in [("==", "`=`"), ("!=", "`<>`"), ("&&", "AND"), ("||", "OR")] {
            let text = GOOD.replace(
                "cmd_ext AND ctrl.ena_ext",
                &format!("cmd_ext {bad} ctrl.ena_ext"),
            );
            let m = messages(&text);
            assert!(m.contains("[U23]") && m.contains(hint), "{bad}: {m}");
        }
    }

    #[test]
    fn u23_output_argument_must_not_be_an_expression() {
        let call = |arg: &str| {
            GOOD.replace(
                "ctrl.at_ext := lim_ext;",
                &format!("TONR(IN := lim_ext, Q => {arg}, ET => );"),
            )
        };
        assert_eq!(messages(&call("ctrl.at_ext")), "");
        assert_eq!(messages(&call("arr[i + 1]")), "");
        assert!(messages(&call("a + b")).contains("輸出實參不得是運算式"));
        assert!(messages(&call("5")).contains("輸出實參不得是運算式"));
    }

    #[test]
    fn u23_non_litest_types() {
        for ty in ["UINT", "LREAL", "TIME", "WORD"] {
            let text = GOOD.replace("step : INT", &format!("step : {ty}"));
            let m = messages(&text);
            assert!(
                m.contains("[U23]") && m.contains("不是 LiteST 型別"),
                "{ty}: {m}"
            );
        }
        let ok = GOOD.replace("step : INT", "step : STRING[20]");
        assert_eq!(messages(&ok), "");
    }

    #[test]
    fn u23_keywords_that_are_not_body_statements() {
        let text = GOOD.replace("(* 夾具本體 *)", "GOTO done;");
        assert!(messages(&text).contains("`GOTO` 不屬於 LiteST 程式本體"));
    }

    #[test]
    fn u23_body_line_limit() {
        let filler = "x := 0;\n".repeat(MAX_BODY_LINES + 1);
        let text = GOOD.replace("(* 夾具本體 *)", &filler);
        assert!(messages(&text).contains("超過 LiteST 單一程式 1000 行"));
        let filler_ok = "x := 0;\n".repeat(100);
        assert!(!messages(&GOOD.replace("(* 夾具本體 *)", &filler_ok)).contains("1000"));
    }

    #[test]
    fn diagnostics_carry_file_and_line() {
        let text = GOOD.replace("ELSIF", "ELSEIF");
        let d = &check_unit_st(&ty(), &text)[0];
        assert_eq!(d.code, "U23");
        assert!(
            d.location
                .as_deref()
                .unwrap()
                .starts_with("types[cylinder_double].st 第 "),
            "{d}"
        );
    }
}
