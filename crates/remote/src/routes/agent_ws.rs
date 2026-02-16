//! Agent WebSocket 端点
//!
//! 处理 Agent Daemon 的 WebSocket 连接，包括：
//! - Agent 注册和认证
//! - 心跳处理
//! - 任务下发
//! - 事件接收

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::error::ErrorResponse;
use crate::AppState;
use crate::db::servers::{ServerRepository, ServerStatus};
use crate::services::agent_connection::AgentConnectionManager;
use utils::crypto::hash_agent_token;

/// Agent 连接状态
pub struct AgentConnection {
    pub server_id: Uuid,
    pub agent_id: Uuid,
    pub connected_at: chrono::DateTime<Utc>,
}

/// 全局 Agent 连接管理器
pub type AgentConnections = Arc<RwLock<HashMap<Uuid, AgentConnection>>>;

/// 创建 Agent WebSocket 路由
pub fn router() -> Router<AppState> {
    Router::new().route("/agent/ws", get(agent_ws_handler))
}

/// WebSocket 消息封装
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsMessage {
    id: String,
    #[serde(rename = "type")]
    msg_type: String,
    timestamp: chrono::DateTime<Utc>,
    payload: JsonValue,
}

impl WsMessage {
    fn new(msg_type: &str, payload: JsonValue) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            msg_type: msg_type.to_string(),
            timestamp: Utc::now(),
            payload,
        }
    }

    fn error(code: &str, message: &str) -> Self {
        Self::new(
            "error",
            serde_json::json!({
                "code": code,
                "message": message,
            }),
        )
    }
}

/// Agent 注册消息
#[derive(Debug, Deserialize)]
struct AgentRegisterPayload {
    /// Agent Token（保留用于协议兼容，实际认证已在 WebSocket 升级前完成）
    #[allow(dead_code)]
    agent_token: String,
    hostname: String,
    agent_version: String,
    system_info: JsonValue,
    available_executors: Vec<ExecutorInfo>,
}

/// 执行器信息
#[derive(Debug, Deserialize)]
struct ExecutorInfo {
    executor_type: String,
    version: Option<String>,
    capabilities: Vec<String>,
    #[allow(dead_code)]
    config_path: Option<String>,
}

/// 心跳消息
#[derive(Debug, Deserialize)]
struct HeartbeatPayload {
    system_stats: JsonValue,
    #[allow(dead_code)]
    running_executions: Vec<JsonValue>,
    #[allow(dead_code)]
    executor_statuses: Vec<JsonValue>,
}

/// WebSocket 连接查询参数（用于预认证）
#[derive(Debug, Deserialize)]
struct AgentWsParams {
    /// Agent Token，在 WebSocket 升级前验证
    token: String,
}

/// WebSocket 升级处理
///
/// 在升级 WebSocket 连接前通过 query parameter 验证 Agent Token，
/// 防止未认证的连接消耗服务器资源（DoS 防护）。
async fn agent_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<AgentWsParams>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 在 HTTP 升级前验证 token，拒绝无效连接
    let token_hash = hash_agent_token(&params.token);
    let server_repo = ServerRepository::new(&state.pool);

    match server_repo.find_by_agent_token_hash(&token_hash).await {
        Ok(Some(server)) => {
            debug!("WebSocket pre-auth succeeded for server_id={}, upgrading connection", server.id);
            Ok(ws
                .max_frame_size(1024 * 1024)        // 1MB per frame
                .max_message_size(4 * 1024 * 1024)   // 4MB per message
                .on_upgrade(move |socket| handle_agent_connection(socket, state, server.id)))
        }
        Ok(None) => {
            warn!("WebSocket pre-auth failed: invalid agent token");
            Err(ErrorResponse::new(StatusCode::UNAUTHORIZED, "Invalid agent token"))
        }
        Err(e) => {
            error!("WebSocket pre-auth database error: {:?}", e);
            Err(ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Authentication error"))
        }
    }
}

/// 处理 Agent 连接
///
/// server_id 已在 WebSocket 升级前通过预认证确定。
/// 此处仍需等待 Agent 发送注册消息以获取系统信息和执行器列表。
async fn handle_agent_connection(socket: WebSocket, state: AppState, server_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    // 等待注册消息
    let register_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        wait_for_register(&mut receiver),
    )
    .await;

    let register_payload = match register_result {
        Ok(Ok(payload)) => payload,
        Ok(Err(e)) => {
            error!("Failed to receive register message: {:?}", e);
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&WsMessage::error("register_failed", &e.to_string()))
                        .unwrap_or_default()
                        .into(),
                ))
                .await;
            return;
        }
        Err(_) => {
            error!("Timeout waiting for register message");
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&WsMessage::error("timeout", "Register timeout"))
                        .unwrap_or_default()
                        .into(),
                ))
                .await;
            return;
        }
    };

    let server_repo = ServerRepository::new(&state.pool);
    let agent_id = Uuid::new_v4();

    info!(
        "Agent registered: server_id={}, agent_id={}, hostname={}",
        server_id, agent_id, register_payload.hostname
    );

    // 注册连接
    let conn_manager = state.agent_connection_manager();
    if let Err(e) = conn_manager.register_connection(server_id, agent_id).await {
        error!("Failed to register connection: {:?}", e);
    }

    // 更新服务器状态
    if let Err(e) = server_repo
        .update_server_status(
            server_id,
            ServerStatus::Online,
            Some(agent_id),
            Some(&register_payload.agent_version),
            None,
        )
        .await
    {
        error!("Failed to update server status: {:?}", e);
    }

    // 更新系统信息
    if let Err(e) = server_repo
        .update_system_info(server_id, &register_payload.system_info)
        .await
    {
        error!("Failed to update system info: {:?}", e);
    }

    // 注册执行器
    for executor in &register_payload.available_executors {
        if let Err(e) = server_repo
            .upsert_executor(
                server_id,
                &executor.executor_type,
                executor.version.as_deref(),
                &executor.capabilities,
            )
            .await
        {
            error!("Failed to register executor: {:?}", e);
        }
    }

    // 发送注册确认
    let ack = WsMessage::new(
        "agent_register_ack",
        serde_json::json!({
            "agent_id": agent_id,
            "server_id": server_id,
            "heartbeat_interval_ms": 30000,
            "pending_configs": [],
        }),
    );

    if let Err(e) = sender
        .send(Message::Text(serde_json::to_string(&ack).unwrap_or_default().into()))
        .await
    {
        error!("Failed to send register ack: {:?}", e);
        return;
    }

    // 主消息循环
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_agent_message(&text, server_id, &state, &mut sender).await {
                            error!("Failed to handle message: {:?}", e);
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            error!("Failed to send pong: {:?}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Agent disconnected: server_id={}", server_id);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {:?}", e);
                        break;
                    }
                    None => {
                        info!("Agent connection closed: server_id={}", server_id);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 从连接管理器中移除
    let conn_manager = state.agent_connection_manager();
    conn_manager.remove_connection(server_id).await;

    // 更新服务器状态为离线
    if let Err(e) = server_repo
        .update_server_status(server_id, ServerStatus::Offline, None, None, None)
        .await
    {
        error!("Failed to update server status to offline: {:?}", e);
    }

    info!("Agent connection ended: server_id={}", server_id);
}

/// 等待注册消息
async fn wait_for_register<S>(receiver: &mut S) -> anyhow::Result<AgentRegisterPayload>
where
    S: StreamExt<Item = Result<Message, axum::Error>> + Unpin,
{
    while let Some(msg) = receiver.next().await {
        match msg? {
            Message::Text(text) => {
                let ws_msg: WsMessage = serde_json::from_str(&text)?;
                if ws_msg.msg_type == "agent_register" {
                    let payload: AgentRegisterPayload = serde_json::from_value(ws_msg.payload)?;
                    return Ok(payload);
                }
            }
            Message::Ping(_) => continue,
            _ => continue,
        }
    }
    Err(anyhow::anyhow!("Connection closed before register"))
}

/// 处理 Agent 消息
async fn handle_agent_message<S>(
    text: &str,
    server_id: Uuid,
    state: &AppState,
    _sender: &mut S,
) -> anyhow::Result<()>
where
    S: SinkExt<Message> + Unpin,
{
    let msg: WsMessage = serde_json::from_str(text)?;

    match msg.msg_type.as_str() {
        "heartbeat" => {
            let payload: HeartbeatPayload = serde_json::from_value(msg.payload)?;
            let server_repo = ServerRepository::new(&state.pool);
            
            // 更新数据库中的心跳
            server_repo
                .update_heartbeat(server_id, &payload.system_stats)
                .await?;
            
            // 更新连接管理器中的心跳时间
            let conn_manager = state.agent_connection_manager();
            if let Err(e) = conn_manager.update_heartbeat(server_id).await {
                warn!("Failed to update heartbeat in connection manager: {:?}", e);
            }
            
            debug!("Heartbeat received from server_id={}", server_id);
        }
        "execution_status" => {
            debug!("Execution status received: {:?}", msg.payload);
            // TODO: 处理执行状态更新
        }
        "tool_use" => {
            debug!("Tool use event received: {:?}", msg.payload);
            // TODO: 存储工具使用事件
        }
        "skill_invoke" => {
            debug!("Skill invoke event received: {:?}", msg.payload);
            // TODO: 存储技能调用事件
        }
        "log_stream" => {
            // TODO: 转发日志流
        }
        "approval_request" => {
            debug!("Approval request received: {:?}", msg.payload);
            // TODO: 处理审批请求
        }
        "config_ack" => {
            debug!("Config ack received: {:?}", msg.payload);
            // TODO: 更新配置部署状态
        }
        _ => {
            warn!("Unknown message type: {}", msg.msg_type);
        }
    }

    Ok(())
}
