//! CI platform services — generic, pipeline-agnostic business logic.

pub mod artifact_service;
pub mod build_service;
pub mod environment_service;
pub mod error_service;
pub mod executor;
pub mod github_service;
pub mod project_service;
pub mod step_executor;
