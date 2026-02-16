use axum::{
    Json, Router,
    http::{Method, Request, header, header::HeaderName},
    middleware,
    routing::get,
};
use serde::Serialize;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    services::{ServeDir, ServeFile},
    trace::{DefaultOnFailure, TraceLayer},
};
use tracing::{Level, Span, field};

use crate::{AppState, auth::require_session};

#[cfg(feature = "vk-billing")]
mod billing;
#[cfg(not(feature = "vk-billing"))]
mod billing {
    use axum::Router;

    use crate::AppState;
    pub fn public_router() -> Router<AppState> {
        Router::new()
    }
    pub fn protected_router() -> Router<AppState> {
        Router::new()
    }
}
mod agent_ws;
mod electric_proxy;
pub(crate) mod error;
mod github_app;
mod identity;
mod issue_assignees;
mod issue_comment_reactions;
mod issue_comments;
mod issue_followers;
mod issue_relationships;
mod issue_tags;
mod issues;
mod migration;
mod notifications;
mod oauth;
pub(crate) mod organization_members;
mod organizations;
mod project_statuses;
mod projects;
mod pull_requests;
mod review;
mod servers;
mod tags;
mod tokens;
mod workspaces;

pub fn router(state: AppState) -> Router {
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let request_id = request
                .extensions()
                .get::<RequestId>()
                .and_then(|id| id.header_value().to_str().ok());
            let is_health = request.uri().path() == "/health";
            let span = if is_health {
                tracing::trace_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = field::Empty
                )
            } else {
                tracing::debug_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = field::Empty
                )
            };
            if let Some(request_id) = request_id {
                span.record("request_id", field::display(request_id));
            }
            span
        })
        .on_response(
            |response: &axum::http::Response<_>, latency: std::time::Duration, span: &Span| {
                if span.is_disabled() {
                    return;
                }
                let status = response.status().as_u16();
                let latency_ms = latency.as_millis();
                if status >= 500 {
                    tracing::error!(status, latency_ms, "server error");
                } else if status >= 400 {
                    tracing::warn!(status, latency_ms, "client error");
                } else {
                    tracing::debug!(status, latency_ms, "request completed");
                }
            },
        )
        .on_failure(DefaultOnFailure::new().level(Level::ERROR));

    let v1_public = Router::<AppState>::new()
        .route("/health", get(health))
        .merge(oauth::public_router())
        .merge(organization_members::public_router())
        .merge(tokens::public_router())
        .merge(review::public_router())
        .merge(github_app::public_router())
        .merge(billing::public_router())
        .merge(agent_ws::router());

    let v1_protected = Router::<AppState>::new()
        .merge(identity::router())
        .merge(projects::router())
        .merge(organizations::router())
        .merge(organization_members::protected_router())
        .merge(oauth::protected_router())
        .merge(electric_proxy::router())
        .merge(github_app::protected_router())
        .merge(project_statuses::router())
        .merge(tags::router())
        .merge(issue_comments::router())
        .merge(issue_comment_reactions::router())
        .merge(issues::router())
        .merge(issue_assignees::router())
        .merge(issue_followers::router())
        .merge(issue_tags::router())
        .merge(issue_relationships::router())
        .merge(pull_requests::router())
        .merge(notifications::router())
        .merge(workspaces::router())
        .merge(servers::router())
        .merge(billing::protected_router())
        .merge(migration::router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_session,
        ));

    let static_dir = "/srv/static";
    let spa =
        ServeDir::new(static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));

    Router::<AppState>::new()
        .nest("/v1", v1_public)
        .nest("/v1", v1_protected)
        .fallback_service(spa)
        .layer(middleware::from_fn(
            crate::middleware::version::add_version_headers,
        ))
        .layer({
            // CORS 配置：通过环境变量 CORS_ALLOWED_ORIGINS 设置允许的来源
            // 多个来源用逗号分隔，例如 "https://app.example.com,http://localhost:3000"
            let origins = if let Ok(origins_str) = std::env::var("CORS_ALLOWED_ORIGINS") {
                let parsed: Vec<_> = origins_str
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                tracing::info!("CORS allowed origins: {:?}", parsed);
                AllowOrigin::list(parsed)
            } else {
                // 开发环境回退：允许 localhost 来源
                tracing::warn!("CORS_ALLOWED_ORIGINS not set, allowing localhost origins for development");
                AllowOrigin::list([
                    "http://localhost:3000".parse().unwrap(),
                    "http://localhost:5173".parse().unwrap(),
                    "http://127.0.0.1:3000".parse().unwrap(),
                    "http://127.0.0.1:5173".parse().unwrap(),
                ])
            };

            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
                .allow_credentials(true)
        })
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid {},
        ))
        .with_state(state)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
