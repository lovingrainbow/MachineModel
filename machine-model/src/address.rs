//! 位址解析：依家族設定表把 `X010`、`D100`、`R5.3` 解析成「類別＋數值＋位元」。
//!
//! 位址校驗以本模組為準，執行期解碼以 Driver 為準（spec §2.4）。

use std::fmt;

use crate::family::{ClassProfile, Family};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    /// 解析時命中的位址類別（來自家族設定表，與解析所用的家族一致）。
    pub class: &'static ClassProfile,
    /// 以數值表示的編號（`X010` 為 8）。
    pub number: u32,
    pub bit: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    Empty,
    UnknownClass(String),
    BadNumber(String),
    OutOfRange {
        class: &'static str,
        number: u32,
        max: u32,
    },
    BitNotAllowed(&'static str),
    BadBit(String),
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "位址為空"),
            Self::UnknownClass(c) => write!(f, "此 PLC 家族沒有位址類別 `{c}`"),
            Self::BadNumber(n) => write!(f, "位址編號 `{n}` 無效（進位或字元不符）"),
            Self::OutOfRange { class, number, max } => {
                write!(f, "{class} 類別的編號 {number} 超過上限 {max}")
            }
            Self::BitNotAllowed(c) => write!(f, "{c} 類別不接受位元後綴"),
            Self::BadBit(b) => write!(f, "位元後綴 `{b}` 無效，必須是 0 到 15"),
        }
    }
}

/// 解析位址。類別字母不分大小寫；編號依該類別的進位解析（八進位類別拒絕 8、9）。
pub fn parse(family: Family, text: &str) -> Result<Address, AddressError> {
    if text.is_empty() {
        return Err(AddressError::Empty);
    }
    let letters_end = text
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(text.len());
    let letters = text[..letters_end].to_ascii_uppercase();
    let rest = &text[letters_end..];
    let classes = family.profile().classes;
    let Some(class) = classes.iter().find(|c| c.name == letters) else {
        return Err(AddressError::UnknownClass(letters));
    };

    let (digits, bit_text) = match rest.split_once('.') {
        Some((d, b)) => (d, Some(b)),
        None => (rest, None),
    };
    let valid_digits = !digits.is_empty() && digits.chars().all(|c| c.is_digit(class.radix.base()));
    if !valid_digits {
        return Err(AddressError::BadNumber(digits.to_owned()));
    }
    let number = u32::from_str_radix(digits, class.radix.base())
        .map_err(|_| AddressError::BadNumber(digits.to_owned()))?;
    if number > class.max {
        return Err(AddressError::OutOfRange {
            class: class.name,
            number,
            max: class.max,
        });
    }

    let bit = match bit_text {
        None => None,
        Some(b) => {
            if !class.bit_suffix {
                return Err(AddressError::BitNotAllowed(class.name));
            }
            if b.is_empty() || !b.bytes().all(|c| c.is_ascii_digit()) {
                return Err(AddressError::BadBit(b.to_owned()));
            }
            match b.parse::<u8>() {
                Ok(v) if v <= 15 => Some(v),
                _ => return Err(AddressError::BadBit(b.to_owned())),
            }
        }
    };
    Ok(Address { class, number, bit })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(family: Family, s: &str) -> Result<Address, AddressError> {
        parse(family, s)
    }

    #[test]
    fn octal_x_is_not_decimal() {
        let a = p(Family::InovanceEasy, "X010").unwrap();
        assert_eq!(a.number, 8);
        let b = p(Family::InovanceEasy, "X7").unwrap();
        let c = p(Family::InovanceEasy, "X007").unwrap();
        assert_eq!(b, c);
    }

    #[test]
    fn octal_class_rejects_digits_8_and_9() {
        assert!(matches!(
            p(Family::InovanceEasy, "X008"),
            Err(AddressError::BadNumber(_))
        ));
        assert!(matches!(
            p(Family::MitsubishiFx3g, "Y019"),
            Err(AddressError::BadNumber(_))
        ));
    }

    #[test]
    fn decimal_classes_accept_8_and_9() {
        assert_eq!(p(Family::InovanceEasy, "M19").unwrap().number, 19);
        assert_eq!(p(Family::InovanceEasy, "D100").unwrap().number, 100);
    }

    #[test]
    fn longest_class_name_wins_and_case_is_ignored() {
        let a = p(Family::InovanceEasy, "sd10").unwrap();
        assert_eq!(a.class.name, "SD");
        let b = p(Family::InovanceEasy, "S10").unwrap();
        assert_eq!(b.class.name, "S");
    }

    #[test]
    fn range_limits() {
        assert!(p(Family::InovanceEasy, "X1777").is_ok());
        assert!(matches!(
            p(Family::InovanceEasy, "X2000"),
            Err(AddressError::OutOfRange { .. })
        ));
        assert!(p(Family::InovanceEasy, "D7999").is_ok());
        assert!(matches!(
            p(Family::InovanceEasy, "D8000"),
            Err(AddressError::OutOfRange { .. })
        ));
    }

    #[test]
    fn bit_suffix_on_d_and_r_but_not_other_classes() {
        let a = p(Family::InovanceEasy, "R100.15").unwrap();
        assert_eq!(a.bit, Some(15));
        let d = p(Family::InovanceEasy, "D0.1").unwrap();
        assert_eq!((d.number, d.bit), (0, Some(1)));
        assert!(matches!(
            p(Family::InovanceEasy, "R100.16"),
            Err(AddressError::BadBit(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "R100."),
            Err(AddressError::BadBit(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "M100.1"),
            Err(AddressError::BitNotAllowed("M"))
        ));
    }

    #[test]
    fn malformed_addresses() {
        assert_eq!(p(Family::InovanceEasy, ""), Err(AddressError::Empty));
        assert!(matches!(
            p(Family::InovanceEasy, "Q10"),
            Err(AddressError::UnknownClass(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "X"),
            Err(AddressError::BadNumber(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "X 10"),
            Err(AddressError::BadNumber(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "X+7"),
            Err(AddressError::BadNumber(_))
        ));
        assert!(matches!(
            p(Family::InovanceEasy, "D99999999999"),
            Err(AddressError::BadNumber(_))
        ));
        assert!(matches!(
            p(Family::MitsubishiFx3g, "D100"),
            Err(AddressError::UnknownClass(_))
        ));
    }
}
