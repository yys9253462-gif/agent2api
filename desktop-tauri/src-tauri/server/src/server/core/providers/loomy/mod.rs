//! Loomy（讯飞）的**账号面**（本仓库保留的部分）。
//!
//! ── 这份文件为什么这么小 ────────────────────────────────────
//! 本家的转发、积分、签到、新手任务、模型目录、短信登录的**执行体**都已搬到
//! **提供商插件**（`agent2api-providers` 的 `loomy/`），不随本仓库发布。
//! 留在这里的只有 `mask_phone` —— 它是纯本地字符串处理（给账号起默认名字用），
//! 不含任何上游协议知识。
//!
//! ── 硬约束 ──────────────────────────────────────────────────
//! release 是 `panic=abort`：本目录零 unwrap/expect/panic。

/// 手机号脱敏（`138****8000`）—— 账号默认名的展示用。
///
/// ── 为什么按字符而不是字节切 ────────────────────────────────
/// `phone` 来自 `POST /api/accounts` 的请求体（只 `trim` + 按字符数截断，
/// 不校验格式），所以「11 位数字 + 汉字」是合法输入。按字节回切会让切片
/// 落在汉字中间 → `panic: not a char boundary` → `panic = "abort"` 整进程退出。
pub fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() < 7 {
        return phone.to_string();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

#[cfg(test)]
mod tests {
    use super::mask_phone;

    /// 回归：非 ASCII 的 `phone` 不得 panic。
    ///
    /// 旧实现按字节回切 `&phone[phone.len() - 4..]`，第 16 字节落在 `你` 的
    /// 续字节（0xa0）上 → panic。这条测试钉住的是「按字符切」这件事。
    #[test]
    fn non_ascii_phone_does_not_panic() {
        assert_eq!(mask_phone("13800008000"), "138****8000");
        // 汉字在尾部：按字符切不会落到字符中间
        let masked = mask_phone("13800008000你");
        assert!(masked.starts_with("138****"));
        // 太短不处理
        assert_eq!(mask_phone("123456"), "123456");
    }
}
