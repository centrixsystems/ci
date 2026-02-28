//! CiModule — implements the centrix Module trait for the CI platform.

use async_trait::async_trait;
use diesel_async::AsyncPgConnection;
use diesel_async::SimpleAsyncConnection;

use erp_core::modules::{Module, ModuleInfo, ModuleResult};
use erp_core::orm::{Environment, ModelHandlerRegistry, ModelSourcingRegistry};

/// SQL migration for CI platform tables.
///
/// Creates all 8 tables for the generic CI platform with tenant_id for RLS.
pub const MIGRATION_SQL: &str = r#"
-- ================================================================
-- CI Platform Tables (generic, pipeline-agnostic)
-- ================================================================

CREATE TABLE IF NOT EXISTS ci_projects (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    name            VARCHAR(255) NOT NULL,
    github_repo     VARCHAR(255) NOT NULL UNIQUE,
    default_branch  VARCHAR(255) NOT NULL DEFAULT 'main',
    pipeline_config JSONB,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_projects_repo ON ci_projects (github_repo);
CREATE INDEX IF NOT EXISTS idx_ci_projects_tenant ON ci_projects (tenant_id);

CREATE TABLE IF NOT EXISTS ci_triggers (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    project_id      BIGINT NOT NULL REFERENCES ci_projects(id) ON DELETE CASCADE,
    event_type      VARCHAR(32) NOT NULL,
    branch_pattern  VARCHAR(255),
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ci_builds (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    project_id      BIGINT NOT NULL REFERENCES ci_projects(id) ON DELETE CASCADE,
    commit_sha      VARCHAR(40) NOT NULL,
    branch          VARCHAR(255) NOT NULL,
    pr_number       INTEGER,
    author          VARCHAR(255),
    message         TEXT,
    fingerprint     VARCHAR(64) NOT NULL,
    trigger_event   VARCHAR(32) NOT NULL,
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    duration_ms     INTEGER,
    summary         JSONB,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_builds_fingerprint ON ci_builds (fingerprint);
CREATE INDEX IF NOT EXISTS idx_ci_builds_branch ON ci_builds (branch);
CREATE INDEX IF NOT EXISTS idx_ci_builds_status ON ci_builds (status);
CREATE INDEX IF NOT EXISTS idx_ci_builds_created ON ci_builds (create_date DESC);
CREATE INDEX IF NOT EXISTS idx_ci_builds_project ON ci_builds (project_id);
CREATE INDEX IF NOT EXISTS idx_ci_builds_tenant ON ci_builds (tenant_id);

CREATE TABLE IF NOT EXISTS ci_build_steps (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    build_id        BIGINT NOT NULL REFERENCES ci_builds(id) ON DELETE CASCADE,
    name            VARCHAR(64) NOT NULL,
    sequence        INTEGER NOT NULL DEFAULT 0,
    status          VARCHAR(32) NOT NULL DEFAULT 'pending',
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    duration_ms     INTEGER,
    exit_code       INTEGER,
    stdout          TEXT,
    stderr          TEXT,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_build_steps_build ON ci_build_steps (build_id);

CREATE TABLE IF NOT EXISTS ci_environments (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    project_id      BIGINT NOT NULL REFERENCES ci_projects(id) ON DELETE CASCADE,
    build_id        BIGINT REFERENCES ci_builds(id),
    pr_number       INTEGER NOT NULL,
    branch          VARCHAR(255) NOT NULL,
    commit_sha      VARCHAR(40) NOT NULL,
    status          VARCHAR(32) NOT NULL DEFAULT 'requested',
    url             VARCHAR(512),
    last_activity   TIMESTAMPTZ,
    idle_timeout_min INTEGER NOT NULL DEFAULT 60,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_environments_status ON ci_environments (status);
CREATE INDEX IF NOT EXISTS idx_ci_environments_project ON ci_environments (project_id);
CREATE INDEX IF NOT EXISTS idx_ci_environments_tenant ON ci_environments (tenant_id);

CREATE TABLE IF NOT EXISTS ci_errors (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    project_id      BIGINT REFERENCES ci_projects(id),
    fingerprint     VARCHAR(64) NOT NULL,
    category        VARCHAR(32) NOT NULL,
    severity        VARCHAR(16) NOT NULL DEFAULT 'error',
    title           VARCHAR(500) NOT NULL,
    file_path       VARCHAR(500),
    line_number     INTEGER,
    first_seen_at   TIMESTAMPTZ NOT NULL,
    last_seen_at    TIMESTAMPTZ NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    status          VARCHAR(16) NOT NULL DEFAULT 'open',
    assigned_to     VARCHAR(255),
    notes           TEXT,
    raw_text        TEXT NOT NULL,
    normalized_text TEXT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_errors_fingerprint ON ci_errors (fingerprint);
CREATE INDEX IF NOT EXISTS idx_ci_errors_status ON ci_errors (status);
CREATE INDEX IF NOT EXISTS idx_ci_errors_tenant ON ci_errors (tenant_id);

CREATE TABLE IF NOT EXISTS ci_error_occurrences (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    error_id        BIGINT NOT NULL REFERENCES ci_errors(id) ON DELETE CASCADE,
    build_id        BIGINT NOT NULL REFERENCES ci_builds(id) ON DELETE CASCADE,
    step_name       VARCHAR(64) NOT NULL,
    raw_output      TEXT,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ci_artifacts (
    id              BIGSERIAL PRIMARY KEY,
    tenant_id       UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    build_id        BIGINT NOT NULL REFERENCES ci_builds(id) ON DELETE CASCADE,
    name            VARCHAR(255) NOT NULL,
    artifact_type   VARCHAR(32) NOT NULL,
    content         TEXT,
    size_bytes      BIGINT,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    create_uid      BIGINT,
    create_date     TIMESTAMPTZ DEFAULT NOW(),
    write_uid       BIGINT,
    write_date      TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ci_artifacts_build ON ci_artifacts (build_id);
"#;

/// Run CI platform migration.
pub async fn run_migration(conn: &mut AsyncPgConnection) -> anyhow::Result<()> {
    conn.batch_execute(MIGRATION_SQL)
        .await
        .map_err(|e| anyhow::anyhow!("CI migration failed: {e}"))?;
    Ok(())
}

/// CI module for framework registration.
pub struct CiModule;

impl CiModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Module for CiModule {
    fn info(&self) -> ModuleInfo {
        ModuleInfo {
            name: "ci".to_string(),
            version: "1.0.0".to_string(),
            summary: "CI/CD Management Platform".to_string(),
            description: "Generic CI management platform for any GitHub project".to_string(),
            depends: vec!["base".to_string(), "mail".to_string()],
            auto_install: false,
            category: "DevOps".to_string(),
        }
    }

    fn init(&self, _env: &Environment) -> ModuleResult<()> {
        tracing::info!("Initializing CI module");
        Ok(())
    }

    fn post_init(&self, _env: &Environment) -> ModuleResult<()> {
        tracing::info!("CI module initialized");
        Ok(())
    }

    async fn register_handlers(
        &self,
        _registry: &ModelHandlerRegistry,
        model_sourcing: &ModelSourcingRegistry,
    ) {
        // CI models are infrastructure (DirectCrud, no event sourcing)
        let direct_crud_models = [
            "ci.project",
            "ci.trigger",
            "ci.build",
            "ci.build.step",
            "ci.environment",
            "ci.error",
            "ci.error.occurrence",
            "ci.artifact",
        ];

        for model in direct_crud_models {
            model_sourcing.register_direct_crud(model).await;
        }

        tracing::info!(
            "CI module: registered {} DirectCrud models",
            direct_crud_models.len()
        );
    }
}
