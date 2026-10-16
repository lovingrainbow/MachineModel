//! 命名規則（前綴表，規格 §5）。這張程式內的表是前綴的唯一事實來源。

use crate::schema::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Prefix {
    Cmd,
    Man,
    Ena,
    Par,
    At,
    Sts,
    Error,
    Count,
    Pv,
    Msg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// 綁定要求（規格 §5「綁定」欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindPolicy {
    /// 純接線，不需要 Tag（`cmd`、`ena`）。
    Wiring,
    /// 選擇性綁定；未綁列入 HMI 覆蓋率（U16）。
    Optional,
    /// 必綁（`error`，U6）。
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixInfo {
    pub prefix: Prefix,
    pub name: &'static str,
    pub direction: Direction,
    /// 推測的 role（規格 §6.1）。
    pub role: Role,
    /// 操作候選（規格 §6.2）。
    pub write_candidate: bool,
    pub bind: BindPolicy,
    /// 未寫 `type` 時的預設型別；`None` 表示必須明寫。
    pub default_type: Option<&'static str>,
}

const fn row(
    prefix: Prefix,
    name: &'static str,
    direction: Direction,
    role: Role,
    write_candidate: bool,
    bind: BindPolicy,
    default_type: Option<&'static str>,
) -> PrefixInfo {
    PrefixInfo {
        prefix,
        name,
        direction,
        role,
        write_candidate,
        bind,
        default_type,
    }
}

/// 順序必須與 [`Prefix`] 的宣告順序一致（`Prefix::info` 以列舉值當索引，由測試保證）。
static TABLE: [PrefixInfo; 10] = [
    row(
        Prefix::Cmd,
        "cmd",
        Direction::In,
        Role::Command,
        false,
        BindPolicy::Wiring,
        Some("bool"),
    ),
    row(
        Prefix::Man,
        "man",
        Direction::In,
        Role::Command,
        true,
        BindPolicy::Optional,
        Some("bool"),
    ),
    row(
        Prefix::Ena,
        "ena",
        Direction::In,
        Role::Command,
        false,
        BindPolicy::Wiring,
        Some("bool"),
    ),
    row(
        Prefix::Par,
        "par",
        Direction::In,
        Role::Setpoint,
        true,
        BindPolicy::Optional,
        None,
    ),
    row(
        Prefix::At,
        "at",
        Direction::Out,
        Role::Status,
        false,
        BindPolicy::Optional,
        Some("bool"),
    ),
    row(
        Prefix::Sts,
        "sts",
        Direction::Out,
        Role::Status,
        false,
        BindPolicy::Optional,
        Some("bool"),
    ),
    row(
        Prefix::Error,
        "error",
        Direction::Out,
        Role::Alarm,
        false,
        BindPolicy::Required,
        Some("bool"),
    ),
    row(
        Prefix::Count,
        "count",
        Direction::Out,
        Role::Counter,
        false,
        BindPolicy::Optional,
        Some("int32"),
    ),
    row(
        Prefix::Pv,
        "pv",
        Direction::Out,
        Role::ProcessValue,
        false,
        BindPolicy::Optional,
        None,
    ),
    row(
        Prefix::Msg,
        "msg",
        Direction::Out,
        Role::Message,
        false,
        BindPolicy::Optional,
        None,
    ),
];

impl Prefix {
    pub const ALL: [Prefix; 10] = [
        Prefix::Cmd,
        Prefix::Man,
        Prefix::Ena,
        Prefix::Par,
        Prefix::At,
        Prefix::Sts,
        Prefix::Error,
        Prefix::Count,
        Prefix::Pv,
        Prefix::Msg,
    ];

    pub fn info(self) -> &'static PrefixInfo {
        &TABLE[self as usize]
    }
}

/// `par_ext_time` → `(Par, "ext_time")`。前綴後必須有 `_` 與至少一個字元。
pub fn split(name: &str) -> Option<(Prefix, &str)> {
    Prefix::ALL.into_iter().find_map(|p| {
        let rest = name.strip_prefix(p.info().name)?.strip_prefix('_')?;
        (!rest.is_empty()).then_some((p, rest))
    })
}

/// 名稱是否以任一 `<前綴>_` 開頭（`field` 腳名稱不得如此，規格 §4.1）。
pub fn has_prefix_underscore(name: &str) -> bool {
    Prefix::ALL.iter().any(|p| {
        name.strip_prefix(p.info().name)
            .is_some_and(|r| r.starts_with('_'))
    })
}

/// `ext_time` → `ext`：`<前綴>_` 之後的第一段是「功能」（規格 §4.1）。
pub fn function_of(rest: &str) -> &str {
    rest.split('_').next().unwrap_or(rest)
}

/// 已知功能名詞典（規格 §6.4）。沒有收錄的功能名沒有預設 label。
pub fn default_label(function: &str) -> Option<&'static str> {
    Some(match function {
        "ext" => "伸出",
        "ret" => "縮回",
        "start" => "啟動",
        "stop" => "停止",
        "reset" => "復歸",
        "home" => "原點",
        "auto" => "自動",
        "manual" => "手動",
        "mode" => "模式",
        "up" => "上升",
        "down" => "下降",
        "open" => "開啟",
        "close" => "關閉",
        "run" | "running" => "運轉",
        "estop" => "緊急停止",
        _ => return None,
    })
}

/// 以前綴名稱（`cmd`、`man`…）序列化，與 `machine.yaml` 與類型檔的寫法一致。
impl serde::Serialize for Prefix {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.info().name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Role;

    #[test]
    fn table_is_complete_and_indexed_by_enum_order() {
        assert_eq!(Prefix::ALL.len(), 10);
        for (i, p) in Prefix::ALL.iter().enumerate() {
            assert_eq!(*p as usize, i);
            assert_eq!(p.info().prefix, *p);
        }
    }

    #[test]
    fn spec_table_values() {
        let info = |p: Prefix| p.info();
        assert_eq!(info(Prefix::Man).role, Role::Command);
        assert_eq!(info(Prefix::Par).role, Role::Setpoint);
        assert_eq!(info(Prefix::Error).role, Role::Alarm);
        assert_eq!(info(Prefix::Count).role, Role::Counter);
        assert_eq!(info(Prefix::Pv).role, Role::ProcessValue);
        assert_eq!(info(Prefix::Msg).role, Role::Message);
        assert_eq!(info(Prefix::At).direction, Direction::Out);
        assert_eq!(info(Prefix::Cmd).direction, Direction::In);
        // 只有 man 與 par 是寫入候選
        let candidates: Vec<_> = Prefix::ALL
            .iter()
            .filter(|p| p.info().write_candidate)
            .map(|p| p.info().name)
            .collect();
        assert_eq!(candidates, ["man", "par"]);
        // 只有 error 必綁；cmd、ena 是純接線
        assert_eq!(info(Prefix::Error).bind, BindPolicy::Required);
        assert_eq!(info(Prefix::Cmd).bind, BindPolicy::Wiring);
        assert_eq!(info(Prefix::Ena).bind, BindPolicy::Wiring);
        assert_eq!(info(Prefix::Man).bind, BindPolicy::Optional);
        // 預設型別
        assert_eq!(info(Prefix::Count).default_type, Some("int32"));
        assert_eq!(info(Prefix::Man).default_type, Some("bool"));
        assert_eq!(info(Prefix::Error).default_type, Some("bool"));
        for p in [Prefix::Par, Prefix::Pv, Prefix::Msg] {
            assert_eq!(p.info().default_type, None, "{p:?}");
        }
    }

    #[test]
    fn split_recognises_prefix_underscore_rest() {
        assert_eq!(split("par_ext_time"), Some((Prefix::Par, "ext_time")));
        assert_eq!(split("man_mode"), Some((Prefix::Man, "mode")));
        assert_eq!(split("error_ext"), Some((Prefix::Error, "ext")));
        for bad in [
            "par", "par_", "parx_a", "lim_ext", "Par_a", "", "_par_a", "at",
        ] {
            assert_eq!(split(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn field_names_must_not_look_like_control_pins() {
        for name in ["cmd_x", "man_", "error_a", "pv_1"] {
            assert!(has_prefix_underscore(name), "{name}");
        }
        for name in ["lim_ext", "valve_ext", "pvx", "cmd", "atx_a"] {
            assert!(!has_prefix_underscore(name), "{name}");
        }
    }

    #[test]
    fn function_and_label() {
        assert_eq!(function_of("ext_time"), "ext");
        assert_eq!(function_of("ext"), "ext");
        assert_eq!(default_label("ext"), Some("伸出"));
        assert_eq!(default_label("ret"), Some("縮回"));
        assert_eq!(default_label("zzz"), None);
    }
}
