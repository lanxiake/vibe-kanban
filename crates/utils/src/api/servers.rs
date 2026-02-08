//! Server 和 Agent 相关的 API 类型定义
//!
//! 用于 AI 工具跨设备管理平台的服务器管理功能

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::Type;
use ts_rs::TS;
use uuid::Uuid;

/// 服务器状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, TS, Default)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "server_status", rename_all = "lowercase")]
#[ts(export)]
#[ts(use_ts_enum)]
pub enum ServerStatus {
    /// 等待 Agent 连接
    #[default]
    Pending,
    /// Agent 在线
    Online,
    /// Agent 离线
    Offline,
    /// 连接错误
    Error,
}

/// 执行器状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, TS, Default)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "executor_status", rename_all = "lowercase")]
#[ts(export)]
#[ts(use_ts_enum)]
pub enum ExecutorStatus {
    /// 状态未知
    #[default]
    Unknown,
    /// 可用
    Available,
    /// 忙碌中
    Busy,
    /// 错误
    Error,
}

/// 服务器实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS)]
#[ts(export)]
pub struct Server {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub host: String,
    pub port: i32,
    pub status: ServerStatus,
    pub agent_id: Option<Uuid>,
    pub agent_version: Option<String>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub connection_error: Option<String>,
    #[sqlx(json)]
    pub system_info: Option<JsonValue>,
    #[sqlx(json)]
    pub system_stats: Option<JsonValue>,
    pub ssh_enabled: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i32>,
    pub ssh_username: Option<String>,
    #[sqlx(json)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 服务器执行器实体
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS)]
#[ts(export)]
pub struct ServerExecutor {
    pub id: Uuid,
    pub server_id: Uuid,
    pub executor_type: String,
    pub executor_name: Option<String>,
    pub version: Option<String>,
    pub status: ExecutorStatus,
    pub current_task_id: Option<Uuid>,
    #[sqlx(json)]
    pub capabilities: Vec<String>,
    #[sqlx(json)]
    pub config: JsonValue,
    pub total_executions: i32,
    pub successful_executions: i32,
    pub failed_executions: i32,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 系统信息结构
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: i32,
    pub total_memory_gb: f64,
    pub disk_total_gb: f64,
    pub hostname: String,
}

/// 系统统计信息结构
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SystemStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_available_gb: f64,
    pub disk_usage_percent: f64,
    pub disk_available_gb: f64,
    pub running_tasks: i32,
}

/// 创建服务器请求
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub host: String,
    pub port: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub ssh_config: Option<SshConfig>,
}

/// SSH 配置
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SshConfig {
    pub enabled: bool,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub username: Option<String>,
    pub auth_type: SshAuthType,
    /// 密码或私钥（敏感信息，不会在响应中返回）
    #[serde(skip_serializing)]
    pub credentials: Option<String>,
}

/// SSH 认证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
#[ts(use_ts_enum)]
pub enum SshAuthType {
    Password,
    Key,
}

impl std::fmt::Display for SshAuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SshAuthType::Password => write!(f, "password"),
            SshAuthType::Key => write!(f, "key"),
        }
    }
}

/// 创建服务器响应
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateServerResponse {
    pub server: Server,
    /// Agent Token（仅在创建时返回，用于 Agent 连接认证）
    pub agent_token: String,
    /// 生成的 Docker 启动命令
    pub docker_command: String,
}

/// 更新服务器请求
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub ssh_config: Option<SshConfig>,
}

/// 服务器列表响应
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListServersResponse {
    pub servers: Vec<ServerWithExecutors>,
    pub total: i64,
}

/// 带执行器信息的服务器
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ServerWithExecutors {
    #[serde(flatten)]
    pub server: Server,
    pub executors: Vec<ServerExecutor>,
}

/// 服务器详情响应
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetServerResponse {
    pub server: ServerWithExecutors,
}

/// 重新生成 Token 响应
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegenerateTokenResponse {
    pub agent_token: String,
    pub docker_command: String,
}

/// 执行器信息（Agent 注册时上报）
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExecutorInfo {
    pub executor_type: String,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub config_path: Option<String>,
}
