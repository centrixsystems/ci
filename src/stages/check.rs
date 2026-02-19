use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run `cargo check --workspace` to verify compilation.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let output = containers::rust_base(client, source)
        .with_exec(vec!["cargo", "check", "--workspace"])
        .stdout()
        .await?;

    Ok(format!("[check] Compile check passed.\n{output}"))
}
