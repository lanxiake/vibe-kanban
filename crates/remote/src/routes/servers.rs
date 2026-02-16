//! 服务器管理 API 路由
//!
//! 提供服务器的 CRUD 操作和 Agent Token 管理

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use utils::api::servers::{
    CreateServerRequest, CreateServerResponse, GetServerResponse, ListServersResponse,
    RegenerateTokenResponse, UpdateServerRequest,
};
use utils::crypto::hash_agent_token;
use uuid::Uuid;

use super::error::ErrorResponse;
use crate::{
    AppState,
    auth::RequestContext,
    db::{
        identity_errors::IdentityError,
        organization_members::{assert_admin, assert_membership},
        servers::ServerRepository,
    },
};

/// 创建服务器路由
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/organizations/{org_id}/servers",
            post(create_server).get(list_servers),
        )
        .route(
            "/organizations/{org_id}/servers/{server_id}",
            get(get_server).put(update_server).delete(delete_server),
        )
        .route(
            "/organizations/{org_id}/servers/{server_id}/regenerate-token",
            post(regenerate_token),
        )
}

/// 生成 Agent Token
fn generate_agent_token() -> String {
    // 生成 32 字节的随机 token，编码为 base64
    use rand::Rng;
    let mut rng = rand::rng();
    let token_bytes: [u8; 32] = rng.random();
    format!("vk_agent_{}", base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        token_bytes,
    ))
}

/// 生成 Docker 启动命令
///
/// 使用单引号包裹参数值防止 shell 注入
fn generate_docker_command(center_url: &str, agent_token: &str) -> String {
    let safe_url = center_url.replace('\'', "'\\''");
    let safe_token = agent_token.replace('\'', "'\\''");
    format!(
        r#"docker run -d \
  --name vibe-agent \
  --restart unless-stopped \
  -e CENTER_URL='{}' \
  -e AGENT_TOKEN='{}' \
  -v /path/to/repos:/app/workspaces \
  -v /path/to/config:/app/config \
  vibe-agent-daemon:latest"#,
        safe_url, safe_token
    )
}

/// 创建服务器
///
/// POST /organizations/{org_id}/servers
async fn create_server(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path(org_id): Path<Uuid>,
    Json(payload): Json<CreateServerRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 验证用户是否为组织管理员
    assert_admin(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            IdentityError::PermissionDenied => {
                ErrorResponse::new(StatusCode::FORBIDDEN, "Admin access required")
            }
            _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        })?;

    // 验证请求参数
    let name = payload.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Server name must be between 1 and 255 characters",
        ));
    }

    let host = payload.host.trim();
    if host.is_empty() {
        return Err(ErrorResponse::new(
            StatusCode::BAD_REQUEST,
            "Server host is required",
        ));
    }

    // 生成 Agent Token
    let agent_token = generate_agent_token();
    let token_hash = hash_agent_token(&agent_token);

    // 创建服务器
    let server_repo = ServerRepository::new(&state.pool);
    let server = server_repo
        .create_server(org_id, &payload, &token_hash)
        .await
        .map_err(|e| match e {
            IdentityError::OrganizationConflict(msg) => {
                ErrorResponse::new(StatusCode::CONFLICT, msg)
            }
            _ => {
                tracing::error!("Failed to create server: {:?}", e);
                ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create server")
            }
        })?;

    // 生成 Docker 命令
    let center_url = std::env::var("CENTER_WS_URL")
        .unwrap_or_else(|_| "wss://your-center.com/v1/agent/ws".to_string());
    let docker_command = generate_docker_command(&center_url, &agent_token);

    // 记录分析事件
    if let Some(analytics) = state.analytics() {
        analytics.track(
            ctx.user.id,
            "server_created",
            serde_json::json!({
                "server_id": server.id,
                "organization_id": org_id,
            }),
        );
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateServerResponse {
            server,
            agent_token,
            docker_command,
        }),
    ))
}

/// 获取服务器列表
///
/// GET /organizations/{org_id}/servers
async fn list_servers(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 验证用户是否为组织成员
    assert_membership(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            _ => ErrorResponse::new(StatusCode::FORBIDDEN, "Access denied"),
        })?;

    let server_repo = ServerRepository::new(&state.pool);
    let servers = server_repo.list_servers(org_id).await.map_err(|e| {
        tracing::error!("Failed to list servers: {:?}", e);
        ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to list servers")
    })?;

    Ok(Json(ListServersResponse {
        total: servers.len() as i64,
        servers,
    }))
}

/// 获取服务器详情
///
/// GET /organizations/{org_id}/servers/{server_id}
async fn get_server(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path((org_id, server_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 验证用户是否为组织成员
    assert_membership(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            _ => ErrorResponse::new(StatusCode::FORBIDDEN, "Access denied"),
        })?;

    let server_repo = ServerRepository::new(&state.pool);
    let server = server_repo
        .get_server_with_executors(server_id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Server not found")
            }
            _ => {
                tracing::error!("Failed to get server: {:?}", e);
                ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to get server")
            }
        })?;

    // 验证服务器属于该组织
    if server.server.organization_id != org_id {
        return Err(ErrorResponse::new(
            StatusCode::NOT_FOUND,
            "Server not found",
        ));
    }

    Ok(Json(GetServerResponse { server }))
}

/// 更新服务器
///
/// PUT /organizations/{org_id}/servers/{server_id}
async fn update_server(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path((org_id, server_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateServerRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 验证用户是否为组织管理员
    assert_admin(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            IdentityError::PermissionDenied => {
                ErrorResponse::new(StatusCode::FORBIDDEN, "Admin access required")
            }
            _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        })?;

    // 验证请求参数
    if let Some(ref name) = payload.name {
        let name = name.trim();
        if name.is_empty() || name.len() > 255 {
            return Err(ErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "Server name must be between 1 and 255 characters",
            ));
        }
    }

    let server_repo = ServerRepository::new(&state.pool);

    // 验证服务器属于该组织
    let existing = server_repo.get_server(server_id).await.map_err(|e| match e {
        IdentityError::NotFound => ErrorResponse::new(StatusCode::NOT_FOUND, "Server not found"),
        _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    })?;

    if existing.organization_id != org_id {
        return Err(ErrorResponse::new(
            StatusCode::NOT_FOUND,
            "Server not found",
        ));
    }

    let server = server_repo
        .update_server(server_id, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update server: {:?}", e);
            ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to update server")
        })?;

    Ok(Json(server))
}

/// 删除服务器
///
/// DELETE /organizations/{org_id}/servers/{server_id}
async fn delete_server(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path((org_id, server_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 验证用户是否为组织管理员
    assert_admin(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            IdentityError::PermissionDenied => {
                ErrorResponse::new(StatusCode::FORBIDDEN, "Admin access required")
            }
            _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        })?;

    let server_repo = ServerRepository::new(&state.pool);

    // 验证服务器属于该组织
    let existing = server_repo.get_server(server_id).await.map_err(|e| match e {
        IdentityError::NotFound => ErrorResponse::new(StatusCode::NOT_FOUND, "Server not found"),
        _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    })?;

    if existing.organization_id != org_id {
        return Err(ErrorResponse::new(
            StatusCode::NOT_FOUND,
            "Server not found",
        ));
    }

    server_repo.delete_server(server_id).await.map_err(|e| {
        tracing::error!("Failed to delete server: {:?}", e);
        ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete server")
    })?;

    // 记录分析事件
    if let Some(analytics) = state.analytics() {
        analytics.track(
            ctx.user.id,
            "server_deleted",
            serde_json::json!({
                "server_id": server_id,
                "organization_id": org_id,
            }),
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

/// 重新生成 Agent Token
///
/// POST /organizations/{org_id}/servers/{server_id}/regenerate-token
async fn regenerate_token(
    State(state): State<AppState>,
    axum::extract::Extension(ctx): axum::extract::Extension<RequestContext>,
    Path((org_id, server_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ErrorResponse> {
    // 先验证管理员权限
    assert_admin(&state.pool, org_id, ctx.user.id)
        .await
        .map_err(|e| match e {
            IdentityError::NotFound => {
                ErrorResponse::new(StatusCode::NOT_FOUND, "Organization not found")
            }
            IdentityError::PermissionDenied => {
                ErrorResponse::new(StatusCode::FORBIDDEN, "Admin access required")
            }
            _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        })?;

    let server_repo = ServerRepository::new(&state.pool);

    // 验证服务器归属
    let server = server_repo.get_server(server_id).await.map_err(|e| match e {
        IdentityError::NotFound => ErrorResponse::new(StatusCode::NOT_FOUND, "Server not found"),
        _ => ErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    })?;

    if server.organization_id != org_id {
        return Err(ErrorResponse::new(
            StatusCode::NOT_FOUND,
            "Server not found",
        ));
    }

    // 生成新的 Agent Token
    let agent_token = generate_agent_token();
    let token_hash = hash_agent_token(&agent_token);

    // 更新 Token Hash
    server_repo
        .update_agent_token_hash(server_id, &token_hash)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update agent token: {:?}", e);
            ErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to regenerate token",
            )
        })?;

    // 生成 Docker 命令
    let center_url = std::env::var("CENTER_WS_URL")
        .unwrap_or_else(|_| "wss://your-center.com/v1/agent/ws".to_string());
    let docker_command = generate_docker_command(&center_url, &agent_token);

    // 记录分析事件
    if let Some(analytics) = state.analytics() {
        analytics.track(
            ctx.user.id,
            "server_token_regenerated",
            serde_json::json!({
                "server_id": server_id,
            }),
        );
    }

    Ok(Json(RegenerateTokenResponse {
        agent_token,
        docker_command,
    }))
}
