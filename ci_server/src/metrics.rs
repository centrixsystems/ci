//! Prometheus metrics for CI platform observability.

use metrics::{counter, gauge, histogram};

/// Initialize metrics exporter (Prometheus).
pub fn init_metrics() {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    if let Err(e) = builder.install() {
        tracing::warn!("Failed to install Prometheus exporter: {}", e);
    }
}

/// Record a webhook received event.
pub fn webhook_received(event_type: &str) {
    counter!("ci_webhooks_received_total", "event" => event_type.to_string()).increment(1);
}

/// Record a build state transition.
pub fn build_status_changed(status: &str) {
    counter!("ci_builds_total", "status" => status.to_string()).increment(1);
}

/// Record build duration.
pub fn build_duration(duration_ms: u64) {
    histogram!("ci_build_duration_ms").record(duration_ms as f64);
}

/// Record step duration.
pub fn step_duration(step_name: &str, duration_ms: u64) {
    histogram!("ci_step_duration_ms", "step" => step_name.to_string()).record(duration_ms as f64);
}

/// Set current active environment count.
pub fn active_environments(count: usize) {
    gauge!("ci_active_environments").set(count as f64);
}

/// Record an error occurrence.
pub fn error_recorded(category: &str) {
    counter!("ci_errors_total", "category" => category.to_string()).increment(1);
}
