//! WebSocket 连接管理模块
//!
//! 负责与中心服务器的 WebSocket 连接建立、维护和重连

use std::time::Duration;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::time::{interval, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::executor_discovery::discover_executors;
use crate::protocol::{
    AgentRegisterAckPayload, AgentRegisterPayload, ExecutorInfo, HeartbeatPayload, WsMessage,
};
use crate::system_monitor::SystemMonitor;

/// 重连配置
struct ReconnectConfig {
    /// 初始重连延迟（毫秒）
    initial_delay_ms: u64,
    /// 最大重连延迟（毫秒）
    max_delay_ms: u64,
    /// 退避乘数
    backoff_multiplier: f64,
    /// 抖动百分比
    jitter_percent: u8,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            jitter_percent: 20,
        }
    }
}

/// 连接管理器
pub struct ConnectionManager {
    center_url: String,
    agent_token: String,
    heartbeat_interval: u64,
    system_monitor: SystemMonitor,
    reconnect_config: ReconnectConfig,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(
        center_url: String,
        agent_token: String,
        heartbeat_interval: u64,
        system_monitor: SystemMonitor,
    ) -> Self {
        Self {
            center_url,
            agent_token,
            heartbeat_interval,
            system_monitor,
            reconnect_config: ReconnectConfig::default(),
        }
    }

    /// 运行连接管理器（阻塞，内部处理重连）
    pub async fn run(mut self) -> Result<()> {
        let mut retry_count = 0u32;
        let mut current_delay = self.reconnect_config.initial_delay_ms;

        loop {
            match self.connect_and_run().await {
                Ok(()) => {
                    info!("Connection closed gracefully");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    error!("Connection error (attempt {}): {:?}", retry_count, e);

                    // 计算重连延迟
                    let jitter = self.calculate_jitter(current_delay);
                    let delay_with_jitter = current_delay + jitter;

                    warn!(
                        "Reconnecting in {} ms (attempt {})",
                        delay_with_jitter, retry_count
                    );

                    tokio::time::sleep(Duration::from_millis(delay_with_jitter)).await;

                    // 指数退避
                    current_delay = ((current_delay as f64 * self.reconnect_config.backoff_multiplier)
                        as u64)
                        .min(self.reconnect_config.max_delay_ms);
                }
            }
        }

        Ok(())
    }

    /// 连接并运行
    async fn connect_and_run(&mut self) -> Result<()> {
        info!("Connecting to center: {}", self.center_url);

        // 建立 WebSocket 连接
        let (ws_stream, _) = connect_async(&self.center_url)
            .await
            .context("Failed to connect to center")?;

        info!("WebSocket connection established");

        let (mut write, mut read) = ws_stream.split();

        // 发送注册消息
        let register_payload = AgentRegisterPayload {
            agent_token: self.agent_token.clone(),
            hostname: self.system_monitor.get_system_info().hostname.clone(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            system_info: self.system_monitor.get_system_info(),
            available_executors: self.discover_executors(),
        };

        let register_msg = WsMessage::new(
            "agent_register",
            serde_json::to_value(&register_payload)?,
        );

        write
            .send(Message::Text(serde_json::to_string(&register_msg)?.into()))
            .await
            .context("Failed to send register message")?;

        debug!("Sent register message");

        // 等待注册响应
        let register_ack = timeout(Duration::from_secs(30), read.next())
            .await
            .context("Timeout waiting for register ack")?
            .ok_or_else(|| anyhow::anyhow!("Connection closed before register ack"))?
            .context("Failed to receive register ack")?;

        let ack_msg: WsMessage = match register_ack {
            Message::Text(text) => serde_json::from_str(&text)?,
            _ => return Err(anyhow::anyhow!("Unexpected message type")),
        };

        if ack_msg.msg_type != "agent_register_ack" {
            return Err(anyhow::anyhow!(
                "Unexpected message type: {}",
                ack_msg.msg_type
            ));
        }

        let ack_payload: AgentRegisterAckPayload = serde_json::from_value(ack_msg.payload)?;
        info!(
            "Registered successfully. Agent ID: {}, Server ID: {}",
            ack_payload.agent_id, ack_payload.server_id
        );

        // 启动心跳任务
        let heartbeat_interval = Duration::from_secs(self.heartbeat_interval);
        let mut heartbeat_timer = interval(heartbeat_interval);

        // 主循环
        loop {
            tokio::select! {
                // 心跳
                _ = heartbeat_timer.tick() => {
                    let stats = self.system_monitor.get_system_stats();
                    let heartbeat = HeartbeatPayload {
                        system_stats: stats,
                        running_executions: vec![],
                        executor_statuses: vec![],
                    };

                    let msg = WsMessage::new("heartbeat", serde_json::to_value(&heartbeat)?);
                    write
                        .send(Message::Text(serde_json::to_string(&msg)?.into()))
                        .await
                        .context("Failed to send heartbeat")?;

                    debug!("Sent heartbeat");
                }

                // 接收消息
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_message(&text, &mut write).await?;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Received close frame");
                            break;
                        }
                        Some(Err(e)) => {
                            return Err(anyhow::anyhow!("WebSocket error: {:?}", e));
                        }
                        None => {
                            info!("Connection closed");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// 处理接收到的消息
    async fn handle_message<S>(&mut self, text: &str, _write: &mut S) -> Result<()>
    where
        S: SinkExt<Message> + Unpin,
    {
        let msg: WsMessage = serde_json::from_str(text)?;

        match msg.msg_type.as_str() {
            "task_assign" => {
                info!("Received task assignment: {:?}", msg.payload);
                // TODO: 实现任务处理
            }
            "task_cancel" => {
                info!("Received task cancel: {:?}", msg.payload);
                // TODO: 实现任务取消
            }
            "config_push" => {
                info!("Received config push: {:?}", msg.payload);
                // TODO: 实现配置应用
            }
            "approval_response" => {
                info!("Received approval response: {:?}", msg.payload);
                // TODO: 实现审批响应处理
            }
            "command" => {
                info!("Received command: {:?}", msg.payload);
                // TODO: 实现命令处理
            }
            _ => {
                warn!("Unknown message type: {}", msg.msg_type);
            }
        }

        Ok(())
    }

    /// 发现可用的执行器
    fn discover_executors(&self) -> Vec<ExecutorInfo> {
        executor_discovery::discover_executors()
    }

    /// 计算抖动值
    fn calculate_jitter(&self, delay: u64) -> u64 {
        use rand::Rng;
        let jitter_range = (delay as f64 * self.reconnect_config.jitter_percent as f64 / 100.0) as u64;
        if jitter_range > 0 {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..jitter_range)
        } else {
            0
        }
    }
}
