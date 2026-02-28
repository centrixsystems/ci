//! ci.artifact — Build artifacts (logs, test results, coverage reports).

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_artifacts;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_artifacts)]
pub struct CiArtifact {
    pub id: i64,
    pub tenant_id: Uuid,
    pub build_id: i64,
    pub name: String,
    pub artifact_type: String,
    pub content: Option<String>,
    pub size_bytes: Option<i64>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_artifacts)]
pub struct NewCiArtifact {
    pub tenant_id: Uuid,
    pub build_id: i64,
    pub name: String,
    pub artifact_type: String,
    pub content: Option<String>,
    pub size_bytes: Option<i64>,
}
