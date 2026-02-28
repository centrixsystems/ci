//! Step executor — runs individual pipeline steps (Dagger functions).
//!
//! The step executor is generic: it discovers and runs steps from
//! the project's pipeline definition, not hardcoded stages.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::build_step::NewCiBuildStep;
use crate::schema::ci_build_steps;

/// Record a step starting.
pub async fn start_step(
    conn: &mut AsyncPgConnection,
    build_id: i64,
    step_name: &str,
    sequence: i32,
    tenant_id: uuid::Uuid,
) -> anyhow::Result<i64> {
    let new_step = NewCiBuildStep {
        tenant_id,
        build_id,
        name: step_name.to_string(),
        sequence,
        status: "running".to_string(),
    };

    let result: crate::models::build_step::CiBuildStep =
        diesel::insert_into(ci_build_steps::table)
            .values(&new_step)
            .get_result(conn)
            .await?;

    Ok(result.id)
}

/// Record a step completing.
pub async fn complete_step(
    conn: &mut AsyncPgConnection,
    step_id: i64,
    exit_code: i32,
    duration_ms: i32,
    stdout: Option<String>,
    stderr: Option<String>,
) -> anyhow::Result<()> {
    let status = if exit_code == 0 { "success" } else { "failure" };

    diesel::update(ci_build_steps::table.find(step_id))
        .set((
            ci_build_steps::status.eq(status),
            ci_build_steps::exit_code.eq(exit_code),
            ci_build_steps::duration_ms.eq(duration_ms),
            ci_build_steps::stdout.eq(stdout),
            ci_build_steps::stderr.eq(stderr),
            ci_build_steps::finished_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)
        .await?;

    Ok(())
}
