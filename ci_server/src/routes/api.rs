//! REST API for builds and projects.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};

use crate::models::build::CiBuild;
use crate::models::build_step::CiBuildStep;
use crate::schema::{ci_build_steps, ci_builds};

/// JSON response for a build with its steps.
#[derive(Debug, Serialize)]
pub struct BuildJson {
    pub id: i64,
    pub project_id: i64,
    pub commit_sha: String,
    pub branch: String,
    pub pr_number: Option<i32>,
    pub author: Option<String>,
    pub message: Option<String>,
    pub status: String,
    pub trigger_event: String,
    pub duration_ms: Option<i32>,
    pub create_date: Option<chrono::DateTime<chrono::Utc>>,
    pub steps: Vec<StepJson>,
}

#[derive(Debug, Serialize)]
pub struct StepJson {
    pub id: i64,
    pub name: String,
    pub sequence: i32,
    pub status: String,
    pub duration_ms: Option<i32>,
    pub exit_code: Option<i32>,
}

/// Get a build by ID with its steps.
pub async fn get_build(conn: &mut AsyncPgConnection, build_id: i64) -> anyhow::Result<BuildJson> {
    let build: CiBuild = ci_builds::table.find(build_id).first(conn).await?;

    let steps: Vec<CiBuildStep> = ci_build_steps::table
        .filter(ci_build_steps::build_id.eq(build_id))
        .order(ci_build_steps::sequence.asc())
        .load(conn)
        .await?;

    Ok(BuildJson {
        id: build.id,
        project_id: build.project_id,
        commit_sha: build.commit_sha,
        branch: build.branch,
        pr_number: build.pr_number,
        author: build.author,
        message: build.message,
        status: build.status,
        trigger_event: build.trigger_event,
        duration_ms: build.duration_ms,
        create_date: build.create_date,
        steps: steps
            .into_iter()
            .map(|s| StepJson {
                id: s.id,
                name: s.name,
                sequence: s.sequence,
                status: s.status,
                duration_ms: s.duration_ms,
                exit_code: s.exit_code,
            })
            .collect(),
    })
}

/// Get the latest build for a project + branch.
pub async fn get_latest_build(
    conn: &mut AsyncPgConnection,
    project_id: i64,
    branch: &str,
) -> anyhow::Result<BuildJson> {
    let build: CiBuild = ci_builds::table
        .filter(ci_builds::project_id.eq(project_id))
        .filter(ci_builds::branch.eq(branch))
        .order(ci_builds::id.desc())
        .first(conn)
        .await?;

    let steps: Vec<CiBuildStep> = ci_build_steps::table
        .filter(ci_build_steps::build_id.eq(build.id))
        .order(ci_build_steps::sequence.asc())
        .load(conn)
        .await?;

    Ok(BuildJson {
        id: build.id,
        project_id: build.project_id,
        commit_sha: build.commit_sha,
        branch: build.branch,
        pr_number: build.pr_number,
        author: build.author,
        message: build.message,
        status: build.status,
        trigger_event: build.trigger_event,
        duration_ms: build.duration_ms,
        create_date: build.create_date,
        steps: steps
            .into_iter()
            .map(|s| StepJson {
                id: s.id,
                name: s.name,
                sequence: s.sequence,
                status: s.status,
                duration_ms: s.duration_ms,
                exit_code: s.exit_code,
            })
            .collect(),
    })
}

// ── Trigger API ──

/// Request body for manually triggering a build.
#[derive(Debug, Deserialize)]
pub struct TriggerRequest {
    pub project_id: i64,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
}

/// Response for a triggered build.
#[derive(Debug, Serialize)]
pub struct TriggerResponse {
    pub id: i64,
    pub status: String,
}

/// Manually trigger a build for a project.
pub async fn trigger_build(
    conn: &mut AsyncPgConnection,
    req: TriggerRequest,
) -> anyhow::Result<TriggerResponse> {
    use crate::models::build::NewCiBuild;
    use crate::schema::ci_projects;

    // Look up the project
    let project: crate::models::project::CiProject = ci_projects::table
        .find(req.project_id)
        .first(conn)
        .await
        .map_err(|_| anyhow::anyhow!("Project not found: {}", req.project_id))?;

    let branch = req.branch.unwrap_or_else(|| project.default_branch.clone());
    let commit_sha = req.commit_sha.unwrap_or_else(|| "HEAD".to_string());
    let fingerprint = format!("{}-{}-manual", commit_sha, branch);

    let new_build = NewCiBuild {
        tenant_id: project.tenant_id,
        project_id: project.id,
        commit_sha,
        branch,
        pr_number: None,
        author: Some("manual".to_string()),
        message: Some("Manual trigger via API".to_string()),
        fingerprint,
        trigger_event: "manual".to_string(),
        status: "pending".to_string(),
    };

    let build = crate::services::build_service::create_build(conn, new_build).await?;

    Ok(TriggerResponse {
        id: build.id,
        status: build.status,
    })
}

/// List builds with optional limit.
pub async fn list_builds(
    conn: &mut AsyncPgConnection,
    limit: i64,
) -> anyhow::Result<Vec<BuildJson>> {
    let builds: Vec<CiBuild> = ci_builds::table
        .order(ci_builds::id.desc())
        .limit(limit)
        .load(conn)
        .await?;

    let mut result = Vec::with_capacity(builds.len());
    for build in builds {
        let steps: Vec<CiBuildStep> = ci_build_steps::table
            .filter(ci_build_steps::build_id.eq(build.id))
            .order(ci_build_steps::sequence.asc())
            .load(conn)
            .await?;

        result.push(BuildJson {
            id: build.id,
            project_id: build.project_id,
            commit_sha: build.commit_sha,
            branch: build.branch,
            pr_number: build.pr_number,
            author: build.author,
            message: build.message,
            status: build.status,
            trigger_event: build.trigger_event,
            duration_ms: build.duration_ms,
            create_date: build.create_date,
            steps: steps
                .into_iter()
                .map(|s| StepJson {
                    id: s.id,
                    name: s.name,
                    sequence: s.sequence,
                    status: s.status,
                    duration_ms: s.duration_ms,
                    exit_code: s.exit_code,
                })
                .collect(),
        });
    }

    Ok(result)
}
