//! WebSocket 消息协议定义
//!
//! 定义 Agent 与 Center 之间的所有消息类型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// WebSocket 消息封装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// 消息 ID，用于请求-响应匹配
    pub id: String,
    /// 消息类型
    #[serde(rename = "type")]
    pub msg_type: String,
    /// ISO 8601 时间戳
    pub timestamp: DateTime<Utc>,
    /// 具体消息内容
    pub payload: JsonValue,
}

impl WsMessage {
    /// 创建新消息
    pub fn new(msg_type: &str, payload: JsonValue) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: msg_type.to_string(),
            timestamp: Utc::now(),
            payload,
        }
    }
}

// ===== Agent → Center (上行消息) =====

/// Agent 注册消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterPayload {
    /// Agent Token
    pub agent_token: String,
    /// 主机名
    pub hostname: String,
    /// Agent 版本
    pub agent_version: String,
    /// 系统信息
    pub system_info: SystemInfo,
    /// 可用的执行器列表
    pub available_executors: Vec<ExecutorInfo>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// 操作系统
    pub os: String,
    /// CPU 架构
    pub arch: String,
    /// CPU 核心数
    pub cpu_cores: i32,
    /// 总内存（GB）
    pub total_memory_gb: f64,
    /// 总磁盘空间（GB）
    pub disk_total_gb: f64,
    /// 主机名
    pub hostname: String,
}

/// 执行器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorInfo {
    /// 执行器类型
    pub executor_type: String,
    /// 版本
    pub version: Option<String>,
    /// 能力列表
    pub capabilities: Vec<String>,
    /// 配置文件路径
    pub config_path: Option<String>,
}

/// 注册响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterAckPayload {
    /// 分配的 Agent ID
    pub agent_id: Uuid,
    /// 关联的服务器 ID
    pub server_id: Uuid,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 待应用的配置
    pub pending_configs: Vec<JsonValue>,
}

/// 心跳消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// 系统统计信息
    pub system_stats: SystemStats,
    /// 运行中的执行
    pub running_executions: Vec<RunningExecution>,
    /// 执行器状态
    pub executor_statuses: Vec<ExecutorStatusInfo>,
}

/// 系统统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// CPU 使用率
    pub cpu_usage_percent: f64,
    /// 内存使用率
    pub memory_usage_percent: f64,
    /// 可用内存（GB）
    pub memory_available_gb: f64,
    /// 磁盘使用率
    pub disk_usage_percent: f64,
    /// 可用磁盘空间（GB）
    pub disk_available_gb: f64,
    /// 负载平均值
    pub load_average: [f64; 3],
}

/// 运行中的执行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningExecution {
    /// 执行 ID
    pub execution_id: Uuid,
    /// 任务 ID
    pub task_id: Uuid,
    /// 执行器类型
    pub executor_type: String,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 状态
    pub status: String,
}

/// 执行器状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStatusInfo {
    /// 执行器类型
    pub executor_type: String,
    /// 状态
    pub status: String,
    /// 错误信息
    pub error_message: Option<String>,
}

// ===== Center → Agent (下行消息) =====

/// 任务分配消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignPayload {
    /// 分配 ID
    pub assignment_id: Uuid,
    /// 任务 ID
    pub task_id: Uuid,
    /// 任务标题
    pub task_title: String,
    /// 任务描述
    pub task_description: Option<String>,
    /// 执行器类型
    pub executor_type: String,
    /// 提示词
    pub prompt: String,
    /// 工作区配置
    pub workspace: WorkspaceConfig,
    /// 执行选项
    pub options: ExecutionOptions,
}

/// 工作区配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// 仓库列表
    pub repos: Vec<RepoConfig>,
    /// 环境变量
    pub env_vars: Option<std::collections::HashMap<String, String>>,
}

/// 仓库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    /// 仓库 URL
    pub url: String,
    /// 分支
    pub branch: String,
    /// 目标分支
    pub target_branch: Option<String>,
    /// 工作目录
    pub working_dir: Option<String>,
}

/// 执行选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// 自动提交
    pub auto_commit: bool,
    /// 运行设置脚本
    pub run_setup_scripts: bool,
    /// 运行清理脚本
    pub run_cleanup_scripts: bool,
    /// 超时时间（分钟）
    pub timeout_minutes: Option<u32>,
}

/// 任务取消消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelPayload {
    /// 任务 ID
    pub task_id: Uuid,
    /// 执行 ID
    pub execution_id: Option<Uuid>,
    /// 取消原因
    pub reason: Option<String>,
    /// 是否强制终止
    pub force: bool,
}

/// 配置下发消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPushPayload {
    /// 部署 ID
    pub deployment_id: Uuid,
    /// 配置类型
    pub config_type: String,
    /// 配置名称
    pub config_name: String,
    /// 配置版本
    pub config_version: i32,
    /// 配置内容
    pub content: JsonValue,
    /// 应用模式
    pub apply_mode: String,
    /// 目标执行器
    pub target_executors: Option<Vec<String>>,
}

/// 审批响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponsePayload {
    /// 请求 ID
    pub request_id: Uuid,
    /// 是否批准
    pub approved: bool,
    /// 消息
    pub message: Option<String>,
    /// 响应者
    pub responded_by: Option<String>,
    /// 响应时间
    pub responded_at: DateTime<Utc>,
}

/// 管理命令消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPayload {
    /// 命令 ID
    pub command_id: Uuid,
    /// 命令类型
    pub command_type: String,
    /// 参数
    pub params: Option<JsonValue>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u32>,
}
