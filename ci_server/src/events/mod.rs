//! Event sourcing for CI platform.
//!
//! Build and Environment are EventSourced models.
//! Events are the source of truth for state transitions.

pub mod build;
pub mod environment;
