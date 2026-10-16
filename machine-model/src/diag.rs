//! 診斷訊息：規則編號、等級、繁體中文訊息與位置。

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 規則編號，例如 `R3`；解析失敗但不屬於特定規則時為 `PARSE`。
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    /// 人讀的位置，例如 `tags[plc1.X000]`。
    pub location: Option<String>,
}

impl Diagnostic {
    pub fn error(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            location: Some(location.into()),
        }
    }

    pub fn warning(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            location: Some(location.into()),
        }
    }

    pub fn info(
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Info,
            message: message.into(),
            location: Some(location.into()),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "[{}] {}：{}", self.code, loc, self.message),
            None => write!(f, "[{}] {}", self.code, self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_code_location_and_message() {
        let d = Diagnostic::error("R1", "tags[plc1.X000]", "id 格式不符");
        assert_eq!(d.to_string(), "[R1] tags[plc1.X000]：id 格式不符");
        assert!(d.is_error());
    }

    #[test]
    fn warning_and_info_are_not_errors() {
        let w = Diagnostic::warning("U14", "tags[plc1.A]", "role 與前綴不同");
        let i = Diagnostic::info("U17", "types[x]", "本地覆蓋");
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(i.severity, Severity::Info);
        assert!(!w.is_error() && !i.is_error());
    }
}
