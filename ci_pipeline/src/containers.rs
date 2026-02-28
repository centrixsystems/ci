use dagger_sdk::{Container, Directory, Query, Service};

/// Rust build container with Diesel/PG deps and cargo caches.
pub fn rust_base(client: &Query, source: Directory) -> Container {
    client
        .container()
        .from("rust:1.85-bookworm")
        .with_exec(vec!["apt-get", "update"])
        .with_exec(vec![
            "apt-get", "install", "-y",
            "libpq-dev", "pkg-config", "build-essential", "postgresql-client",
        ])
        .with_mounted_cache(
            "/usr/local/cargo/registry",
            client.cache_volume("cargo-registry"),
        )
        .with_mounted_cache(
            "/usr/local/cargo/git",
            client.cache_volume("cargo-git"),
        )
        .with_mounted_cache(
            "/app/target",
            client.cache_volume("cargo-target"),
        )
        .with_workdir("/app")
        .with_directory("/app", source)
        .with_env_variable("CARGO_TARGET_DIR", "/app/target")
        .with_env_variable("RUST_BACKTRACE", "1")
}

/// PostgreSQL 18 service for integration tests.
pub fn postgres(client: &Query) -> Service {
    client
        .container()
        .from("postgres:18-alpine")
        .with_env_variable("POSTGRES_DB", "erp_test")
        .with_env_variable("POSTGRES_USER", "erp")
        .with_env_variable("POSTGRES_PASSWORD", "erp_password")
        .with_exposed_port(5432)
        .as_service()
}

/// Node 22 container for frontend builds.
pub fn node_base(client: &Query, static_dir: Directory) -> Container {
    client
        .container()
        .from("node:22-slim")
        .with_mounted_cache("/app/node_modules", client.cache_volume("npm-cache"))
        .with_workdir("/app")
        .with_directory("/app", static_dir)
}
