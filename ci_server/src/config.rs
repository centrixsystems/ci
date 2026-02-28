//! CI platform configuration — loaded from environment variables.

#[derive(Clone, Debug)]
pub struct CiConfig {
    /// GitHub webhook secret for HMAC validation.
    pub github_webhook_secret: String,
    /// GitHub personal access token for API calls.
    pub github_token: String,
    /// Throttle window in seconds between duplicate builds.
    pub throttle_window_secs: u64,
    /// Maximum number of concurrent builds across all projects.
    pub max_concurrent_builds: usize,
    /// Dashboard base URL for GitHub status links.
    pub dashboard_url: String,
    /// Maximum running ephemeral environments.
    pub max_running_envs: usize,
    /// Maximum environments per PR.
    pub max_envs_per_pr: usize,
    /// Maximum total environments.
    pub max_envs_global: usize,
    /// Days before dormant environments are destroyed.
    pub dormant_ttl_days: i64,
    /// Minutes of inactivity before environment goes dormant.
    pub idle_timeout_min: i64,
}

impl CiConfig {
    pub fn from_env() -> Self {
        let github_webhook_secret = std::env::var("CI_WEBHOOK_SECRET").unwrap_or_default();
        let github_token = std::env::var("CI_GITHUB_TOKEN").unwrap_or_default();
        let throttle_window_secs = std::env::var("CI_THROTTLE_WINDOW")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        let max_concurrent_builds = std::env::var("CI_MAX_CONCURRENT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let dashboard_url =
            std::env::var("CI_DASHBOARD_URL").unwrap_or_else(|_| "http://localhost:9090/ci".to_string());
        let max_running_envs = std::env::var("CI_MAX_RUNNING_ENVS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);
        let max_envs_per_pr = std::env::var("CI_MAX_ENVS_PER_PR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let max_envs_global = std::env::var("CI_MAX_ENVS_GLOBAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20);
        let dormant_ttl_days = std::env::var("CI_DORMANT_TTL_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(7);
        let idle_timeout_min = std::env::var("CI_IDLE_TIMEOUT_MIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        if github_webhook_secret.is_empty() {
            tracing::warn!("CI_WEBHOOK_SECRET not set -- webhook signature validation disabled");
        }
        if github_token.is_empty() {
            tracing::warn!("CI_GITHUB_TOKEN not set -- GitHub status updates disabled");
        }

        Self {
            github_webhook_secret,
            github_token,
            throttle_window_secs,
            max_concurrent_builds,
            dashboard_url,
            max_running_envs,
            max_envs_per_pr,
            max_envs_global,
            dormant_ttl_days,
            idle_timeout_min,
        }
    }
}
