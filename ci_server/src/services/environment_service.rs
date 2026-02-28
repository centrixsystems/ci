//! Ephemeral environment management (pluggable backends).

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::environment::{CiEnvironment, NewCiEnvironment};
use crate::schema::ci_environments;

/// Count currently active (non-destroyed) environments.
pub async fn count_active(conn: &mut AsyncPgConnection) -> anyhow::Result<i64> {
    let count = ci_environments::table
        .filter(ci_environments::status.ne("destroyed"))
        .filter(ci_environments::active.eq(true))
        .count()
        .get_result(conn)
        .await?;
    Ok(count)
}

/// Create a new environment record.
pub async fn create_environment(
    conn: &mut AsyncPgConnection,
    new_env: NewCiEnvironment,
) -> anyhow::Result<CiEnvironment> {
    let result = diesel::insert_into(ci_environments::table)
        .values(&new_env)
        .get_result::<CiEnvironment>(conn)
        .await?;
    Ok(result)
}

/// Update environment status.
pub async fn update_status(
    conn: &mut AsyncPgConnection,
    env_id: i64,
    status: &str,
) -> anyhow::Result<()> {
    diesel::update(ci_environments::table.find(env_id))
        .set(ci_environments::status.eq(status))
        .execute(conn)
        .await?;
    Ok(())
}

/// List environments for a specific PR.
pub async fn list_for_pr(
    conn: &mut AsyncPgConnection,
    project_id: i64,
    pr_number: i32,
) -> anyhow::Result<Vec<CiEnvironment>> {
    let results = ci_environments::table
        .filter(ci_environments::project_id.eq(project_id))
        .filter(ci_environments::pr_number.eq(pr_number))
        .filter(ci_environments::active.eq(true))
        .order(ci_environments::id.desc())
        .load::<CiEnvironment>(conn)
        .await?;
    Ok(results)
}
