//! 服务器数据库操作
//!
//! 提供服务器和执行器的 CRUD 操作

use serde_json::Value as JsonValue;
use sqlx::{PgPool, query_as};
use uuid::Uuid;

pub use utils::api::servers::{
    CreateServerRequest, ExecutorStatus, Server, ServerExecutor, ServerStatus, ServerWithExecutors,
    SystemInfo, SystemStats, UpdateServerRequest,
};

use super::identity_errors::IdentityError;
use super::organization_members::assert_membership;

/// 服务器数据库操作仓库
pub struct ServerRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> ServerRepository<'a> {
    /// 创建新的 ServerRepository 实例
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// 验证用户是否有权限访问该组织
    pub async fn assert_membership(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), IdentityError> {
        assert_membership(self.pool, organization_id, user_id).await
    }

    /// 创建服务器
    pub async fn create_server(
        &self,
        organization_id: Uuid,
        request: &CreateServerRequest,
        agent_token_hash: &str,
    ) -> Result<Server, IdentityError> {
        let port = request.port.unwrap_or(9999);
        let tags = serde_json::to_value(request.tags.clone().unwrap_or_default())
            .unwrap_or(JsonValue::Array(vec![]));

        // 处理 SSH 配置
        let (ssh_enabled, ssh_host, ssh_port, ssh_username, ssh_auth_type) =
            if let Some(ref ssh) = request.ssh_config {
                (
                    ssh.enabled,
                    ssh.host.clone(),
                    ssh.port,
                    ssh.username.clone(),
                    ssh.auth_type.to_string(),
                )
            } else {
                (false, None, None, None, String::new())
            };

        let server = query_as!(
            Server,
            r#"
            INSERT INTO servers (
                organization_id, name, description, host, port,
                ssh_enabled, ssh_host, ssh_port, ssh_username, ssh_auth_type,
                tags, agent_token_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING
                id              AS "id!: Uuid",
                organization_id AS "organization_id!: Uuid",
                name            AS "name!",
                description,
                host            AS "host!",
                port            AS "port!",
                status          AS "status!: ServerStatus",
                agent_id        AS "agent_id: Uuid",
                agent_version,
                last_heartbeat_at,
                connection_error,
                system_info,
                system_stats,
                ssh_enabled     AS "ssh_enabled!",
                ssh_host,
                ssh_port,
                ssh_username,
                tags            AS "tags!",
                created_at      AS "created_at!",
                updated_at      AS "updated_at!"
            "#,
            organization_id,
            request.name,
            request.description,
            request.host,
            port,
            ssh_enabled,
            ssh_host,
            ssh_port,
            ssh_username,
            if ssh_auth_type.is_empty() {
                None
            } else {
                Some(ssh_auth_type)
            },
            tags,
            agent_token_hash
        )
        .fetch_one(self.pool)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error()
                && db_err.is_unique_violation()
            {
                return IdentityError::OrganizationConflict(
                    "A server with this name already exists in this organization".to_string(),
                );
            }
            IdentityError::from(e)
        })?;

        Ok(server)
    }

    /// 获取服务器列表
    pub async fn list_servers(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ServerWithExecutors>, IdentityError> {
        // 获取所有服务器
        let servers = query_as!(
            Server,
            r#"
            SELECT
                id              AS "id!: Uuid",
                organization_id AS "organization_id!: Uuid",
                name            AS "name!",
                description,
                host            AS "host!",
                port            AS "port!",
                status          AS "status!: ServerStatus",
                agent_id        AS "agent_id: Uuid",
                agent_version,
                last_heartbeat_at,
                connection_error,
                system_info,
                system_stats,
                ssh_enabled     AS "ssh_enabled!",
                ssh_host,
                ssh_port,
                ssh_username,
                tags            AS "tags!",
                created_at      AS "created_at!",
                updated_at      AS "updated_at!"
            FROM servers
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
            organization_id
        )
        .fetch_all(self.pool)
        .await?;

        // 获取每个服务器的执行器
        let mut result = Vec::with_capacity(servers.len());
        for server in servers {
            let executors = self.list_executors(server.id).await?;
            result.push(ServerWithExecutors {
                server,
                executors,
            });
        }

        Ok(result)
    }

    /// 获取单个服务器
    pub async fn get_server(&self, server_id: Uuid) -> Result<Server, IdentityError> {
        query_as!(
            Server,
            r#"
            SELECT
                id              AS "id!: Uuid",
                organization_id AS "organization_id!: Uuid",
                name            AS "name!",
                description,
                host            AS "host!",
                port            AS "port!",
                status          AS "status!: ServerStatus",
                agent_id        AS "agent_id: Uuid",
                agent_version,
                last_heartbeat_at,
                connection_error,
                system_info,
                system_stats,
                ssh_enabled     AS "ssh_enabled!",
                ssh_host,
                ssh_port,
                ssh_username,
                tags            AS "tags!",
                created_at      AS "created_at!",
                updated_at      AS "updated_at!"
            FROM servers
            WHERE id = $1
            "#,
            server_id
        )
        .fetch_optional(self.pool)
        .await?
        .ok_or(IdentityError::NotFound)
    }

    /// 获取带执行器的服务器详情
    pub async fn get_server_with_executors(
        &self,
        server_id: Uuid,
    ) -> Result<ServerWithExecutors, IdentityError> {
        let server = self.get_server(server_id).await?;
        let executors = self.list_executors(server_id).await?;
        Ok(ServerWithExecutors { server, executors })
    }

    /// 更新服务器
    pub async fn update_server(
        &self,
        server_id: Uuid,
        request: &UpdateServerRequest,
    ) -> Result<Server, IdentityError> {
        let existing = self.get_server(server_id).await?;

        let name = request.name.clone().unwrap_or(existing.name);
        let description = request.description.clone().or(existing.description);
        let host = request.host.clone().unwrap_or(existing.host);
        let port = request.port.unwrap_or(existing.port);
        let tags = request
            .tags
            .as_ref()
            .map(|t| serde_json::to_value(t).unwrap_or(JsonValue::Array(vec![])))
            .unwrap_or(existing.tags);

        let server = query_as!(
            Server,
            r#"
            UPDATE servers
            SET name = $2, description = $3, host = $4, port = $5, tags = $6
            WHERE id = $1
            RETURNING
                id              AS "id!: Uuid",
                organization_id AS "organization_id!: Uuid",
                name            AS "name!",
                description,
                host            AS "host!",
                port            AS "port!",
                status          AS "status!: ServerStatus",
                agent_id        AS "agent_id: Uuid",
                agent_version,
                last_heartbeat_at,
                connection_error,
                system_info,
                system_stats,
                ssh_enabled     AS "ssh_enabled!",
                ssh_host,
                ssh_port,
                ssh_username,
                tags            AS "tags!",
                created_at      AS "created_at!",
                updated_at      AS "updated_at!"
            "#,
            server_id,
            name,
            description,
            host,
            port,
            tags
        )
        .fetch_one(self.pool)
        .await?;

        Ok(server)
    }

    /// 删除服务器
    pub async fn delete_server(&self, server_id: Uuid) -> Result<(), IdentityError> {
        let result = sqlx::query!("DELETE FROM servers WHERE id = $1", server_id)
            .execute(self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(IdentityError::NotFound);
        }

        Ok(())
    }

    /// 更新 Agent Token Hash
    pub async fn update_agent_token_hash(
        &self,
        server_id: Uuid,
        token_hash: &str,
    ) -> Result<(), IdentityError> {
        sqlx::query!(
            "UPDATE servers SET agent_token_hash = $2 WHERE id = $1",
            server_id,
            token_hash
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 通过 Agent Token Hash 查找服务器
    pub async fn find_by_agent_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Server>, IdentityError> {
        let server = query_as!(
            Server,
            r#"
            SELECT
                id              AS "id!: Uuid",
                organization_id AS "organization_id!: Uuid",
                name            AS "name!",
                description,
                host            AS "host!",
                port            AS "port!",
                status          AS "status!: ServerStatus",
                agent_id        AS "agent_id: Uuid",
                agent_version,
                last_heartbeat_at,
                connection_error,
                system_info,
                system_stats,
                ssh_enabled     AS "ssh_enabled!",
                ssh_host,
                ssh_port,
                ssh_username,
                tags            AS "tags!",
                created_at      AS "created_at!",
                updated_at      AS "updated_at!"
            FROM servers
            WHERE agent_token_hash = $1
            "#,
            token_hash
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(server)
    }

    /// 更新服务器状态（Agent 连接/断开时调用）
    pub async fn update_server_status(
        &self,
        server_id: Uuid,
        status: ServerStatus,
        agent_id: Option<Uuid>,
        agent_version: Option<&str>,
        connection_error: Option<&str>,
    ) -> Result<(), IdentityError> {
        sqlx::query!(
            r#"
            UPDATE servers
            SET status = $2,
                agent_id = $3,
                agent_version = $4,
                connection_error = $5,
                last_heartbeat_at = CASE WHEN $2 = 'online' THEN NOW() ELSE last_heartbeat_at END
            WHERE id = $1
            "#,
            server_id,
            status as ServerStatus,
            agent_id,
            agent_version,
            connection_error
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 更新心跳和系统状态
    pub async fn update_heartbeat(
        &self,
        server_id: Uuid,
        system_stats: &JsonValue,
    ) -> Result<(), IdentityError> {
        sqlx::query!(
            r#"
            UPDATE servers
            SET last_heartbeat_at = NOW(),
                system_stats = $2,
                status = 'online'
            WHERE id = $1
            "#,
            server_id,
            system_stats
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 更新系统信息（Agent 注册时调用）
    pub async fn update_system_info(
        &self,
        server_id: Uuid,
        system_info: &JsonValue,
    ) -> Result<(), IdentityError> {
        sqlx::query!(
            "UPDATE servers SET system_info = $2 WHERE id = $1",
            server_id,
            system_info
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 获取服务器的执行器列表
    pub async fn list_executors(&self, server_id: Uuid) -> Result<Vec<ServerExecutor>, IdentityError> {
        let executors = query_as!(
            ServerExecutor,
            r#"
            SELECT
                id                    AS "id!: Uuid",
                server_id             AS "server_id!: Uuid",
                executor_type         AS "executor_type!",
                executor_name,
                version,
                status                AS "status!: ExecutorStatus",
                current_task_id       AS "current_task_id: Uuid",
                capabilities          AS "capabilities!",
                config                AS "config!",
                total_executions      AS "total_executions!",
                successful_executions AS "successful_executions!",
                failed_executions     AS "failed_executions!",
                last_used_at,
                created_at            AS "created_at!",
                updated_at            AS "updated_at!"
            FROM server_executors
            WHERE server_id = $1
            ORDER BY executor_type
            "#,
            server_id
        )
        .fetch_all(self.pool)
        .await?;

        Ok(executors)
    }

    /// 创建或更新执行器（Agent 注册时调用）
    pub async fn upsert_executor(
        &self,
        server_id: Uuid,
        executor_type: &str,
        version: Option<&str>,
        capabilities: &[String],
    ) -> Result<ServerExecutor, IdentityError> {
        let capabilities_json = serde_json::to_value(capabilities).unwrap_or(JsonValue::Array(vec![]));

        let executor = query_as!(
            ServerExecutor,
            r#"
            INSERT INTO server_executors (server_id, executor_type, version, capabilities, status)
            VALUES ($1, $2, $3, $4, 'available')
            ON CONFLICT (server_id, executor_type)
            DO UPDATE SET
                version = EXCLUDED.version,
                capabilities = EXCLUDED.capabilities,
                status = 'available',
                updated_at = NOW()
            RETURNING
                id                    AS "id!: Uuid",
                server_id             AS "server_id!: Uuid",
                executor_type         AS "executor_type!",
                executor_name,
                version,
                status                AS "status!: ExecutorStatus",
                current_task_id       AS "current_task_id: Uuid",
                capabilities          AS "capabilities!",
                config                AS "config!",
                total_executions      AS "total_executions!",
                successful_executions AS "successful_executions!",
                failed_executions     AS "failed_executions!",
                last_used_at,
                created_at            AS "created_at!",
                updated_at            AS "updated_at!"
            "#,
            server_id,
            executor_type,
            version,
            capabilities_json
        )
        .fetch_one(self.pool)
        .await?;

        Ok(executor)
    }

    /// 更新执行器状态
    pub async fn update_executor_status(
        &self,
        executor_id: Uuid,
        status: ExecutorStatus,
        current_task_id: Option<Uuid>,
    ) -> Result<(), IdentityError> {
        sqlx::query!(
            r#"
            UPDATE server_executors
            SET status = $2, current_task_id = $3
            WHERE id = $1
            "#,
            executor_id,
            status as ExecutorStatus,
            current_task_id
        )
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 增加执行器统计
    pub async fn increment_executor_stats(
        &self,
        executor_id: Uuid,
        success: bool,
    ) -> Result<(), IdentityError> {
        if success {
            sqlx::query!(
                r#"
                UPDATE server_executors
                SET total_executions = total_executions + 1,
                    successful_executions = successful_executions + 1,
                    last_used_at = NOW()
                WHERE id = $1
                "#,
                executor_id
            )
            .execute(self.pool)
            .await?;
        } else {
            sqlx::query!(
                r#"
                UPDATE server_executors
                SET total_executions = total_executions + 1,
                    failed_executions = failed_executions + 1,
                    last_used_at = NOW()
                WHERE id = $1
                "#,
                executor_id
            )
            .execute(self.pool)
            .await?;
        }

        Ok(())
    }

    /// 检查心跳超时的服务器并更新状态
    pub async fn mark_offline_servers(&self, timeout_seconds: i64) -> Result<i64, IdentityError> {
        let result = sqlx::query!(
            r#"
            UPDATE servers
            SET status = 'offline'
            WHERE status = 'online'
              AND last_heartbeat_at < NOW() - INTERVAL '1 second' * $1
            "#,
            timeout_seconds
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
