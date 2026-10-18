//! 家族設定表：每個 PLC 家族（即驅動型號）的位址語法、位址類別與範圍。
//!
//! 這是**資料**，不是散落在程式裡的判斷（spec G9）。補規則就是加資料。
//! 家族名稱與 `DataService` `mm-drivers` 的驅動型號相同（spec §3.3），共 55 種 PLC 驅動；
//! 非 PLC 的驅動（機器人、CNC、儀表、讀碼器、扭力槍、模擬器）不在此表中。
//!
//! 可信度分三級，見各常數的註解：
//! - **已驗證**：匯川 Easy 的 `X`／`Y`、三菱 FX 的 `X`／`Y`（沿用既有黃金案例）。
//! - **依廠商慣例、待驗證**：前綴來自驅動型錄的位址範例；進位與字／位元寬度依廠商慣例；
//!   範圍一律為暫用上限 [`UNVERIFIED_MAX`]，未經實機或手冊對照。
//! - **非空（`Grammar::Opaque`）**：位址有結構性語法（如 `DB1.DBW0`、`N7:0`、標籤名稱），
//!   只檢查非空，不檢查範圍。

use serde::{Deserialize, Serialize};

/// PLC 家族（驅動型號）。序列化名稱即驅動型號，例如 `inovance_easy_tcp`。
///
/// 舊名 `inovance_easy`、`mitsubishi_fx3g`、`mitsubishi_fx3s` 保留為別名（spec §3.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    // 三菱 MELSEC
    MelsecMcBinary,
    MelsecMcAscii,
    MelsecMcRBinary,
    MelsecA1EBinary,
    MelsecA1EAscii,
    MelsecA3CTcp,
    #[serde(alias = "mitsubishi_fx3g")]
    MelsecFxSerialTcp,
    #[serde(alias = "mitsubishi_fx3s")]
    MelsecFxLinksTcp,
    MelsecCipNet,
    // 西門子 S7
    SiemensS7_1200,
    SiemensS7_1500,
    SiemensS7_300,
    SiemensS7_400,
    SiemensS7_200Smart,
    SiemensS7_200,
    SiemensFetchWrite,
    SiemensPpiTcp,
    // 歐姆龍
    OmronFinsTcp,
    OmronFinsUdp,
    OmronCipNet,
    OmronHostLinkTcp,
    OmronHostLinkCModeTcp,
    // 基恩士
    KeyenceMcBinary,
    KeyenceMcAscii,
    KeyenceNanoTcp,
    // 匯川
    #[serde(alias = "inovance_easy")]
    InovanceEasyTcp,
    InovanceH3uTcp,
    InovanceH5uTcp,
    InovanceAmTcp,
    // 台達
    DeltaDvpTcp,
    DeltaAsTcp,
    DeltaSerialAsciiTcp,
    // Allen-Bradley
    AllenBradleyCip,
    AllenBradleyMicroCip,
    AllenBradleySlcNet,
    AllenBradleyPccc,
    AllenBradleyDf1Tcp,
    // 倍福
    BeckhoffAds,
    // 松下
    PanasonicMewtocolTcp,
    PanasonicMcBinary,
    // 永宏
    FatekProgramTcp,
    // LS 樂星
    LSisFastEnet,
    LSisCnetTcp,
    // 富士
    FujiSpbTcp,
    FujiSphNet,
    FujiCsType,
    // 信捷
    XinjeTcp,
    XinjeInternalNet,
    XinjeSerialTcp,
    // 橫河、安川、GE、豐煒、豐田、西蒙
    YokogawaLinkTcp,
    YaskawaMemobusTcp,
    GeSrtpTcp,
    VigorTcp,
    ToyotaToyoPuc,
    CimonHmiTcp,
}

/// 進位。八進位用於匯川 Easy 與三菱 FX 的 `X`／`Y`，十六進位用於三菱 MC 的部分軟元件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    Octal,
    Decimal,
    Hexadecimal,
}

impl Radix {
    pub fn base(self) -> u32 {
        match self {
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
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
    /// 是否接受 `.0`～`.15` 的位元後綴（例如 `D100.3`）。
    pub bit_suffix: bool,
    /// 非空語法的佔位類別（見 [`OPAQUE_CLASS`]），不參與範圍、型別與重疊檢查。
    pub opaque: bool,
}

/// 位址語法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    /// 前綴加編號（可接 `.位元`），前綴必須在表中。
    Classes(&'static [ClassProfile]),
    /// 有結構性語法（如 `DB1.DBW0`、`N7:0`、標籤名），只檢查非空。
    Opaque,
}

/// 非空語法位址所用的佔位類別。
pub static OPAQUE_CLASS: ClassProfile = ClassProfile {
    name: "",
    radix: Radix::Decimal,
    max: u32::MAX,
    width: Width::Word,
    read_only: false,
    bit_suffix: false,
    opaque: true,
};

/// 範圍未經實機或手冊驗證時的暫用上限（數值）。
const UNVERIFIED_MAX: u32 = 65535;

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
        opaque: false,
    }
}

// ----------------------------------------------------------------------------
// 已驗證
// ----------------------------------------------------------------------------

/// 範圍取自 machine-model.md §4.4（來源：Driver 的 Addressing）。
/// `X`／`Y` 八進位、`X` 唯讀已由黃金案例驗證；其餘範圍沿用同一來源。
/// 同時用於 H3U 與 H5U（兩者的 `D`、`M`、`SM`、`SD`、`R`、`B` 範圍待各自手冊驗證）。
static INOVANCE_EASY: &[ClassProfile] = &[
    class("D", Radix::Decimal, 7999, Width::Word, false, true),
    class("SD", Radix::Decimal, 1023, Width::Word, false, false),
    class("R", Radix::Decimal, 32767, Width::Word, false, true),
    class("M", Radix::Decimal, 7999, Width::Bit, false, false),
    class("SM", Radix::Decimal, 1023, Width::Bit, false, false),
    class("B", Radix::Decimal, 32767, Width::Bit, false, false),
    class("S", Radix::Decimal, 4095, Width::Bit, false, false),
    class("X", Radix::Octal, 0o1777, Width::Bit, true, false),
    class("Y", Radix::Octal, 0o1777, Width::Bit, false, false),
];

/// 三菱 FX 的 X／Y 為八進位。上限 `0o377` 是 P1 的寬鬆值，**未對照機型手冊驗證**。
/// 目前只收 X／Y；D、M 等需要手冊範圍後再加入。
static MITSUBISHI_FX: &[ClassProfile] = &[
    class("X", Radix::Octal, 0o377, Width::Bit, true, false),
    class("Y", Radix::Octal, 0o377, Width::Bit, false, false),
];

// ----------------------------------------------------------------------------
// 依廠商慣例、待驗證（範圍一律為 UNVERIFIED_MAX）
// ----------------------------------------------------------------------------

/// 三菱 MC 協定（Q／QnA／iQ-R 系列）的常見軟元件。
/// 前綴來自型錄範例；`W`、`X`、`Y`、`B` 採十六進位，屬廠商慣例，待驗證。
static MELSEC_MC: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "W",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "X",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        true,
        false,
    ),
    class(
        "Y",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "B",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// iQ-R 系列多了檔案暫存器 `ZR`。
static MELSEC_MC_R: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "W",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "ZR",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "X",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        true,
        false,
    ),
    class(
        "Y",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "B",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 西門子 S7-200／200 Smart／PPI 的 `V`（位元組區）、`M`、`I`（輸入，唯讀）、`Q`（輸出）。
/// `I0.0` 的 `.0` 為位元索引，因此 `I`、`Q` 接受位元後綴。
static SIEMENS_S7_200: &[ClassProfile] = &[
    class(
        "V",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("I", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, true, true),
    class("Q", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, false, true),
];

/// 歐姆龍 FINS：`D`、`CIO`、`W`、`H`、`A` 皆為字組，可接位元後綴（如 `CIO10.0`）。
static OMRON_FINS: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "CIO",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "W",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "H",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "A",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
];

/// 歐姆龍 HostLink（上位鏈路）。
static OMRON_HOSTLINK: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "CIO",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "W",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "H",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
];

/// 歐姆龍 `HostLink` C-Mode。
static OMRON_CMODE: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "DM",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "LR",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "HR",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
];

/// 基恩士 KV／Nano 系列：`DM`、`EM` 為字組，`MR`、`R` 為繼電器位元。
static KEYENCE: &[ClassProfile] = &[
    class(
        "DM",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "EM",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "MR",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 匯川 AM 系列：以 `MW`、`MX`、`IW`、`QW` 表示字組與位元。
static INOVANCE_AM: &[ClassProfile] = &[
    class(
        "MW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "MX",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        true,
    ),
    class(
        "IW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        true,
        false,
    ),
    class(
        "QW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
];

/// 台達 DVP 系列。`X`、`Y` 為八進位（廠商慣例，待驗證）。
static DELTA: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("X", Radix::Octal, UNVERIFIED_MAX, Width::Bit, true, false),
    class("Y", Radix::Octal, UNVERIFIED_MAX, Width::Bit, false, false),
];

/// 台達 AS 系列：`X0.0` 形式的位元後綴。
static DELTA_AS: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("X", Radix::Octal, UNVERIFIED_MAX, Width::Bit, true, true),
    class("Y", Radix::Octal, UNVERIFIED_MAX, Width::Bit, false, true),
];

/// 松下 FP（Mewtocol）：`DT` 字組，`R`、`X`、`Y` 為位元。
static PANASONIC_FP: &[ClassProfile] = &[
    class(
        "DT",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "X",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        true,
        false,
    ),
    class(
        "Y",
        Radix::Hexadecimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 松下 MC 協定。
static PANASONIC_MC: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 永宏 FBS。
static FATEK: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("X", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, true, false),
    class(
        "Y",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// LS XGK／XGB `FEnet`：`DW`、`MW` 為字組，`MX` 為位元。
static LS_XGB_FENET: &[ClassProfile] = &[
    class(
        "DW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "MW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "MX",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// LS XGB Cnet。
static LS_XGB_CNET: &[ClassProfile] = &[
    class(
        "DW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "MW",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
];

/// 富士 FLEX-PC（SPB）。
static FUJI_SPB: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 信捷 XC 系列（含 `HD`、`HM`）。
static XINJE: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "HD",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "HM",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 信捷內部網路與串列over TCP 的基本軟元件。
static XINJE_BASIC: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 橫河 FA-M3 Link。
static YOKOGAWA: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "B",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("I", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, true, false),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 安川 MEMOBUS（MP2000／MP3000）。
static YASKAWA: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// GE `PACSystems` SRTP。`AI`、`I` 為唯讀輸入。
static GE_SRTP: &[ClassProfile] = &[
    class(
        "R",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "AI",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        true,
        false,
    ),
    class("I", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, true, false),
    class(
        "Q",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 豐煒 VB／VH。
static VIGOR: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class("X", Radix::Decimal, UNVERIFIED_MAX, Width::Bit, true, false),
    class(
        "Y",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

/// 豐田 PC10G／PC3JG（PUC 協定）。
static TOYOTA: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
    class(
        "K",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
];

/// 西蒙 CIMON HMI 協定。
static CIMON: &[ClassProfile] = &[
    class(
        "D",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Word,
        false,
        false,
    ),
    class(
        "M",
        Radix::Decimal,
        UNVERIFIED_MAX,
        Width::Bit,
        false,
        false,
    ),
];

impl Family {
    /// 全部 55 種 PLC 家族（驅動型號）。
    pub const ALL: &'static [Family] = &[
        Family::MelsecMcBinary,
        Family::MelsecMcAscii,
        Family::MelsecMcRBinary,
        Family::MelsecA1EBinary,
        Family::MelsecA1EAscii,
        Family::MelsecA3CTcp,
        Family::MelsecFxSerialTcp,
        Family::MelsecFxLinksTcp,
        Family::MelsecCipNet,
        Family::SiemensS7_1200,
        Family::SiemensS7_1500,
        Family::SiemensS7_300,
        Family::SiemensS7_400,
        Family::SiemensS7_200Smart,
        Family::SiemensS7_200,
        Family::SiemensFetchWrite,
        Family::SiemensPpiTcp,
        Family::OmronFinsTcp,
        Family::OmronFinsUdp,
        Family::OmronCipNet,
        Family::OmronHostLinkTcp,
        Family::OmronHostLinkCModeTcp,
        Family::KeyenceMcBinary,
        Family::KeyenceMcAscii,
        Family::KeyenceNanoTcp,
        Family::InovanceEasyTcp,
        Family::InovanceH3uTcp,
        Family::InovanceH5uTcp,
        Family::InovanceAmTcp,
        Family::DeltaDvpTcp,
        Family::DeltaAsTcp,
        Family::DeltaSerialAsciiTcp,
        Family::AllenBradleyCip,
        Family::AllenBradleyMicroCip,
        Family::AllenBradleySlcNet,
        Family::AllenBradleyPccc,
        Family::AllenBradleyDf1Tcp,
        Family::BeckhoffAds,
        Family::PanasonicMewtocolTcp,
        Family::PanasonicMcBinary,
        Family::FatekProgramTcp,
        Family::LSisFastEnet,
        Family::LSisCnetTcp,
        Family::FujiSpbTcp,
        Family::FujiSphNet,
        Family::FujiCsType,
        Family::XinjeTcp,
        Family::XinjeInternalNet,
        Family::XinjeSerialTcp,
        Family::YokogawaLinkTcp,
        Family::YaskawaMemobusTcp,
        Family::GeSrtpTcp,
        Family::VigorTcp,
        Family::ToyotaToyoPuc,
        Family::CimonHmiTcp,
    ];

    /// 位址語法。
    pub fn grammar(self) -> Grammar {
        // 表格式 match：省略 `Family::` 前綴，讓每個家族只佔一列。
        #[allow(clippy::enum_glob_use)]
        use Family::*;
        match self {
            // 結構化位址（無法以前綴表描述），語法不透明。
            MelsecCipNet | SiemensS7_1200 | SiemensS7_1500 | SiemensS7_300 | SiemensS7_400
            | SiemensFetchWrite | OmronCipNet | AllenBradleyCip | AllenBradleyMicroCip
            | AllenBradleySlcNet | AllenBradleyPccc | AllenBradleyDf1Tcp | BeckhoffAds
            | FujiSphNet | FujiCsType => Grammar::Opaque,
            MelsecMcBinary | MelsecMcAscii | MelsecA1EBinary | MelsecA1EAscii | MelsecA3CTcp => {
                Grammar::Classes(MELSEC_MC)
            }
            MelsecMcRBinary => Grammar::Classes(MELSEC_MC_R),
            MelsecFxSerialTcp | MelsecFxLinksTcp => Grammar::Classes(MITSUBISHI_FX),
            SiemensS7_200Smart | SiemensS7_200 | SiemensPpiTcp => Grammar::Classes(SIEMENS_S7_200),
            OmronFinsTcp | OmronFinsUdp => Grammar::Classes(OMRON_FINS),
            OmronHostLinkTcp => Grammar::Classes(OMRON_HOSTLINK),
            OmronHostLinkCModeTcp => Grammar::Classes(OMRON_CMODE),
            KeyenceMcBinary | KeyenceMcAscii | KeyenceNanoTcp => Grammar::Classes(KEYENCE),
            InovanceEasyTcp | InovanceH3uTcp | InovanceH5uTcp => Grammar::Classes(INOVANCE_EASY),
            InovanceAmTcp => Grammar::Classes(INOVANCE_AM),
            DeltaDvpTcp | DeltaSerialAsciiTcp => Grammar::Classes(DELTA),
            DeltaAsTcp => Grammar::Classes(DELTA_AS),
            PanasonicMewtocolTcp => Grammar::Classes(PANASONIC_FP),
            PanasonicMcBinary => Grammar::Classes(PANASONIC_MC),
            FatekProgramTcp => Grammar::Classes(FATEK),
            LSisFastEnet => Grammar::Classes(LS_XGB_FENET),
            LSisCnetTcp => Grammar::Classes(LS_XGB_CNET),
            FujiSpbTcp => Grammar::Classes(FUJI_SPB),
            XinjeTcp => Grammar::Classes(XINJE),
            XinjeInternalNet | XinjeSerialTcp => Grammar::Classes(XINJE_BASIC),
            YokogawaLinkTcp => Grammar::Classes(YOKOGAWA),
            YaskawaMemobusTcp => Grammar::Classes(YASKAWA),
            GeSrtpTcp => Grammar::Classes(GE_SRTP),
            VigorTcp => Grammar::Classes(VIGOR),
            ToyotaToyoPuc => Grammar::Classes(TOYOTA),
            CimonHmiTcp => Grammar::Classes(CIMON),
        }
    }

    /// 驅動型號名稱（與 `serde` 序列化名稱相同）。
    pub fn as_str(self) -> &'static str {
        // 表格式 match：省略 `Family::` 前綴，讓每個家族只佔一列。
        #[allow(clippy::enum_glob_use)]
        use Family::*;
        match self {
            MelsecMcBinary => "melsec_mc_binary",
            MelsecMcAscii => "melsec_mc_ascii",
            MelsecMcRBinary => "melsec_mc_r_binary",
            MelsecA1EBinary => "melsec_a1_e_binary",
            MelsecA1EAscii => "melsec_a1_e_ascii",
            MelsecA3CTcp => "melsec_a3_c_tcp",
            MelsecFxSerialTcp => "melsec_fx_serial_tcp",
            MelsecFxLinksTcp => "melsec_fx_links_tcp",
            MelsecCipNet => "melsec_cip_net",
            SiemensS7_1200 => "siemens_s7_1200",
            SiemensS7_1500 => "siemens_s7_1500",
            SiemensS7_300 => "siemens_s7_300",
            SiemensS7_400 => "siemens_s7_400",
            SiemensS7_200Smart => "siemens_s7_200_smart",
            SiemensS7_200 => "siemens_s7_200",
            SiemensFetchWrite => "siemens_fetch_write",
            SiemensPpiTcp => "siemens_ppi_tcp",
            OmronFinsTcp => "omron_fins_tcp",
            OmronFinsUdp => "omron_fins_udp",
            OmronCipNet => "omron_cip_net",
            OmronHostLinkTcp => "omron_host_link_tcp",
            OmronHostLinkCModeTcp => "omron_host_link_c_mode_tcp",
            KeyenceMcBinary => "keyence_mc_binary",
            KeyenceMcAscii => "keyence_mc_ascii",
            KeyenceNanoTcp => "keyence_nano_tcp",
            InovanceEasyTcp => "inovance_easy_tcp",
            InovanceH3uTcp => "inovance_h3u_tcp",
            InovanceH5uTcp => "inovance_h5u_tcp",
            InovanceAmTcp => "inovance_am_tcp",
            DeltaDvpTcp => "delta_dvp_tcp",
            DeltaAsTcp => "delta_as_tcp",
            DeltaSerialAsciiTcp => "delta_serial_ascii_tcp",
            AllenBradleyCip => "allen_bradley_cip",
            AllenBradleyMicroCip => "allen_bradley_micro_cip",
            AllenBradleySlcNet => "allen_bradley_slc_net",
            AllenBradleyPccc => "allen_bradley_pccc",
            AllenBradleyDf1Tcp => "allen_bradley_df1_tcp",
            BeckhoffAds => "beckhoff_ads",
            PanasonicMewtocolTcp => "panasonic_mewtocol_tcp",
            PanasonicMcBinary => "panasonic_mc_binary",
            FatekProgramTcp => "fatek_program_tcp",
            LSisFastEnet => "l_sis_fast_enet",
            LSisCnetTcp => "l_sis_cnet_tcp",
            FujiSpbTcp => "fuji_spb_tcp",
            FujiSphNet => "fuji_sph_net",
            FujiCsType => "fuji_cs_type",
            XinjeTcp => "xinje_tcp",
            XinjeInternalNet => "xinje_internal_net",
            XinjeSerialTcp => "xinje_serial_tcp",
            YokogawaLinkTcp => "yokogawa_link_tcp",
            YaskawaMemobusTcp => "yaskawa_memobus_tcp",
            GeSrtpTcp => "ge_srtp_tcp",
            VigorTcp => "vigor_tcp",
            ToyotaToyoPuc => "toyota_toyo_puc",
            CimonHmiTcp => "cimon_hmi_tcp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(family: Family) -> &'static [ClassProfile] {
        match family.grammar() {
            Grammar::Classes(classes) => classes,
            Grammar::Opaque => panic!("{} 應為前綴表語法", family.as_str()),
        }
    }

    #[test]
    fn inovance_x_and_y_are_octal_and_x_is_read_only() {
        let c = classes(Family::InovanceEasyTcp);
        let x = c.iter().find(|c| c.name == "X").unwrap();
        let y = c.iter().find(|c| c.name == "Y").unwrap();
        assert_eq!(x.radix, Radix::Octal);
        assert_eq!(x.max, 1023);
        assert!(x.read_only);
        assert!(!y.read_only);
    }

    #[test]
    fn there_are_55_plc_families_with_unique_names() {
        assert_eq!(Family::ALL.len(), 55);
        let mut names: Vec<&str> = Family::ALL.iter().map(|f| f.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 55);
    }

    #[test]
    fn every_family_name_round_trips_through_serde() {
        for &f in Family::ALL {
            let parsed: Family = serde_yaml_ng::from_str(f.as_str()).unwrap();
            assert_eq!(parsed, f, "{}", f.as_str());
        }
    }

    #[test]
    fn legacy_family_names_are_aliases() {
        let inovance: Family = serde_yaml_ng::from_str("inovance_easy").unwrap();
        assert_eq!(inovance, Family::InovanceEasyTcp);
        assert_eq!(
            serde_yaml_ng::from_str::<Family>("mitsubishi_fx3g").unwrap(),
            Family::MelsecFxSerialTcp
        );
        assert_eq!(
            serde_yaml_ng::from_str::<Family>("mitsubishi_fx3s").unwrap(),
            Family::MelsecFxLinksTcp
        );
    }

    #[test]
    fn opaque_families_are_the_structured_address_ones() {
        assert_eq!(Family::SiemensS7_1200.grammar(), Grammar::Opaque);
        assert_eq!(Family::AllenBradleyCip.grammar(), Grammar::Opaque);
        assert_eq!(Family::BeckhoffAds.grammar(), Grammar::Opaque);
        assert!(matches!(
            Family::MelsecA3CTcp.grammar(),
            Grammar::Classes(_)
        ));
        let opaque = Family::ALL
            .iter()
            .filter(|f| f.grammar() == Grammar::Opaque)
            .count();
        assert_eq!(opaque, 15);
    }
}
