//! KPI queries for the CI dashboard.

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

/// Build success rate over N days.
#[derive(Debug, Serialize, QueryableByName)]
pub struct BuildSuccessRate {
    #[diesel(sql_type = BigInt)]
    pub total: i64,
    #[diesel(sql_type = BigInt)]
    pub success: i64,
    #[diesel(sql_type = Double)]
    pub rate: f64,
}

pub async fn query_success_rate(
    conn: &mut AsyncPgConnection,
    days: i32,
) -> anyhow::Result<BuildSuccessRate> {
    let result = diesel::sql_query(format!(
        "SELECT \
            COUNT(*) AS total, \
            COUNT(*) FILTER (WHERE status = 'success') AS success, \
            COALESCE(COUNT(*) FILTER (WHERE status = 'success')::float / NULLIF(COUNT(*), 0), 0) AS rate \
         FROM ci_builds \
         WHERE create_date >= NOW() - INTERVAL '{days} days' \
           AND status IN ('success', 'failure')"
    ))
    .get_result(conn)
    .await?;
    Ok(result)
}

/// Average build duration over N days.
#[derive(Debug, Serialize, QueryableByName)]
pub struct AvgBuildDuration {
    #[diesel(sql_type = Nullable<Double>)]
    pub avg_ms: Option<f64>,
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

pub async fn query_avg_duration(
    conn: &mut AsyncPgConnection,
    days: i32,
) -> anyhow::Result<AvgBuildDuration> {
    let result = diesel::sql_query(format!(
        "SELECT \
            AVG(duration_ms)::float AS avg_ms, \
            COUNT(*) AS count \
         FROM ci_builds \
         WHERE create_date >= NOW() - INTERVAL '{days} days' \
           AND duration_ms IS NOT NULL"
    ))
    .get_result(conn)
    .await?;
    Ok(result)
}

/// Environment utilization snapshot.
#[derive(Debug, Serialize, QueryableByName)]
pub struct EnvironmentUtilization {
    #[diesel(sql_type = BigInt)]
    pub total: i64,
    #[diesel(sql_type = BigInt)]
    pub running: i64,
    #[diesel(sql_type = BigInt)]
    pub dormant: i64,
    #[diesel(sql_type = BigInt)]
    pub creating: i64,
}

pub async fn query_env_utilization(
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<EnvironmentUtilization> {
    let result = diesel::sql_query(
        "SELECT \
            COUNT(*) FILTER (WHERE status != 'destroyed') AS total, \
            COUNT(*) FILTER (WHERE status = 'running') AS running, \
            COUNT(*) FILTER (WHERE status = 'dormant') AS dormant, \
            COUNT(*) FILTER (WHERE status = 'creating') AS creating \
         FROM ci_environments",
    )
    .get_result(conn)
    .await?;
    Ok(result)
}

/// Build count grouped by status.
#[derive(Debug, Serialize, QueryableByName)]
pub struct BuildsByStatus {
    #[diesel(sql_type = Text)]
    pub status: String,
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

pub async fn query_builds_by_status(
    conn: &mut AsyncPgConnection,
    days: i32,
) -> anyhow::Result<Vec<BuildsByStatus>> {
    let results = diesel::sql_query(format!(
        "SELECT status, COUNT(*) AS count \
         FROM ci_builds \
         WHERE create_date >= NOW() - INTERVAL '{days} days' \
         GROUP BY status \
         ORDER BY count DESC"
    ))
    .load(conn)
    .await?;
    Ok(results)
}
