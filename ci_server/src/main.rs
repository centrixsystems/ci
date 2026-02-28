//! Centrix CI Server — generic CI management platform.
//!
//! A standalone binary that manages CI/CD pipelines from any GitHub repo.
//! Built on the Centrix framework (ORM, views, event sourcing, DBOS).
//!
//! Individual projects own their pipeline definitions (Dagger modules).
//! This platform handles: project registration, webhook reception,
//! build triggering/tracking, dashboard/UI, and observability.

mod ci_module;
mod config;
mod dashboard;
mod events;
mod handlers;
mod metrics;
mod models;
mod routes;
mod schema;
mod seeder;
mod services;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use clap::Parser;
use erp_bus::{create_router as create_bus_router, MemoryStore};
use erp_core::db::{DataServices, DatabaseConfig};
use erp_core::modules::ModuleLoader;
use erp_core::orm::events::projections::GenericProjection;
use erp_core::orm::events::replay::ReplayEngine;
use erp_core::orm::ModelHandlerRegistry;
use erp_dbos::{DbosConfig, DbosRuntime};
use erp_web::session::{spawn_vacuum_task, SessionStore};
use erp_web::{create_app, AppState, AttachmentStoreConfig, DevMode};

#[derive(Parser)]
#[command(name = "centrix-ci", about = "Centrix CI Management Platform")]
struct Cli {
    /// Server port
    #[arg(short, long, env = "CI_PORT", default_value = "9090")]
    port: u16,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Enable dev mode features
    #[arg(long, default_value = "")]
    dev: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_default();
    if log_format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .init();
    }

    let cli = Cli::parse();
    let dev_mode = DevMode::from_features(&cli.dev);

    tracing::info!("Starting Centrix CI Server...");

    // Database connection
    let db_url = cli
        .database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "postgres://erp:erp_password@localhost:5433/erp".to_string());

    let db_config = DatabaseConfig::from_env(&db_url);
    let data_services = DataServices::new(&db_config).await?;

    // Run framework migrations
    {
        let mut conn = data_services
            .diesel
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
        tracing::info!("Running database migrations...");
        erp_migration::run_migrations(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        tracing::info!("Database migrations completed.");
    }

    // Run CI-specific migration (creates ci_* tables)
    {
        let mut conn = data_services
            .diesel
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
        tracing::info!("Running CI module migration...");
        ci_module::run_migration(&mut conn).await?;
        tracing::info!("CI module migration completed.");
    }

    // Register projections
    {
        let mut projections = data_services.projection_registry.write().await;
        projections.register(Box::new(GenericProjection));
        tracing::info!("Registered {} event projections", projections.len());
    }

    // Seed base data if needed
    let is_seeded = {
        let mut conn = data_services
            .diesel
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
        erp_migration::seeder::is_seeded(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
    };

    if !is_seeded {
        // Run phase 1 + phase 2 seeding
        {
            let mut conn = data_services
                .diesel
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
            erp_migration::seeder::seed_phase1(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        // Replay events
        {
            let replay = ReplayEngine::new(data_services.projection_registry.clone());
            let mut conn = data_services
                .diesel
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
            replay
                .replay_unprocessed(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("replay: {e}"))?;
        }

        {
            let mut conn = data_services
                .diesel
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
            erp_migration::seeder::seed_phase2a(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }

        {
            let replay = ReplayEngine::new(data_services.projection_registry.clone());
            let mut conn = data_services
                .diesel
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
            replay
                .replay_unprocessed(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("replay: {e}"))?;
        }

        {
            let mut conn = data_services
                .diesel
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
            erp_migration::seeder::seed_phase2b(&mut conn)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }

    // Seed CI-specific data (models, views, actions, menus)
    {
        let mut conn = data_services
            .diesel
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("diesel pool: {e}"))?;
        seeder::seed_ci_module(&mut conn).await?;
    }

    // Load modules
    let handler_registry = ModelHandlerRegistry::new();
    let mut loader = ModuleLoader::new(&data_services.diesel);
    loader.register_module(erp_base::BaseModule::new());
    loader.register_module(erp_mail::MailModule::new());
    loader.register_module(ci_module::CiModule::new());
    let _load_result = loader
        .load(&handler_registry, &data_services.model_sourcing)
        .await?;
    let handler_registry = Arc::new(handler_registry);

    // DBOS runtime
    let dbos_config = DbosConfig::new(&db_url)
        .with_application_id("centrix-ci")
        .with_application_version(env!("CARGO_PKG_VERSION"))
        .with_max_connections(5);
    let dbos_runtime = DbosRuntime::new(dbos_config).await?;
    dbos_runtime.initialize_event_store().await?;
    dbos_runtime.start().await?;
    let dbos_runtime_handle = Arc::new(dbos_runtime);

    // Session store
    let session_store = Arc::new(SessionStore::new());
    let data_arc = Arc::new(data_services);

    spawn_vacuum_task(session_store.clone());

    // Bus store
    let bus_store = MemoryStore::shared();

    // Flux template engine (minimal for CI)
    let flux_engine = Arc::new(erp_core::ir::FluxTemplateEngine::new());

    // CI router state
    let ci_config = config::CiConfig::from_env();
    let ci_state = routes::CiRouterState {
        pool: data_arc.diesel.clone(),
        config: ci_config,
    };

    // App state (for framework web client)
    let state = AppState {
        data: data_arc,
        session_store,
        totp_challenge_store: Arc::new(erp_web::session::TotpChallengeStore::new()),
        handler_registry,
        dbos_runtime: Some(dbos_runtime_handle.clone()),
        dev_mode,
        started_at: std::time::Instant::now(),
        attachment_config: AttachmentStoreConfig {
            store: "filesystem".to_string(),
            path: "./filestore".to_string(),
            max_size_bytes: 10 * 1024 * 1024,
        },
        bus_store: Some(bus_store.clone()),
        flux_engine,
        forge: None,
    };

    // Build routers
    let web_app = create_app(state);
    let ci_router = routes::ci_router(ci_state);
    let bus_router = create_bus_router(bus_store);

    let app = Router::new()
        .merge(web_app)
        .nest("/ci", ci_router)
        .nest("/bus", bus_router);

    // Initialize metrics
    metrics::init_metrics();

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    tracing::info!("Centrix CI Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("Stopping DBOS runtime...");
    dbos_runtime_handle.stop().await;
    tracing::info!("Shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received SIGINT, shutting down..."),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down..."),
    }
}
