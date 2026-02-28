mod containers;
mod stages;

use clap::{Parser, Subcommand};
use dagger_sdk::{Directory, HostDirectoryOpts, Query};

#[derive(Parser)]
#[command(name = "centrix-ci", about = "Centrix CI/CD Pipeline")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fast compile check
    Check {
        #[arg(long)]
        source: String,
    },
    /// Format check
    Fmt {
        #[arg(long)]
        source: String,
    },
    /// Clippy lint
    Lint {
        #[arg(long)]
        source: String,
    },
    /// Unit tests
    Test {
        #[arg(long)]
        source: String,
    },
    /// Module lifecycle integration test
    #[command(name = "integration-test")]
    IntegrationTest {
        #[arg(long)]
        source: String,
    },
    /// Validate module manifests and XML
    #[command(name = "module-lint")]
    ModuleLint {
        #[arg(long)]
        source: String,
    },
    /// Build Tailwind CSS
    #[command(name = "tailwind-build")]
    TailwindBuild {
        #[arg(long)]
        source: String,
    },
    /// Deploy to dev server
    Deploy {
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "192.168.3.148")]
        host: String,
    },
    /// Security audit
    #[command(name = "security-audit")]
    SecurityAudit {
        #[arg(long)]
        source: String,
    },
    /// Full pipeline (check + fmt + lint + test + module-lint + integration)
    All {
        #[arg(long)]
        source: String,
    },
}

fn host_directory(client: &Query, source: &str) -> Directory {
    client.host().directory_opts(
        source,
        HostDirectoryOpts {
            exclude: Some(vec![
                "target/",
                ".git/",
                "ci/",
                "erp_web/static/node_modules/",
            ]),
            include: None,
            gitignore: None,
            no_cache: None,
        },
    )
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let Cli { command } = Cli::parse();

    dagger_sdk::connect(|client| async move {
        match command {
            Command::Check { source } => {
                let src = host_directory(&client, &source);
                let out = stages::check::run(&client, src).await?;
                println!("{out}");
            }
            Command::Fmt { source } => {
                let src = host_directory(&client, &source);
                let out = stages::fmt::run(&client, src).await?;
                println!("{out}");
            }
            Command::Lint { source } => {
                let src = host_directory(&client, &source);
                let out = stages::lint::run(&client, src).await?;
                println!("{out}");
            }
            Command::Test { source } => {
                let src = host_directory(&client, &source);
                let out = stages::test::run(&client, src).await?;
                println!("{out}");
            }
            Command::IntegrationTest { source } => {
                let src = host_directory(&client, &source);
                let out = stages::integration::run(&client, src).await?;
                println!("{out}");
            }
            Command::ModuleLint { source } => {
                let src = host_directory(&client, &source);
                let out = stages::module_lint::run(&client, src).await?;
                println!("{out}");
            }
            Command::TailwindBuild { source } => {
                let src = host_directory(&client, &source);
                let out = stages::tailwind::run(&client, src).await?;
                println!("{out}");
            }
            Command::Deploy { source, host } => {
                let src = host_directory(&client, &source);
                let out = stages::deploy::run(&client, src, &host).await?;
                println!("{out}");
            }
            Command::SecurityAudit { source } => {
                let src = host_directory(&client, &source);
                let out = stages::security::run(&client, src).await?;
                println!("{out}");
            }
            Command::All { source } => {
                let src = host_directory(&client, &source);

                println!("=== Phase 1: Fast Gates ===");
                let (check_out, fmt_out) = tokio::try_join!(
                    stages::check::run(&client, src.clone()),
                    stages::fmt::run(&client, src.clone()),
                )?;
                println!("{check_out}\n{fmt_out}");

                println!("=== Phase 2: Quality Gates ===");
                let (lint_out, test_out, mlint_out) = tokio::try_join!(
                    stages::lint::run(&client, src.clone()),
                    stages::test::run(&client, src.clone()),
                    stages::module_lint::run(&client, src.clone()),
                )?;
                println!("{lint_out}\n{test_out}\n{mlint_out}");

                println!("=== Phase 3: Integration ===");
                let int_out = stages::integration::run(&client, src).await?;
                println!("{int_out}");

                println!("\n=== Full CI Pipeline Complete ===");
            }
        }
        Ok(())
    })
    .await?;

    Ok(())
}
