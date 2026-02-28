//! Build executor — background task that polls for pending builds and runs them.
//!
//! Picks up `status = 'pending'` builds, checks out the repo, runs each
//! pipeline step as a shell command, and records stdout/stderr/exit_code.

use std::sync::Arc;
use std::time::Instant;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;

use erp_core::db::diesel_pool::DieselPool;

use crate::config::CiConfig;
use crate::schema::{ci_builds, ci_projects};
use crate::services::step_executor;

/// Run the executor loop forever. Spawned as a background tokio task.
pub async fn run_executor(pool: Arc<DieselPool>, config: CiConfig) {
    tracing::info!(
        workspace = %config.workspace_dir,
        max_concurrent = config.max_concurrent_builds,
        "Build executor started"
    );

    loop {
        if let Err(e) = poll_and_execute(&pool, &config).await {
            tracing::error!("Executor poll error: {e}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

/// Poll for one pending build and execute it.
async fn poll_and_execute(pool: &DieselPool, config: &CiConfig) -> anyhow::Result<()> {
    let mut conn = pool.get().await?;

    // Check how many builds are currently running
    let running_count: i64 = ci_builds::table
        .filter(ci_builds::status.eq("running"))
        .count()
        .get_result(&mut conn)
        .await?;

    if running_count >= config.max_concurrent_builds as i64 {
        return Ok(());
    }

    // Pick the oldest pending build
    let pending: Option<PendingBuild> = ci_builds::table
        .inner_join(ci_projects::table)
        .filter(ci_builds::status.eq("pending"))
        .order(ci_builds::id.asc())
        .select((
            ci_builds::id,
            ci_builds::tenant_id,
            ci_builds::commit_sha,
            ci_builds::branch,
            ci_projects::github_repo,
            ci_projects::pipeline_config,
        ))
        .first(&mut conn)
        .await
        .optional()?;

    let build = match pending {
        Some(b) => b,
        None => return Ok(()),
    };

    tracing::info!(
        build_id = build.id,
        repo = %build.github_repo,
        branch = %build.branch,
        "Executing build"
    );

    // Mark as running
    diesel::update(ci_builds::table.find(build.id))
        .set((
            ci_builds::status.eq("running"),
            ci_builds::started_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .await?;

    crate::metrics::build_status_changed("running");

    // Parse pipeline config
    let pipeline = parse_pipeline(&build.pipeline_config);
    let build_start = Instant::now();

    // Determine working directory
    let work_dir = if let Some(ref local_path) = pipeline.local_path {
        local_path.clone()
    } else {
        // Clone from GitHub
        let workspace = format!("{}/{}", config.workspace_dir, build.id);
        tokio::fs::create_dir_all(&workspace).await?;
        let clone_url = format!("https://github.com/{}.git", build.github_repo);

        let clone_result = Command::new("git")
            .args(["clone", "--depth", "1", "--branch", &build.branch, &clone_url, &workspace])
            .output()
            .await;

        match clone_result {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::error!(build_id = build.id, "git clone failed: {stderr}");
                finish_build(&mut conn, build.id, "failure", build_start, Some(&format!("git clone failed: {stderr}"))).await?;
                return Ok(());
            }
            Err(e) => {
                tracing::error!(build_id = build.id, "git clone error: {e}");
                finish_build(&mut conn, build.id, "failure", build_start, Some(&format!("git clone error: {e}"))).await?;
                return Ok(());
            }
        }

        // Checkout specific commit if not HEAD
        if build.commit_sha != "HEAD" && build.commit_sha.len() >= 7 {
            let _ = Command::new("git")
                .args(["checkout", &build.commit_sha])
                .current_dir(&workspace)
                .output()
                .await;
        }

        workspace
    };

    // If local_path, do a git pull to get latest
    if pipeline.local_path.is_some() {
        let pull_result = Command::new("git")
            .args(["pull", "--ff-only"])
            .current_dir(&work_dir)
            .output()
            .await;

        if let Ok(output) = pull_result {
            if !output.status.success() {
                tracing::warn!(
                    build_id = build.id,
                    "git pull failed (continuing with current state): {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    // Execute each step
    let mut all_passed = true;
    let timeout = std::time::Duration::from_secs(pipeline.timeout_secs);

    for (seq, step_def) in pipeline.steps.iter().enumerate() {
        let step_start = Instant::now();

        let step_id = step_executor::start_step(
            &mut conn,
            build.id,
            &step_def.name,
            (seq + 1) as i32,
            build.tenant_id,
        )
        .await?;

        tracing::info!(
            build_id = build.id,
            step = %step_def.name,
            command = %step_def.command,
            "Running step"
        );

        // Run the command with timeout
        let cmd_result = tokio::time::timeout(timeout, async {
            Command::new("bash")
                .args(["-c", &step_def.command])
                .current_dir(&work_dir)
                .env("CI", "true")
                .env("CI_BUILD_ID", build.id.to_string())
                .env("CI_BRANCH", &build.branch)
                .env("CI_COMMIT", &build.commit_sha)
                .output()
                .await
        })
        .await;

        let (exit_code, stdout_str, stderr_str) = match cmd_result {
            Ok(Ok(output)) => {
                let code = output.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                // Truncate to 64KB per field
                let stdout = if stdout.len() > 65536 {
                    format!("...truncated...\n{}", &stdout[stdout.len() - 65536..])
                } else {
                    stdout
                };
                let stderr = if stderr.len() > 65536 {
                    format!("...truncated...\n{}", &stderr[stderr.len() - 65536..])
                } else {
                    stderr
                };
                (code, stdout, stderr)
            }
            Ok(Err(e)) => {
                (-1, String::new(), format!("Failed to execute command: {e}"))
            }
            Err(_) => {
                (-1, String::new(), format!("Step timed out after {}s", timeout.as_secs()))
            }
        };

        let step_duration = step_start.elapsed().as_millis() as i32;

        step_executor::complete_step(
            &mut conn,
            step_id,
            exit_code,
            step_duration,
            Some(stdout_str),
            Some(stderr_str),
        )
        .await?;

        crate::metrics::step_duration(&step_def.name, step_duration as u64);

        if exit_code != 0 {
            tracing::warn!(
                build_id = build.id,
                step = %step_def.name,
                exit_code,
                "Step failed"
            );
            all_passed = false;
            // Mark remaining steps as skipped
            for (skip_seq, skip_step) in pipeline.steps.iter().enumerate().skip(seq + 1) {
                let skip_id = step_executor::start_step(
                    &mut conn,
                    build.id,
                    &skip_step.name,
                    (skip_seq + 1) as i32,
                    build.tenant_id,
                )
                .await?;
                step_executor::complete_step(&mut conn, skip_id, -1, 0, None, Some("Skipped (previous step failed)".to_string()))
                    .await?;
            }
            break;
        }

        tracing::info!(
            build_id = build.id,
            step = %step_def.name,
            duration_ms = step_duration,
            "Step passed"
        );
    }

    let final_status = if all_passed { "success" } else { "failure" };
    finish_build(&mut conn, build.id, final_status, build_start, None).await?;

    // Cleanup cloned workspace (only if we cloned, not local_path)
    if pipeline.local_path.is_none() {
        let workspace = format!("{}/{}", config.workspace_dir, build.id);
        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    Ok(())
}

/// Update build to terminal status with timing.
async fn finish_build(
    conn: &mut diesel_async::AsyncPgConnection,
    build_id: i64,
    status: &str,
    start: Instant,
    error_msg: Option<&str>,
) -> anyhow::Result<()> {
    let duration = start.elapsed().as_millis() as i32;

    let summary = error_msg.map(|msg| serde_json::json!({"error": msg}));

    diesel::update(ci_builds::table.find(build_id))
        .set((
            ci_builds::status.eq(status),
            ci_builds::finished_at.eq(chrono::Utc::now()),
            ci_builds::duration_ms.eq(duration),
            ci_builds::summary.eq(summary),
        ))
        .execute(conn)
        .await?;

    crate::metrics::build_status_changed(status);
    crate::metrics::build_duration(duration as u64);

    tracing::info!(
        build_id,
        status,
        duration_ms = duration,
        "Build finished"
    );

    Ok(())
}

// ── Pipeline config parsing ──

#[derive(Debug, Clone, Queryable)]
struct PendingBuild {
    pub id: i64,
    pub tenant_id: uuid::Uuid,
    pub commit_sha: String,
    pub branch: String,
    pub github_repo: String,
    pub pipeline_config: Option<serde_json::Value>,
}

struct PipelineConfig {
    steps: Vec<StepDef>,
    timeout_secs: u64,
    local_path: Option<String>,
}

struct StepDef {
    name: String,
    command: String,
}

fn parse_pipeline(config: &Option<serde_json::Value>) -> PipelineConfig {
    let config = match config {
        Some(v) => v,
        None => {
            return PipelineConfig {
                steps: vec![StepDef {
                    name: "check".to_string(),
                    command: "echo 'No pipeline configured'".to_string(),
                }],
                timeout_secs: 600,
                local_path: None,
            };
        }
    };

    let steps = config
        .get("steps")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|step| {
                    let name = step.get("name")?.as_str()?.to_string();
                    let command = step.get("command")?.as_str()?.to_string();
                    Some(StepDef { name, command })
                })
                .collect()
        })
        .unwrap_or_default();

    let timeout_secs = config
        .get("timeout_secs")
        .and_then(|t| t.as_u64())
        .unwrap_or(600);

    let local_path = config
        .get("local_path")
        .and_then(|p| p.as_str())
        .map(|s| s.to_string());

    PipelineConfig {
        steps,
        timeout_secs,
        local_path,
    }
}
