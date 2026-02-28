//! ci.trigger — Rules for when builds should execute.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::ci_triggers;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_triggers)]
pub struct CiTrigger {
    pub id: i64,
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub event_type: String,
    pub branch_pattern: Option<String>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_triggers)]
pub struct NewCiTrigger {
    pub tenant_id: Uuid,
    pub project_id: i64,
    pub event_type: String,
    pub branch_pattern: Option<String>,
    pub active: bool,
}
