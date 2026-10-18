//! 邏輯型別詞彙（沿用 `PlcDataService` 的寫法，見 machine-model.md §4.2）。

use std::fmt;

pub use plc_common::ScalarType;
use plc_common::ScalarTypeError;

/// 陣列長度上限（與 `PlcDataService` 的 `MAX_ARRAY_LEN` 相同）。
pub const MAX_ARRAY_LEN: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataType {
    pub scalar: ScalarType,
    /// 純量為 1。
    pub len: usize,
    pub array: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    Unknown(String),
    BadStringLength(String),
    BadArrayLength(String),
    StringArray,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(t) => write!(f, "不認得的型別 `{t}`"),
            Self::BadStringLength(n) => write!(f, "字串長度 `{n}` 無效，必須是 1 到 256 的整數"),
            Self::BadArrayLength(n) => {
                write!(f, "陣列長度 `{n}` 無效，必須是 1 到 {MAX_ARRAY_LEN} 的整數")
            }
            Self::StringArray => write!(f, "不支援字串陣列"),
        }
    }
}

fn digits_only(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

impl From<ScalarTypeError> for TypeError {
    fn from(e: ScalarTypeError) -> Self {
        match e {
            ScalarTypeError::Unknown(t) => Self::Unknown(t),
            ScalarTypeError::BadStringLength(n) => Self::BadStringLength(n),
        }
    }
}

impl DataType {
    /// 解析 `bool`、`int16[10]`、`string:16` 這類寫法。
    pub fn parse(text: &str) -> Result<Self, TypeError> {
        if let Some(open) = text.find('[') {
            let Some(inner) = text[open + 1..].strip_suffix(']') else {
                return Err(TypeError::Unknown(text.to_owned()));
            };
            if !digits_only(inner) {
                return Err(TypeError::BadArrayLength(inner.to_owned()));
            }
            let len = inner
                .parse::<usize>()
                .map_err(|_| TypeError::BadArrayLength(inner.to_owned()))?;
            if !(1..=MAX_ARRAY_LEN).contains(&len) {
                return Err(TypeError::BadArrayLength(inner.to_owned()));
            }
            let scalar = ScalarType::parse(&text[..open])?;
            if matches!(scalar, ScalarType::Text(_)) {
                return Err(TypeError::StringArray);
            }
            return Ok(Self {
                scalar,
                len,
                array: true,
            });
        }
        Ok(Self {
            scalar: ScalarType::parse(text)?,
            len: 1,
            array: false,
        })
    }

    pub fn is_bool(&self) -> bool {
        self.scalar == ScalarType::Bool
    }

    /// 在字組類位址上佔用的字組數（陣列為各元素之和）。
    pub fn words(&self) -> usize {
        let per = match self.scalar {
            ScalarType::Bool | ScalarType::Int16 | ScalarType::Uint16 => 1,
            ScalarType::Int32 | ScalarType::Uint32 | ScalarType::Float32 => 2,
            ScalarType::Int64 | ScalarType::Uint64 | ScalarType::Float64 => 4,
            ScalarType::Text(n) => usize::from(n).div_ceil(2),
        };
        per * self.len
    }

    /// 整數型別的可表示範圍；`uint64` 受 `i64` 限制而取到 `i64::MAX`。
    pub fn int_bounds(&self) -> Option<(i64, i64)> {
        match self.scalar {
            ScalarType::Int16 => Some((i64::from(i16::MIN), i64::from(i16::MAX))),
            ScalarType::Uint16 => Some((0, i64::from(u16::MAX))),
            ScalarType::Int32 => Some((i64::from(i32::MIN), i64::from(i32::MAX))),
            ScalarType::Uint32 => Some((0, i64::from(u32::MAX))),
            ScalarType::Int64 => Some((i64::MIN, i64::MAX)),
            ScalarType::Uint64 => Some((0, i64::MAX)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars() {
        assert_eq!(DataType::parse("bool").unwrap().scalar, ScalarType::Bool);
        assert_eq!(
            DataType::parse("float64").unwrap().scalar,
            ScalarType::Float64
        );
        let t = DataType::parse("string:16").unwrap();
        assert_eq!(t.scalar, ScalarType::Text(16));
        assert!(!t.array);
    }

    #[test]
    fn parses_arrays() {
        let t = DataType::parse("int16[10]").unwrap();
        assert_eq!((t.scalar, t.len, t.array), (ScalarType::Int16, 10, true));
        assert_eq!(t.words(), 10);
        assert!(DataType::parse("int16[500]").is_ok());
    }

    #[test]
    fn rejects_bad_types() {
        assert!(matches!(DataType::parse("int"), Err(TypeError::Unknown(_))));
        assert!(matches!(
            DataType::parse("Bool"),
            Err(TypeError::Unknown(_))
        ));
        assert!(matches!(DataType::parse(""), Err(TypeError::Unknown(_))));
        assert!(matches!(
            DataType::parse("string:0"),
            Err(TypeError::BadStringLength(_))
        ));
        assert!(matches!(
            DataType::parse("string:+4"),
            Err(TypeError::BadStringLength(_))
        ));
        assert!(matches!(
            DataType::parse("string:"),
            Err(TypeError::BadStringLength(_))
        ));
        assert!(matches!(
            DataType::parse("int16[0]"),
            Err(TypeError::BadArrayLength(_))
        ));
        assert!(matches!(
            DataType::parse("int16[501]"),
            Err(TypeError::BadArrayLength(_))
        ));
        assert!(matches!(
            DataType::parse("int16[-1]"),
            Err(TypeError::BadArrayLength(_))
        ));
        assert!(matches!(
            DataType::parse("int16[3"),
            Err(TypeError::Unknown(_))
        ));
        assert_eq!(DataType::parse("string:4[2]"), Err(TypeError::StringArray));
    }

    #[test]
    fn string_length_is_capped_at_256() {
        assert!(DataType::parse("string:256").is_ok());
        assert!(matches!(
            DataType::parse("string:257"),
            Err(TypeError::BadStringLength(_))
        ));
    }

    #[test]
    fn word_widths() {
        assert_eq!(DataType::parse("bool").unwrap().words(), 1);
        assert_eq!(DataType::parse("int32").unwrap().words(), 2);
        assert_eq!(DataType::parse("float64").unwrap().words(), 4);
        assert_eq!(DataType::parse("string:5").unwrap().words(), 3);
        assert_eq!(DataType::parse("string:256").unwrap().words(), 128);
        assert_eq!(DataType::parse("float32[3]").unwrap().words(), 6);
    }

    #[test]
    fn int_bounds_cover_integer_types_only() {
        assert_eq!(
            DataType::parse("int16").unwrap().int_bounds(),
            Some((-32768, 32767))
        );
        assert_eq!(
            DataType::parse("uint16").unwrap().int_bounds(),
            Some((0, 65535))
        );
        assert_eq!(DataType::parse("float32").unwrap().int_bounds(), None);
        assert_eq!(DataType::parse("bool").unwrap().int_bounds(), None);
    }
}
