//! Project CRUD and pipeline discovery.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::models::project::{CiProject, NewCiProject};
use crate::schema::ci_projects;

/// List all active projects.
pub async fn list_projects(conn: &mut AsyncPgConnection) -> anyhow::Result<Vec<CiProject>> {
    let results = ci_projects::table
        .filter(ci_projects::active.eq(true))
        .order(ci_projects::id.asc())
        .load::<CiProject>(conn)
        .await?;
    Ok(results)
}

/// Find a project by its GitHub repo identifier (e.g., "centrixsystems/centrix").
pub async fn find_by_repo(
    conn: &mut AsyncPgConnection,
    github_repo: &str,
) -> anyhow::Result<Option<CiProject>> {
    let result = ci_projects::table
        .filter(ci_projects::github_repo.eq(github_repo))
        .filter(ci_projects::active.eq(true))
        .first::<CiProject>(conn)
        .await
        .optional()?;
    Ok(result)
}

/// Create a new project.
pub async fn create_project(
    conn: &mut AsyncPgConnection,
    new_project: NewCiProject,
) -> anyhow::Result<CiProject> {
    let result = diesel::insert_into(ci_projects::table)
        .values(&new_project)
        .get_result::<CiProject>(conn)
        .await?;
    Ok(result)
}
