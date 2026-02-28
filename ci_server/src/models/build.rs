//! ci.build — A pipeline run (triggered by webhook or manual).

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_builds;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_builds)]
pub struct CiBuild {
    pub id: i64,
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub commit_sha: String,
    pub branch: String,
    pub pr_number: Option<i32>,
    pub author: Option<String>,
    pub message: Option<String>,
    pub fingerprint: String,
    pub trigger_event: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub summary: Option<serde_json::Value>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_builds)]
pub struct NewCiBuild {
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub commit_sha: String,
    pub branch: String,
    pub pr_number: Option<i32>,
    pub author: Option<String>,
    pub message: Option<String>,
    pub fingerprint: String,
    pub trigger_event: String,
    pub status: String,
}
