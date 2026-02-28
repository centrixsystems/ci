use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run `cargo audit` to check for known vulnerabilities.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let output = containers::rust_base(client, source)
        .with_exec(vec!["cargo", "install", "cargo-audit"])
        .with_exec(vec!["cargo", "audit"])
        .stdout()
        .await?;

    Ok(format!("[security] Audit passed.\n{output}"))
}
