//! Vibe Kanban Agent Daemon
//!
//! 远程服务器上的 AI 工具管理守护进程，负责：
//! - 与中心服务器保持 WebSocket 连接
//! - 上报系统状态和心跳
//! - 接收并执行任务
//! - 采集和上报执行事件
//! - 接收和应用配置

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod connection;
mod protocol;
mod system_monitor;

/// Agent Daemon 命令行参数
#[derive(Parser, Debug)]
#[command(name = "vibe-agent")]
#[command(about = "Vibe Kanban Agent Daemon - AI 工具管理守护进程")]
#[command(version)]
struct Args {
    /// 中心服务器 WebSocket URL
    #[arg(long, env = "CENTER_URL")]
    center_url: String,

    /// Agent Token（用于认证）
    #[arg(long, env = "AGENT_TOKEN")]
    agent_token: String,

    /// 日志级别
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// 心跳间隔（秒）
    #[arg(long, env = "HEARTBEAT_INTERVAL", default_value = "30")]
    heartbeat_interval: u64,

    /// 工作区目录
    #[arg(long, env = "WORKSPACE_DIR", default_value = "/app/workspaces")]
    workspace_dir: String,

    /// 配置目录
    #[arg(long, env = "CONFIG_DIR", default_value = "/app/config")]
    config_dir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化日志
    let log_level = args.log_level.parse::<Level>().unwrap_or(Level::INFO);
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive(format!("agent_daemon={}", log_level).parse()?)
                .add_directive(format!("vibe_agent={}", log_level).parse()?),
        )
        .init();

    info!("Vibe Kanban Agent Daemon v{}", env!("CARGO_PKG_VERSION"));
    info!("Center URL: {}", args.center_url);
    info!("Workspace directory: {}", args.workspace_dir);
    info!("Config directory: {}", args.config_dir);

    // 创建系统监控器
    let system_monitor = system_monitor::SystemMonitor::new();
    let system_info = system_monitor.get_system_info();
    info!("System: {} {} ({} cores, {:.1} GB RAM)",
        system_info.os,
        system_info.arch,
        system_info.cpu_cores,
        system_info.total_memory_gb
    );

    // 创建连接管理器并启动
    let connection_manager = connection::ConnectionManager::new(
        args.center_url,
        args.agent_token,
        args.heartbeat_interval,
        system_monitor,
    );

    // 运行连接管理器（阻塞，内部处理重连）
    connection_manager.run().await?;

    Ok(())
}
