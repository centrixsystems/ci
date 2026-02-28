//! Build artifact storage and retrieval.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::artifact::{CiArtifact, NewCiArtifact};
use crate::schema::ci_artifacts;

/// Store a build artifact.
pub async fn store_artifact(
    conn: &mut AsyncPgConnection,
    new_artifact: NewCiArtifact,
) -> anyhow::Result<CiArtifact> {
    let result = diesel::insert_into(ci_artifacts::table)
        .values(&new_artifact)
        .get_result::<CiArtifact>(conn)
        .await?;
    Ok(result)
}

/// List artifacts for a build.
pub async fn list_for_build(
    conn: &mut AsyncPgConnection,
    build_id: i64,
) -> anyhow::Result<Vec<CiArtifact>> {
    let results = ci_artifacts::table
        .filter(ci_artifacts::build_id.eq(build_id))
        .filter(ci_artifacts::active.eq(true))
        .order(ci_artifacts::id.asc())
        .load::<CiArtifact>(conn)
        .await?;
    Ok(results)
}
