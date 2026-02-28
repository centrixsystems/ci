//! GitHub webhook handler — receives push/PR events, creates builds.

use std::sync::Arc;

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};

use erp_core::db::diesel_pool::DieselPool;

use crate::config::CiConfig;
use crate::models::build::NewCiBuild;
use crate::services::{build_service, github_service, project_service};

/// Handle an incoming GitHub webhook payload.
pub async fn handle_webhook(
    config: &CiConfig,
    pool: &Arc<DieselPool>,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Validate signature
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !github_service::validate_signature(&config.github_webhook_secret, &body, signature) {
        tracing::warn!("Webhook signature validation failed");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse event type
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let payload: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    match event_type {
        "push" => handle_push(config, pool, &payload).await,
        "pull_request" => handle_pull_request(config, pool, &payload).await,
        "ping" => {
            tracing::info!("Received GitHub ping webhook");
            Ok(StatusCode::OK)
        }
        _ => {
            tracing::debug!("Ignoring webhook event: {}", event_type);
            Ok(StatusCode::OK)
        }
    }
}

async fn handle_push(
    config: &CiConfig,
    pool: &Arc<DieselPool>,
    payload: &serde_json::Value,
) -> Result<StatusCode, StatusCode> {
    let repo_full_name = payload["repository"]["full_name"]
        .as_str()
        .unwrap_or_default();
    let commit_sha = payload["after"].as_str().unwrap_or_default();
    let branch = payload["ref"]
        .as_str()
        .unwrap_or_default()
        .strip_prefix("refs/heads/")
        .unwrap_or_default();
    let author = payload["pusher"]["name"].as_str().unwrap_or_default();
    let message = payload["head_commit"]["message"]
        .as_str()
        .map(|s| s.to_string());

    if commit_sha.is_empty() || branch.is_empty() {
        return Ok(StatusCode::OK);
    }

    let mut conn = pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find project by repo
    let project = project_service::find_by_repo(&mut conn, repo_full_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let project = match project {
        Some(p) => p,
        None => {
            tracing::debug!("No project registered for repo: {}", repo_full_name);
            return Ok(StatusCode::OK);
        }
    };

    // Compute fingerprint for dedup
    let fingerprint = format!("{}-{}-push", commit_sha, branch);

    // Check throttle
    if build_service::is_duplicate(&mut conn, &fingerprint, config.throttle_window_secs)
        .await
        .unwrap_or(false)
    {
        tracing::info!("Duplicate build throttled: {}", fingerprint);
        return Ok(StatusCode::OK);
    }

    // Create build
    let new_build = NewCiBuild {
        tenant_id: project.tenant_id,
        project_id: project.id,
        commit_sha: commit_sha.to_string(),
        branch: branch.to_string(),
        pr_number: None,
        author: Some(author.to_string()),
        message,
        fingerprint,
        trigger_event: "push".to_string(),
        status: "pending".to_string(),
    };

    match build_service::create_build(&mut conn, new_build).await {
        Ok(build) => {
            tracing::info!(
                build_id = build.id,
                branch = branch,
                "Build created from push webhook"
            );

            // Post pending status to GitHub
            let _ = github_service::post_status(
                &config.github_token,
                repo_full_name,
                commit_sha,
                "pending",
                "Build queued",
                &format!("{}/ci/api/builds/{}", config.dashboard_url, build.id),
                "centrix-ci",
            )
            .await;

            Ok(StatusCode::CREATED)
        }
        Err(e) => {
            tracing::error!("Failed to create build: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_pull_request(
    config: &CiConfig,
    pool: &Arc<DieselPool>,
    payload: &serde_json::Value,
) -> Result<StatusCode, StatusCode> {
    let action = payload["action"].as_str().unwrap_or_default();
    if action != "opened" && action != "synchronize" && action != "reopened" {
        return Ok(StatusCode::OK);
    }

    let repo_full_name = payload["repository"]["full_name"]
        .as_str()
        .unwrap_or_default();
    let pr_number = payload["number"].as_i64().unwrap_or(0) as i32;
    let commit_sha = payload["pull_request"]["head"]["sha"]
        .as_str()
        .unwrap_or_default();
    let branch = payload["pull_request"]["head"]["ref"]
        .as_str()
        .unwrap_or_default();
    let author = payload["pull_request"]["user"]["login"]
        .as_str()
        .unwrap_or_default();

    if commit_sha.is_empty() || branch.is_empty() {
        return Ok(StatusCode::OK);
    }

    let mut conn = pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let project = project_service::find_by_repo(&mut conn, repo_full_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let project = match project {
        Some(p) => p,
        None => return Ok(StatusCode::OK),
    };

    let fingerprint = format!("{}-{}-pr{}", commit_sha, branch, pr_number);

    if build_service::is_duplicate(&mut conn, &fingerprint, config.throttle_window_secs)
        .await
        .unwrap_or(false)
    {
        return Ok(StatusCode::OK);
    }

    let new_build = NewCiBuild {
        tenant_id: project.tenant_id,
        project_id: project.id,
        commit_sha: commit_sha.to_string(),
        branch: branch.to_string(),
        pr_number: Some(pr_number),
        author: Some(author.to_string()),
        message: None,
        fingerprint,
        trigger_event: "pull_request".to_string(),
        status: "pending".to_string(),
    };

    match build_service::create_build(&mut conn, new_build).await {
        Ok(build) => {
            let _ = github_service::post_status(
                &config.github_token,
                repo_full_name,
                commit_sha,
                "pending",
                "Build queued",
                &format!("{}/ci/api/builds/{}", config.dashboard_url, build.id),
                "centrix-ci",
            )
            .await;

            Ok(StatusCode::CREATED)
        }
        Err(e) => {
            tracing::error!("Failed to create build from PR: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
