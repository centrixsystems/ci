use dagger_sdk::{Directory, Query};

use crate::containers;

/// Build Tailwind CSS v4 from erp_web/static/.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let static_dir = source.directory("erp_web/static");

    let output = containers::node_base(client, static_dir)
        .with_exec(vec!["npm", "ci"])
        .with_exec(vec![
            "npx", "@tailwindcss/cli",
            "-i", "css/input.css",
            "-o", "css/main.css",
            "--minify",
        ])
        .stdout()
        .await?;

    Ok(format!("[tailwind] CSS build complete.\n{output}"))
}
