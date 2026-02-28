//! CI platform HTTP routes — webhook, API, WebSocket.

pub mod api;
pub mod webhook;
pub mod websocket;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;

use erp_core::db::diesel_pool::DieselPool;

use crate::config::CiConfig;

/// Shared state for CI route handlers.
#[derive(Clone)]
pub struct CiRouterState {
    pub pool: Arc<DieselPool>,
    pub config: CiConfig,
}

/// Build the CI platform's Axum router (nested at `/ci`).
pub fn ci_router(state: CiRouterState) -> Router {
    Router::new()
        // Webhook
        .route("/webhook/github", post(webhook_handler))
        // Build API
        .route("/api/builds", get(list_builds_handler))
        .route("/api/builds/trigger", post(trigger_build_handler))
        .route("/api/builds/{build_id}", get(get_build))
        .route("/api/builds/latest", get(get_latest_build))
        // KPI API
        .route("/api/kpi/success_rate", get(kpi_success_rate))
        .route("/api/kpi/avg_duration", get(kpi_avg_duration))
        .route("/api/kpi/env_utilization", get(kpi_env_utilization))
        .route("/api/kpi/builds_by_status", get(kpi_builds_by_status))
        // Project API
        .route("/api/projects", get(list_projects))
        .with_state(state)
}

// ── Webhook ──

async fn webhook_handler(
    State(state): State<CiRouterState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    crate::metrics::webhook_received(
        headers
            .get("x-github-event")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
    );

    webhook::handle_webhook(&state.config, &state.pool, &headers, body).await
}

// ── Build API ──

async fn trigger_build_handler(
    State(state): State<CiRouterState>,
    Json(req): Json<api::TriggerRequest>,
) -> Result<(StatusCode, Json<api::TriggerResponse>), StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    api::trigger_build(&mut conn, req)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Trigger build error: {e}");
            StatusCode::BAD_REQUEST
        })
}

#[derive(serde::Deserialize)]
pub struct ListBuildsQuery {
    pub limit: Option<i64>,
}

async fn list_builds_handler(
    State(state): State<CiRouterState>,
    Query(query): Query<ListBuildsQuery>,
) -> Result<Json<Vec<api::BuildJson>>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    api::list_builds(&mut conn, query.limit.unwrap_or(20))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_build(
    State(state): State<CiRouterState>,
    Path(build_id): Path<i64>,
) -> Result<Json<api::BuildJson>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    api::get_build(&mut conn, build_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

#[derive(serde::Deserialize)]
pub struct LatestBuildQuery {
    pub branch: String,
    pub project_id: i64,
}

async fn get_latest_build(
    State(state): State<CiRouterState>,
    Query(query): Query<LatestBuildQuery>,
) -> Result<Json<api::BuildJson>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    api::get_latest_build(&mut conn, query.project_id, &query.branch)
        .await
        .map(Json)
        .map_err(|_| StatusCode::NOT_FOUND)
}

// ── KPI API ──

#[derive(serde::Deserialize)]
pub struct KpiQuery {
    pub days: Option<i32>,
}

async fn kpi_success_rate(
    State(state): State<CiRouterState>,
    Query(query): Query<KpiQuery>,
) -> Result<Json<crate::dashboard::kpi::BuildSuccessRate>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::dashboard::kpi::query_success_rate(&mut conn, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn kpi_avg_duration(
    State(state): State<CiRouterState>,
    Query(query): Query<KpiQuery>,
) -> Result<Json<crate::dashboard::kpi::AvgBuildDuration>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::dashboard::kpi::query_avg_duration(&mut conn, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn kpi_env_utilization(
    State(state): State<CiRouterState>,
) -> Result<Json<crate::dashboard::kpi::EnvironmentUtilization>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::dashboard::kpi::query_env_utilization(&mut conn)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn kpi_builds_by_status(
    State(state): State<CiRouterState>,
    Query(query): Query<KpiQuery>,
) -> Result<Json<Vec<crate::dashboard::kpi::BuildsByStatus>>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::dashboard::kpi::query_builds_by_status(&mut conn, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Project API ──

async fn list_projects(
    State(state): State<CiRouterState>,
) -> Result<Json<Vec<crate::models::project::CiProject>>, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::services::project_service::list_projects(&mut conn)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
