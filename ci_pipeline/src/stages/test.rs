use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run `cargo test --workspace --lib` unit tests.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let output = containers::rust_base(client, source)
        .with_exec(vec!["cargo", "test", "--workspace", "--lib"])
        .stdout()
        .await?;

    Ok(format!("[test] Unit tests passed.\n{output}"))
}
