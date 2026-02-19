use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run `cargo clippy` with correctness errors and all warnings.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let output = containers::rust_base(client, source)
        .with_exec(vec![
            "cargo", "clippy", "--workspace", "--lib",
            "--", "-D", "clippy::correctness", "-W", "clippy::all",
        ])
        .stdout()
        .await?;

    Ok(format!("[lint] Clippy passed.\n{output}"))
}
