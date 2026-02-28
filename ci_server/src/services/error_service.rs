//! Error fingerprinting and deduplication across builds.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use regex::Regex;
use std::sync::LazyLock;

use crate::models::error::{CiError, NewCiError, NewCiErrorOccurrence};
use crate::schema::{ci_error_occurrences, ci_errors};

static NUMERIC_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d+\b").unwrap());
static PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/[a-zA-Z0-9_./-]+").unwrap());

/// Normalize error text for fingerprinting: remove numbers, paths, whitespace.
pub fn normalize(text: &str) -> String {
    let text = NUMERIC_REGEX.replace_all(text, "N");
    let text = PATH_REGEX.replace_all(&text, "PATH");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Generate a fingerprint from normalized error text.
pub fn fingerprint(normalized: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(normalized.as_bytes());
    hex::encode(&hash[..16])
}

/// Classify error category from text.
pub fn classify_category(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if lower.contains("compile") || lower.contains("cannot find") || lower.contains("expected") {
        "compile"
    } else if lower.contains("test") || lower.contains("assertion") || lower.contains("panicked") {
        "test"
    } else if lower.contains("lint") || lower.contains("clippy") || lower.contains("warning") {
        "lint"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout"
    } else {
        "runtime"
    }
}

/// Record an error occurrence, creating or updating the deduplicated error record.
pub async fn record_error(
    conn: &mut AsyncPgConnection,
    build_id: i64,
    step_name: &str,
    raw_text: &str,
    tenant_id: uuid::Uuid,
    project_id: Option<i64>,
) -> anyhow::Result<i64> {
    let normalized = normalize(raw_text);
    let fp = fingerprint(&normalized);
    let category = classify_category(raw_text);
    let now = chrono::Utc::now();

    // Find existing error by fingerprint
    let existing: Option<CiError> = ci_errors::table
        .filter(ci_errors::fingerprint.eq(&fp))
        .filter(ci_errors::tenant_id.eq(tenant_id))
        .first(conn)
        .await
        .optional()?;

    let error_id = if let Some(err) = existing {
        // Update occurrence count and last_seen
        diesel::update(ci_errors::table.find(err.id))
            .set((
                ci_errors::occurrence_count.eq(err.occurrence_count + 1),
                ci_errors::last_seen_at.eq(now),
            ))
            .execute(conn)
            .await?;
        err.id
    } else {
        // Create new error
        let title = raw_text.lines().next().unwrap_or("Unknown error");
        let title = if title.len() > 200 {
            &title[..200]
        } else {
            title
        };

        let new_error = NewCiError {
            tenant_id,
            project_id,
            fingerprint: fp,
            category: category.to_string(),
            severity: "error".to_string(),
            title: title.to_string(),
            file_path: None,
            line_number: None,
            first_seen_at: now,
            last_seen_at: now,
            occurrence_count: 1,
            status: "open".to_string(),
            raw_text: raw_text.to_string(),
            normalized_text: normalized,
        };

        let result: CiError = diesel::insert_into(ci_errors::table)
            .values(&new_error)
            .get_result(conn)
            .await?;
        result.id
    };

    // Record occurrence
    let occurrence = NewCiErrorOccurrence {
        tenant_id,
        error_id,
        build_id,
        step_name: step_name.to_string(),
        raw_output: Some(raw_text.to_string()),
    };

    diesel::insert_into(ci_error_occurrences::table)
        .values(&occurrence)
        .execute(conn)
        .await?;

    crate::metrics::error_recorded(category);
    Ok(error_id)
}
