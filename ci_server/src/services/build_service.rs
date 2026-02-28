//! Build scheduling, throttling, and execution orchestration.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::build::{CiBuild, NewCiBuild};
use crate::schema::ci_builds;

/// Create a new build record.
pub async fn create_build(
    conn: &mut AsyncPgConnection,
    new_build: NewCiBuild,
) -> anyhow::Result<CiBuild> {
    let result = diesel::insert_into(ci_builds::table)
        .values(&new_build)
        .get_result::<CiBuild>(conn)
        .await?;

    crate::metrics::build_status_changed("pending");
    tracing::info!(
        build_id = result.id,
        project_id = result.project_id,
        branch = %result.branch,
        "Build created"
    );

    Ok(result)
}

/// Check if a duplicate build exists within the throttle window.
pub async fn is_duplicate(
    conn: &mut AsyncPgConnection,
    fingerprint: &str,
    throttle_secs: u64,
) -> anyhow::Result<bool> {
    use chrono::Utc;

    let cutoff = Utc::now() - chrono::Duration::seconds(throttle_secs as i64);
    let count: i64 = ci_builds::table
        .filter(ci_builds::fingerprint.eq(fingerprint))
        .filter(ci_builds::create_date.gt(cutoff))
        .count()
        .get_result(conn)
        .await?;

    Ok(count > 0)
}

/// Update build status.
pub async fn update_status(
    conn: &mut AsyncPgConnection,
    build_id: i64,
    status: &str,
) -> anyhow::Result<()> {
    diesel::update(ci_builds::table.find(build_id))
        .set(ci_builds::status.eq(status))
        .execute(conn)
        .await?;

    crate::metrics::build_status_changed(status);
    Ok(())
}

/// Get the latest build for a project + branch.
pub async fn get_latest(
    conn: &mut AsyncPgConnection,
    project_id: i64,
    branch: &str,
) -> anyhow::Result<Option<CiBuild>> {
    let result = ci_builds::table
        .filter(ci_builds::project_id.eq(project_id))
        .filter(ci_builds::branch.eq(branch))
        .order(ci_builds::id.desc())
        .first::<CiBuild>(conn)
        .await
        .optional()?;
    Ok(result)
}

/// Get a build by ID.
pub async fn get_build(
    conn: &mut AsyncPgConnection,
    build_id: i64,
) -> anyhow::Result<Option<CiBuild>> {
    let result = ci_builds::table
        .find(build_id)
        .first::<CiBuild>(conn)
        .await
        .optional()?;
    Ok(result)
}
