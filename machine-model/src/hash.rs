//! 內容雜湊（FNV-1a 64）。用於標準類型的版本釘選與解析結果的指紋；不是密碼學雜湊。

/// 去掉開頭 BOM、忽略所有 `\r`（Windows 的 CRLF 與 LF 得到相同結果）。
pub fn content_hash(text: &str) -> u64 {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes().filter(|b| *b != b'\r') {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// 16 位小寫十六進位。
pub fn hex(value: u64) -> String {
    format!("{value:016x}")
}
