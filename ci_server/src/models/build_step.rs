//! ci.build.step — Individual step within a build.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_build_steps;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_build_steps)]
pub struct CiBuildStep {
    pub id: i64,
    pub tenant_id: Uuid,
    pub build_id: i64,
    pub name: String,
    pub sequence: i32,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_build_steps)]
pub struct NewCiBuildStep {
    pub tenant_id: Uuid,
    pub build_id: i64,
    pub name: String,
    pub sequence: i32,
    pub status: String,
}
