//! GitHub integration — webhook validation, status updates, PR comments.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Validate a GitHub webhook signature (X-Hub-Signature-256).
pub fn validate_signature(secret: &str, payload: &[u8], signature: &str) -> bool {
    if secret.is_empty() {
        tracing::warn!("Webhook secret not configured, skipping validation");
        return true;
    }

    let sig = signature.strip_prefix("sha256=").unwrap_or(signature);
    let sig_bytes = match hex::decode(sig) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(payload);

    mac.verify_slice(&sig_bytes).is_ok()
}

/// Post a commit status to GitHub.
pub async fn post_status(
    token: &str,
    repo: &str,
    sha: &str,
    state: &str,
    description: &str,
    target_url: &str,
    context: &str,
) -> anyhow::Result<()> {
    if token.is_empty() {
        tracing::debug!("GitHub token not set, skipping status update");
        return Ok(());
    }

    let url = format!("https://api.github.com/repos/{repo}/statuses/{sha}");
    let body = serde_json::json!({
        "state": state,
        "description": description,
        "target_url": target_url,
        "context": context,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "centrix-ci")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::warn!("GitHub status update failed: {} {}", status, text);
    }

    Ok(())
}

/// Post a comment on a PR.
pub async fn post_pr_comment(
    token: &str,
    repo: &str,
    pr_number: i32,
    body: &str,
) -> anyhow::Result<()> {
    if token.is_empty() {
        return Ok(());
    }

    let url = format!("https://api.github.com/repos/{repo}/issues/{pr_number}/comments");
    let payload = serde_json::json!({ "body": body });

    let client = reqwest::Client::new();
    client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "centrix-ci")
        .json(&payload)
        .send()
        .await?;

    Ok(())
}
