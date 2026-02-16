//! 密码学工具模块
//!
//! 提供 Agent Token 哈希等安全相关工具函数。
//! 使用 HMAC-SHA256 替代裸 SHA-256，通过环境变量 TOKEN_HASH_SECRET 提供应用级密钥。
//!
//! 注意：此功能在 Phase 1 中全新引入，数据库中不存在旧 SHA-256 哈希数据，
//! 因此无需考虑旧哈希兼容性。

use std::sync::OnceLock;

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 缓存 TOKEN_HASH_SECRET 避免每次调用时读取环境变量
static TOKEN_SECRET: OnceLock<String> = OnceLock::new();

/// 获取 HMAC 密钥，首次调用时从环境变量读取并缓存
fn get_token_secret() -> &'static str {
    TOKEN_SECRET.get_or_init(|| {
        match std::env::var("TOKEN_HASH_SECRET") {
            Ok(secret) if !secret.is_empty() => secret,
            _ => {
                // release 模式下缺少密钥应直接 panic，防止使用公开默认值
                #[cfg(not(debug_assertions))]
                panic!("TOKEN_HASH_SECRET environment variable must be set in production");

                #[cfg(debug_assertions)]
                {
                    tracing::warn!(
                        "TOKEN_HASH_SECRET not set, using default key. Set this in production!"
                    );
                    "vk-default-dev-secret-change-in-production".to_string()
                }
            }
        }
    })
}

/// 使用 HMAC-SHA256 计算 Agent Token 的安全哈希值
///
/// 通过环境变量 `TOKEN_HASH_SECRET` 获取 HMAC 密钥（首次调用后缓存）。
/// - 开发环境（debug）：未设置时回退到内置默认密钥
/// - 生产环境（release）：未设置时 panic
///
/// # Arguments
/// * `token` - 待哈希的 Agent Token 明文
///
/// # Returns
/// 十六进制编码的 HMAC-SHA256 哈希字符串（64 字符）
pub fn hash_agent_token(token: &str) -> String {
    let secret = get_token_secret();
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key length is always valid");
    mac.update(token.as_bytes());
    let result = mac.finalize();
    format!("{:x}", result.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_agent_token_deterministic() {
        // 相同的 token 应产生相同的哈希
        let hash1 = hash_agent_token("vk_agent_test_token_123");
        let hash2 = hash_agent_token("vk_agent_test_token_123");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_agent_token_different_tokens() {
        // 不同的 token 应产生不同的哈希
        let hash1 = hash_agent_token("vk_agent_token_aaa");
        let hash2 = hash_agent_token("vk_agent_token_bbb");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_agent_token_hex_format() {
        let hash = hash_agent_token("test");
        // HMAC-SHA256 输出 32 字节 = 64 hex 字符
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
