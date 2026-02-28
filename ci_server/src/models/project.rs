//! ci.project — A registered GitHub repo with pipeline config.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_projects;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_projects)]
pub struct CiProject {
    pub id: i64,
    pub tenant_id: Uuid,
    pub name: String,
    pub github_repo: String,
    pub default_branch: String,
    pub pipeline_config: Option<serde_json::Value>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_projects)]
pub struct NewCiProject {
    pub tenant_id: Uuid,
    pub name: String,
    pub github_repo: String,
    pub default_branch: String,
    pub pipeline_config: Option<serde_json::Value>,
    pub active: bool,
}
