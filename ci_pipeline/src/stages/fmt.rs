use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run `cargo fmt --workspace --check` to verify formatting.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let output = containers::rust_base(client, source)
        .with_exec(vec!["cargo", "fmt", "--workspace", "--check"])
        .stdout()
        .await?;

    Ok(format!("[fmt] Format check passed.\n{output}"))
}
