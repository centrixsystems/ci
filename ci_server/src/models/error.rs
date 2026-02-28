//! ci.error + ci.error.occurrence — Deduplicated error tracking across builds.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{ci_error_occurrences, ci_errors};

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_errors)]
pub struct CiError {
    pub id: i64,
    pub tenant_id: Uuid,
    pub project_id: Option<i64>,
    pub fingerprint: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub occurrence_count: i32,
    pub status: String,
    pub assigned_to: Option<String>,
    pub notes: Option<String>,
    pub raw_text: String,
    pub normalized_text: String,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = ci_errors)]
pub struct NewCiError {
    pub tenant_id: Uuid,
    pub project_id: Option<i64>,
    pub fingerprint: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub occurrence_count: i32,
    pub status: String,
    pub raw_text: String,
    pub normalized_text: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ci_error_occurrences)]
pub struct CiErrorOccurrence {
    pub id: i64,
    pub tenant_id: Uuid,
    pub error_id: i64,
    pub build_id: i64,
    pub step_name: String,
    pub raw_output: Option<String>,
    pub active: bool,
    pub create_uid: Option<i64>,
    pub create_date: Option<DateTime<Utc>>,
    pub write_uid: Option<i64>,
    pub write_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = ci_error_occurrences)]
pub struct NewCiErrorOccurrence {
    pub tenant_id: Uuid,
    pub error_id: i64,
    pub build_id: i64,
    pub step_name: String,
    pub raw_output: Option<String>,
}
