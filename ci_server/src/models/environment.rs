//! ci.environment — Ephemeral review environment provisioned for a build.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_environments;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_environments)]
pub struct CiEnvironment {
    pub id: i64,
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub build_id: Option<i64>,
    pub pr_number: i32,
    pub branch: String,
    pub commit_sha: String,
    pub status: String,
    pub url: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
    pub idle_timeout_min: i32,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_environments)]
pub struct NewCiEnvironment {
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub build_id: Option<i64>,
    pub pr_number: i32,
    pub branch: String,
    pub commit_sha: String,
    pub status: String,
    pub idle_timeout_min: i32,
}
