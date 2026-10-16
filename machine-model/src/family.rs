//! 家族設定表：每個 PLC 家族的位址類別、進位、範圍與可用協定。
//!
//! 這是**資料**，不是散落在程式裡的判斷（spec G9）。補規則就是加資料。
//! 只有 Inovance 的 `BOOL` 對 X／Y，以及三菱 X／Y 的 `bool` 經黃金案例驗證；
//! 其餘範圍（尤其三菱的上限）標為待驗證，見各常數的註解。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    InovanceEasy,
    MitsubishiFx3g,
    MitsubishiFx3s,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    Octal,
    Decimal,
}

impl Radix {
    pub fn base(self) -> u32 {
        match self {
            Self::Octal => 8,
            Self::Decimal => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// 每個位址是一個位元（線圈、離散輸入）。
    Bit,
    /// 每個位址是一個 16 位元字組。
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassProfile {
    pub name: &'static str,
    pub radix: Radix,
    /// 允許的最大編號（含），以數值表示（八進位 `0o1777` 即 1023）。
    pub max: u32,
    pub width: Width,
    /// 唯讀輸入（例如 `X`）。
    pub read_only: bool,
    /// 是否接受 `.0`～`.15` 的位元後綴（Inovance 的 `D`、`R`）。
    pub bit_suffix: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyProfile {
    pub classes: &'static [ClassProfile],
    /// `service.toml` 可用的協定名稱；空表示沒有可用協定，只能 `online: false`。
    pub protocols: &'static [&'static str],
}

const fn class(
    name: &'static str,
    radix: Radix,
    max: u32,
    width: Width,
    read_only: bool,
    bit_suffix: bool,
) -> ClassProfile {
    ClassProfile {
        name,
        radix,
        max,
        width,
        read_only,
        bit_suffix,
    }
}

/// 範圍取自 machine-model.md §4.4（來源：Driver 的 Addressing）。
static INOVANCE_EASY: FamilyProfile = FamilyProfile {
    classes: &[
        class("D", Radix::Decimal, 7999, Width::Word, false, true),
        class("SD", Radix::Decimal, 1023, Width::Word, false, false),
        class("R", Radix::Decimal, 32767, Width::Word, false, true),
        class("M", Radix::Decimal, 7999, Width::Bit, false, false),
        class("SM", Radix::Decimal, 1023, Width::Bit, false, false),
        class("B", Radix::Decimal, 32767, Width::Bit, false, false),
        class("S", Radix::Decimal, 4095, Width::Bit, false, false),
        class("X", Radix::Octal, 0o1777, Width::Bit, true, false),
        class("Y", Radix::Octal, 0o1777, Width::Bit, false, false),
    ],
    protocols: &["inovance_easy_rtu"],
};

/// 三菱 FX 的 X／Y 為八進位。上限 `0o377` 是 P1 的寬鬆值，**未對照機型手冊驗證**。
static MITSUBISHI_FX: FamilyProfile = FamilyProfile {
    classes: &[
        class("X", Radix::Octal, 0o377, Width::Bit, true, false),
        class("Y", Radix::Octal, 0o377, Width::Bit, false, false),
    ],
    protocols: &[],
};

impl Family {
    pub fn profile(self) -> &'static FamilyProfile {
        match self {
            Self::InovanceEasy => &INOVANCE_EASY,
            Self::MitsubishiFx3g | Self::MitsubishiFx3s => &MITSUBISHI_FX,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::InovanceEasy => "inovance_easy",
            Self::MitsubishiFx3g => "mitsubishi_fx3g",
            Self::MitsubishiFx3s => "mitsubishi_fx3s",
        }
    }

    /// 該家族能否搭配 `service.toml` 的協定名稱。
    pub fn protocol_allowed(self, protocol: &str) -> bool {
        self.profile().protocols.contains(&protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inovance_x_and_y_are_octal_and_x_is_read_only() {
        let p = Family::InovanceEasy.profile();
        let x = p.classes.iter().find(|c| c.name == "X").unwrap();
        let y = p.classes.iter().find(|c| c.name == "Y").unwrap();
        assert_eq!(x.radix, Radix::Octal);
        assert_eq!(x.max, 1023);
        assert!(x.read_only);
        assert!(!y.read_only);
    }

    #[test]
    fn protocol_compatibility() {
        assert!(Family::InovanceEasy.protocol_allowed("inovance_easy_rtu"));
        assert!(!Family::InovanceEasy.protocol_allowed("modbus_rtu"));
        assert!(!Family::MitsubishiFx3g.protocol_allowed("slmp"));
        assert_eq!(Family::MitsubishiFx3s.profile().protocols, &[] as &[&str]);
    }

    #[test]
    fn family_names_round_trip_through_as_str() {
        for f in [
            Family::InovanceEasy,
            Family::MitsubishiFx3g,
            Family::MitsubishiFx3s,
        ] {
            let parsed: Family = serde_yaml_ng::from_str(f.as_str()).unwrap();
            assert_eq!(parsed, f);
        }
    }
}
