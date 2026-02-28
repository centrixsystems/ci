//! REST API for builds and projects.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

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
