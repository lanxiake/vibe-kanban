//! Agent 连接管理服务
//!
//! 负责：
//! - 跟踪活跃的 Agent 连接
//! - 定期检测心跳超时并更新服务器状态
//! - 管理连接生命周期

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::servers::ServerRepository;

/// Agent 连接信息
#[derive(Debug, Clone)]
pub struct AgentConnection {
    /// 服务器 ID
    pub server_id: Uuid,
    /// Agent ID
    pub agent_id: Uuid,
    /// 连接时间
    pub connected_at: chrono::DateTime<Utc>,
    /// 最后心跳时间
    pub last_heartbeat_at: chrono::DateTime<Utc>,
}

/// Agent 连接管理器
pub struct AgentConnectionManager {
    /// 活跃连接（按 server_id 索引）
    connections: Arc<RwLock<HashMap<Uuid, AgentConnection>>>,
    /// 数据库连接池
    pool: PgPool,
    /// 心跳超时时间（秒）
    heartbeat_timeout_seconds: i64,
}

impl AgentConnectionManager {
    /// 创建新的连接管理器
    pub fn new(pool: PgPool, heartbeat_timeout_seconds: i64) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            pool,
            heartbeat_timeout_seconds,
        }
    }

    /// 注册新连接
    pub async fn register_connection(
        &self,
        server_id: Uuid,
        agent_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut connections = self.connections.write().await;
        let now = Utc::now();
        
        connections.insert(
            server_id,
            AgentConnection {
                server_id,
                agent_id,
                connected_at: now,
                last_heartbeat_at: now,
            },
        );

        info!(
            "Agent connection registered: server_id={}, agent_id={}",
            server_id, agent_id
        );

        Ok(())
    }

    /// 更新心跳时间
    pub async fn update_heartbeat(&self, server_id: Uuid) -> Result<(), sqlx::Error> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get_mut(&server_id) {
            conn.last_heartbeat_at = Utc::now();
            Ok(())
        } else {
            warn!("Heartbeat update for unknown server: {}", server_id);
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// 移除连接
    pub async fn remove_connection(&self, server_id: Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(&server_id);
        
        info!("Agent connection removed: server_id={}", server_id);
    }

    /// 获取连接信息
    pub async fn get_connection(&self, server_id: Uuid) -> Option<AgentConnection> {
        let connections = self.connections.read().await;
        connections.get(&server_id).cloned()
    }

    /// 启动心跳超时检测任务
    /// 
    /// 定期检查所有在线服务器的心跳状态，将超时的服务器标记为离线
    pub fn start_heartbeat_monitor(&self) -> tokio::task::JoinHandle<()> {
        let connections = Arc::clone(&self.connections);
        let pool = self.pool.clone();
        let timeout = self.heartbeat_timeout_seconds;
        let server_repo = ServerRepository::new(&pool);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // 每 30 秒检查一次

            loop {
                interval.tick().await;

                // 检查数据库中的心跳超时
                match server_repo.mark_offline_servers(timeout).await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Marked {} servers as offline due to heartbeat timeout", count);
                            
                            // 从内存中移除超时的连接
                            let mut conns = connections.write().await;
                            let now = Utc::now();
                            let timeout_duration = chrono::Duration::seconds(timeout);
                            
                            conns.retain(|server_id, conn| {
                                let should_remove = now - conn.last_heartbeat_at > timeout_duration;
                                if should_remove {
                                    warn!(
                                        "Removing stale connection: server_id={}, last_heartbeat={:?}",
                                        server_id, conn.last_heartbeat_at
                                    );
                                }
                                !should_remove
                            });
                        }
                    }
                    Err(e) => {
                        error!("Failed to check heartbeat timeout: {:?}", e);
                    }
                }
            }
        })
    }

    /// 获取所有活跃连接数
    pub async fn active_connections_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
}
