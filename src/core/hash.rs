//! 哈希工具函数

/// 计算字符串的 SHA-256 十六进制摘要
pub fn sha256_hex(input: &str) -> String {
    use ring::digest::{SHA256, digest};

    let hash = digest(&SHA256, input.as_bytes());
    let mut out = String::with_capacity(hash.as_ref().len() * 2);
    for byte in hash.as_ref() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}
