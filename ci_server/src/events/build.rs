//! Build event definitions for event sourcing.

use serde::{Deserialize, Serialize};

/// Events that can happen to a CI build.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CiBuildEvent {
    /// Build was created from a webhook or manual trigger.
    BuildCreated {
        project_id: i64,
        commit_sha: String,
        branch: String,
        pr_number: Option<i32>,
        author: Option<String>,
        message: Option<String>,
        fingerprint: String,
        trigger_event: String,
    },
    /// Build execution has started.
    BuildStarted,
    /// A build step completed.
    StepCompleted {
        step_name: String,
        exit_code: i32,
        duration_ms: i32,
    },
    /// Build finished successfully.
    BuildSucceeded { duration_ms: i32 },
    /// Build failed.
    BuildFailed {
        duration_ms: i32,
        error_summary: Option<String>,
    },
    /// Build was cancelled.
    BuildCancelled,
}

/// Aggregate state for a CI build.
#[derive(Debug, Clone, Default)]
pub struct CiBuildAggregate {
    pub status: String,
    pub started: bool,
    pub finished: bool,
}

impl CiBuildAggregate {
    pub fn apply(&mut self, event: &CiBuildEvent) {
        match event {
            CiBuildEvent::BuildCreated { .. } => {
                self.status = "pending".to_string();
            }
            CiBuildEvent::BuildStarted => {
                self.status = "running".to_string();
                self.started = true;
            }
            CiBuildEvent::StepCompleted { .. } => {}
            CiBuildEvent::BuildSucceeded { .. } => {
                self.status = "success".to_string();
                self.finished = true;
            }
            CiBuildEvent::BuildFailed { .. } => {
                self.status = "failure".to_string();
                self.finished = true;
            }
            CiBuildEvent::BuildCancelled => {
                self.status = "cancelled".to_string();
                self.finished = true;
            }
        }
    }
}
