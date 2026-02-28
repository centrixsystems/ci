//! Environment event definitions for event sourcing.

use serde::{Deserialize, Serialize};

/// Events that can happen to a CI environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CiEnvironmentEvent {
    /// Environment was requested for a build.
    EnvironmentRequested {
        project_id: i64,
        build_id: Option<i64>,
        pr_number: i32,
        branch: String,
        commit_sha: String,
    },
    /// Environment is being provisioned.
    EnvironmentCreating { url: Option<String> },
    /// Environment is ready and running.
    EnvironmentRunning { url: String },
    /// Environment went dormant due to inactivity.
    EnvironmentDormant,
    /// Environment was woken up from dormant state.
    EnvironmentWoken,
    /// Environment was destroyed.
    EnvironmentDestroyed { reason: String },
}

/// Aggregate state for a CI environment.
#[derive(Debug, Clone, Default)]
pub struct CiEnvironmentAggregate {
    pub status: String,
}

impl CiEnvironmentAggregate {
    pub fn apply(&mut self, event: &CiEnvironmentEvent) {
        match event {
            CiEnvironmentEvent::EnvironmentRequested { .. } => {
                self.status = "requested".to_string();
            }
            CiEnvironmentEvent::EnvironmentCreating { .. } => {
                self.status = "creating".to_string();
            }
            CiEnvironmentEvent::EnvironmentRunning { .. } => {
                self.status = "running".to_string();
            }
            CiEnvironmentEvent::EnvironmentDormant => {
                self.status = "dormant".to_string();
            }
            CiEnvironmentEvent::EnvironmentWoken => {
                self.status = "running".to_string();
            }
            CiEnvironmentEvent::EnvironmentDestroyed { .. } => {
                self.status = "destroyed".to_string();
            }
        }
    }
}
