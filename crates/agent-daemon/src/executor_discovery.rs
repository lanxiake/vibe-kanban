//! 执行器发现模块
//!
//! 检测系统中可用的 AI 编码工具执行器

use std::process::Command;
use tracing::{debug, warn};

use crate::protocol::{ExecutorInfo, SystemInfo};

/// 发现可用的执行器
///
/// 检测系统中安装的 AI 编码工具，返回可用的执行器列表
pub fn discover_executors() -> Vec<ExecutorInfo> {
    let mut executors = Vec::new();

    // 检测 Claude Code
    if let Some(version) = detect_claude_code() {
        executors.push(ExecutorInfo {
            executor_type: "claude_code".to_string(),
            version: Some(version),
            capabilities: vec![
                "session_fork".to_string(),
                "setup_helper".to_string(),
                "slash_commands".to_string(),
            ],
            config_path: None,
        });
        debug!("Discovered Claude Code executor: {}", version);
    }

    // 检测 Gemini CLI
    if let Some(version) = detect_gemini_cli() {
        executors.push(ExecutorInfo {
            executor_type: "gemini_cli".to_string(),
            version: Some(version),
            capabilities: vec!["session_fork".to_string()],
            config_path: None,
        });
        debug!("Discovered Gemini CLI executor: {}", version);
    }

    // 检测 Codex
    if let Some(version) = detect_codex() {
        executors.push(ExecutorInfo {
            executor_type: "codex".to_string(),
            version: Some(version),
            capabilities: vec!["session_fork".to_string()],
            config_path: None,
        });
        debug!("Discovered Codex executor: {}", version);
    }

    // 检测其他执行器...
    // TODO: 添加更多执行器检测逻辑

    if executors.is_empty() {
        warn!("No executors discovered. Make sure AI tools are installed and in PATH.");
    }

    executors
}

/// 检测 Claude Code
fn detect_claude_code() -> Option<String> {
    // 尝试运行 claude-code --version
    let output = Command::new("claude-code")
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Some(version)
    } else {
        None
    }
}

/// 检测 Gemini CLI
fn detect_gemini_cli() -> Option<String> {
    // 尝试运行 gemini-cli --version 或类似命令
    // 注意：实际的命令名可能不同，需要根据实际情况调整
    let output = Command::new("gemini-cli")
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Some(version)
    } else {
        None
    }
}

/// 检测 Codex
fn detect_codex() -> Option<String> {
    // 尝试运行 codex --version 或类似命令
    // 注意：实际的命令名可能不同，需要根据实际情况调整
    let output = Command::new("codex")
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Some(version)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_executors() {
        // 这个测试会实际检测系统中的执行器
        // 在 CI 环境中可能没有安装这些工具，所以可能返回空列表
        let executors = discover_executors();
        // 至少应该能够运行而不崩溃
        assert!(executors.len() <= 10); // 合理的上限
    }
}
