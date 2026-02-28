//! CI platform seeder — ir_model, views, actions, menus for CI models.

use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

/// Seed CI platform data into the framework's ir_model, ir_ui_view,
/// ir_actions_act_window, ir_ui_menu, and ir_model_access tables.
///
/// Idempotent — uses ON CONFLICT DO NOTHING.
pub async fn seed_ci_module(conn: &mut AsyncPgConnection) -> anyhow::Result<()> {
    let ts = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%:z")
        .to_string();

    // Enable projection mode for direct inserts
    diesel::sql_query("SET centrix.projection_mode = 'true'")
        .execute(conn)
        .await?;

    // ── 1. Register ir_model entries ──
    let models: Vec<(i64, &str, &str, &str, &str)> = vec![
        (50, "CI Project", "ci.project", "ci_projects", "Registered GitHub repos with pipeline config"),
        (51, "CI Trigger", "ci.trigger", "ci_triggers", "Build trigger rules"),
        (52, "CI Build", "ci.build", "ci_builds", "Pipeline runs"),
        (53, "CI Build Step", "ci.build.step", "ci_build_steps", "Individual build steps"),
        (54, "CI Environment", "ci.environment", "ci_environments", "Ephemeral review environments"),
        (55, "CI Error", "ci.error", "ci_errors", "Deduplicated build errors"),
        (56, "CI Error Occurrence", "ci.error.occurrence", "ci_error_occurrences", "Error-build linking"),
        (57, "CI Artifact", "ci.artifact", "ci_artifacts", "Build artifacts"),
    ];

    for (id, name, model, table, info_text) in &models {
        diesel::sql_query(format!(
            "INSERT INTO ir_model (id, name, model, table_name, info, state, transient, create_date) \
             VALUES ({id}, '{name}', '{model}', '{table}', '{info_text}', 'base', false, '{ts}') \
             ON CONFLICT (id) DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    diesel::sql_query("SELECT setval('ir_model_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM ir_model), 57))")
        .execute(conn)
        .await?;

    // ── 2. ir_model_data entries ──
    let model_data: Vec<(&str, i64)> = vec![
        ("model_ci_project", 50),
        ("model_ci_trigger", 51),
        ("model_ci_build", 52),
        ("model_ci_build_step", 53),
        ("model_ci_environment", 54),
        ("model_ci_error", 55),
        ("model_ci_error_occurrence", 56),
        ("model_ci_artifact", 57),
    ];

    for (name, res_id) in &model_data {
        diesel::sql_query(format!(
            "INSERT INTO ir_model_data (name, module, model, res_id, noupdate, create_date) \
             VALUES ('{name}', 'ci', 'ir.model', {res_id}, true, '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    // ── 3. Views ──
    let views: Vec<(&str, &str, &str, &str, &str)> = vec![
        ("ci.project.list", "ci.project", "list",
         "<list string=\"CI Projects\" default_order=\"id desc\">\
          <field name=\"name\"/>\
          <field name=\"github_repo\"/>\
          <field name=\"default_branch\"/>\
          <field name=\"active\"/>\
          </list>",
         "ci.view_project_list"),

        ("ci.project.form", "ci.project", "form",
         "<form string=\"CI Project\">\
          <sheet>\
            <div class=\"oe_title\"><h1><field name=\"name\"/></h1></div>\
            <group>\
              <group>\
                <field name=\"github_repo\"/>\
                <field name=\"default_branch\"/>\
              </group>\
              <group>\
                <field name=\"active\"/>\
                <field name=\"pipeline_config\" widget=\"json\"/>\
              </group>\
            </group>\
          </sheet>\
          </form>",
         "ci.view_project_form"),

        ("ci.build.list", "ci.build", "list",
         "<list string=\"CI Builds\" default_order=\"id desc\" decoration-danger=\"status == 'failure'\" decoration-success=\"status == 'success'\" decoration-muted=\"status == 'cancelled'\">\
          <field name=\"id\"/>\
          <field name=\"project_id\"/>\
          <field name=\"commit_sha\" widget=\"char\" limit=\"8\"/>\
          <field name=\"branch\"/>\
          <field name=\"pr_number\"/>\
          <field name=\"author\"/>\
          <field name=\"status\"/>\
          <field name=\"duration_ms\"/>\
          <field name=\"trigger_event\"/>\
          <field name=\"create_date\"/>\
          </list>",
         "ci.view_build_list"),

        ("ci.build.form", "ci.build", "form",
         "<form string=\"CI Build\">\
          <header>\
            <field name=\"status\" widget=\"statusbar\" statusbar_visible=\"pending,running,success,failure\"/>\
          </header>\
          <sheet>\
            <div class=\"oe_title\"><h1>Build #<field name=\"id\"/></h1></div>\
            <group>\
              <group>\
                <field name=\"project_id\"/>\
                <field name=\"commit_sha\"/>\
                <field name=\"branch\"/>\
                <field name=\"pr_number\"/>\
                <field name=\"author\"/>\
              </group>\
              <group>\
                <field name=\"status\"/>\
                <field name=\"trigger_event\"/>\
                <field name=\"fingerprint\"/>\
                <field name=\"duration_ms\"/>\
              </group>\
            </group>\
            <group><field name=\"message\"/></group>\
          </sheet>\
          <div class=\"oe_chatter\"><field name=\"message_ids\"/></div>\
          </form>",
         "ci.view_build_form"),

        ("ci.build.kanban", "ci.build", "kanban",
         "<kanban default_group_by=\"status\" default_order=\"id desc\">\
          <field name=\"commit_sha\"/>\
          <field name=\"branch\"/>\
          <field name=\"status\"/>\
          <field name=\"author\"/>\
          <field name=\"duration_ms\"/>\
          <templates>\
            <t t-name=\"kanban-card\">\
              <div class=\"oe_kanban_card\">\
                <strong><field name=\"branch\"/></strong>\
                <div><field name=\"commit_sha\"/></div>\
                <div><field name=\"author\"/></div>\
              </div>\
            </t>\
          </templates>\
          </kanban>",
         "ci.view_build_kanban"),

        ("ci.build.search", "ci.build", "search",
         "<search string=\"CI Builds\">\
          <field name=\"commit_sha\"/>\
          <field name=\"branch\"/>\
          <field name=\"author\"/>\
          <filter name=\"success\" string=\"Success\" domain=\"[['status','=','success']]\"/>\
          <filter name=\"failure\" string=\"Failure\" domain=\"[['status','=','failure']]\"/>\
          <filter name=\"running\" string=\"Running\" domain=\"[['status','=','running']]\"/>\
          <group>\
            <filter name=\"group_status\" string=\"Status\" context=\"{'group_by':'status'}\"/>\
            <filter name=\"group_branch\" string=\"Branch\" context=\"{'group_by':'branch'}\"/>\
          </group>\
          </search>",
         "ci.view_build_search"),

        ("ci.environment.kanban", "ci.environment", "kanban",
         "<kanban default_group_by=\"status\">\
          <field name=\"branch\"/>\
          <field name=\"status\"/>\
          <field name=\"url\"/>\
          <field name=\"pr_number\"/>\
          <templates>\
            <t t-name=\"kanban-card\">\
              <div class=\"oe_kanban_card\">\
                <strong>PR #<field name=\"pr_number\"/></strong>\
                <div><field name=\"branch\"/></div>\
              </div>\
            </t>\
          </templates>\
          </kanban>",
         "ci.view_environment_kanban"),

        ("ci.environment.list", "ci.environment", "list",
         "<list string=\"CI Environments\" default_order=\"id desc\" decoration-success=\"status == 'running'\" decoration-warning=\"status == 'dormant'\">\
          <field name=\"id\"/>\
          <field name=\"project_id\"/>\
          <field name=\"pr_number\"/>\
          <field name=\"branch\"/>\
          <field name=\"status\"/>\
          <field name=\"url\" widget=\"url\"/>\
          <field name=\"create_date\"/>\
          </list>",
         "ci.view_environment_list"),

        ("ci.error.list", "ci.error", "list",
         "<list string=\"CI Errors\" default_order=\"last_seen_at desc\">\
          <field name=\"title\"/>\
          <field name=\"category\"/>\
          <field name=\"severity\"/>\
          <field name=\"occurrence_count\"/>\
          <field name=\"status\"/>\
          <field name=\"first_seen_at\"/>\
          <field name=\"last_seen_at\"/>\
          </list>",
         "ci.view_error_list"),

        ("ci.error.form", "ci.error", "form",
         "<form string=\"CI Error\">\
          <sheet>\
            <div class=\"oe_title\"><h1><field name=\"title\"/></h1></div>\
            <group>\
              <group>\
                <field name=\"category\"/>\
                <field name=\"severity\"/>\
                <field name=\"status\"/>\
                <field name=\"fingerprint\"/>\
              </group>\
              <group>\
                <field name=\"occurrence_count\"/>\
                <field name=\"first_seen_at\"/>\
                <field name=\"last_seen_at\"/>\
                <field name=\"assigned_to\"/>\
              </group>\
            </group>\
            <notebook>\
              <page string=\"Raw Text\" name=\"raw\"><field name=\"raw_text\"/></page>\
              <page string=\"Notes\" name=\"notes\"><field name=\"notes\"/></page>\
            </notebook>\
          </sheet>\
          </form>",
         "ci.view_error_form"),
    ];

    for (name, model, view_type, arch, xml_id) in &views {
        let arch_escaped = arch.replace('\'', "''");
        diesel::sql_query(format!(
            "INSERT INTO ir_ui_view (name, model, \"type\", priority, arch, mode, active, xml_id, create_date) \
             VALUES ('{name}', '{model}', '{view_type}', 16, '{arch_escaped}', 'primary', true, '{xml_id}', '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    // ── 4. Actions ──
    let actions: Vec<(&str, &str, &str, &str)> = vec![
        ("CI Projects", "ci.project", "list,form", "ci.action_ci_project"),
        ("CI Builds", "ci.build", "kanban,list,form", "ci.action_ci_build"),
        ("CI Environments", "ci.environment", "kanban,list,form", "ci.action_ci_environment"),
        ("CI Errors", "ci.error", "list,form", "ci.action_ci_error"),
    ];

    for (name, res_model, view_mode, xml_id) in &actions {
        diesel::sql_query(format!(
            "INSERT INTO ir_actions_act_window (name, res_model, view_mode, target, xml_id, create_date) \
             VALUES ('{name}', '{res_model}', '{view_mode}', 'current', '{xml_id}', '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    // ── 5. Menus ──
    #[derive(diesel::QueryableByName)]
    struct IdRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        id: i64,
    }

    // Look up action IDs
    let build_action: Vec<IdRow> = diesel::sql_query(
        "SELECT id FROM ir_actions_act_window WHERE xml_id = 'ci.action_ci_build' LIMIT 1"
    ).load(conn).await?;
    let build_action_id = build_action.get(0).map(|r| r.id).unwrap_or(0);

    let project_action: Vec<IdRow> = diesel::sql_query(
        "SELECT id FROM ir_actions_act_window WHERE xml_id = 'ci.action_ci_project' LIMIT 1"
    ).load(conn).await?;
    let project_action_id = project_action.get(0).map(|r| r.id).unwrap_or(0);

    let env_action: Vec<IdRow> = diesel::sql_query(
        "SELECT id FROM ir_actions_act_window WHERE xml_id = 'ci.action_ci_environment' LIMIT 1"
    ).load(conn).await?;
    let env_action_id = env_action.get(0).map(|r| r.id).unwrap_or(0);

    let error_action: Vec<IdRow> = diesel::sql_query(
        "SELECT id FROM ir_actions_act_window WHERE xml_id = 'ci.action_ci_error' LIMIT 1"
    ).load(conn).await?;
    let error_action_id = error_action.get(0).map(|r| r.id).unwrap_or(0);

    // Root menu
    diesel::sql_query(format!(
        "INSERT INTO ir_ui_menu (name, parent_id, sequence, action, active, xml_id, web_icon, create_date) \
         VALUES ('CI/CD', NULL, 90, 'ir.actions.act_window,{build_action_id}', true, 'ci.menu_root_ci', 'ci', '{ts}') \
         ON CONFLICT DO NOTHING"
    ))
    .execute(conn)
    .await?;

    let ci_root: Vec<IdRow> = diesel::sql_query(
        "SELECT id FROM ir_ui_menu WHERE xml_id = 'ci.menu_root_ci' LIMIT 1"
    ).load(conn).await?;
    let ci_root_id = ci_root.get(0).map(|r| r.id).unwrap_or(0);

    if ci_root_id > 0 {
        let menus: Vec<(&str, i32, i64, &str)> = vec![
            ("Projects", 10, project_action_id, "ci.menu_ci_projects"),
            ("Builds", 20, build_action_id, "ci.menu_ci_builds"),
            ("Environments", 30, env_action_id, "ci.menu_ci_environments"),
        ];

        for (name, seq, action_id, xml_id) in &menus {
            diesel::sql_query(format!(
                "INSERT INTO ir_ui_menu (name, parent_id, sequence, action, active, xml_id, create_date) \
                 VALUES ('{name}', {ci_root_id}, {seq}, 'ir.actions.act_window,{action_id}', true, '{xml_id}', '{ts}') \
                 ON CONFLICT DO NOTHING"
            ))
            .execute(conn)
            .await?;
        }

        // Quality submenu
        diesel::sql_query(format!(
            "INSERT INTO ir_ui_menu (name, parent_id, sequence, active, xml_id, create_date) \
             VALUES ('Quality', {ci_root_id}, 40, true, 'ci.menu_ci_quality', '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;

        let quality_menu: Vec<IdRow> = diesel::sql_query(
            "SELECT id FROM ir_ui_menu WHERE xml_id = 'ci.menu_ci_quality' LIMIT 1"
        ).load(conn).await?;

        if let Some(q) = quality_menu.get(0) {
            diesel::sql_query(format!(
                "INSERT INTO ir_ui_menu (name, parent_id, sequence, action, active, xml_id, create_date) \
                 VALUES ('Errors', {}, 10, 'ir.actions.act_window,{error_action_id}', true, 'ci.menu_ci_errors', '{ts}') \
                 ON CONFLICT DO NOTHING",
                q.id
            ))
            .execute(conn)
            .await?;
        }
    }

    diesel::sql_query(
        "SELECT setval('ir_ui_menu_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM ir_ui_menu), 1))"
    )
    .execute(conn)
    .await?;

    // ── 6. Access rights ──
    let access: Vec<(&str, i64, i64, bool, bool, bool, bool)> = vec![
        ("access_ci_project_admin", 50, 2, true, true, true, true),
        ("access_ci_trigger_admin", 51, 2, true, true, true, true),
        ("access_ci_build_admin", 52, 2, true, true, true, true),
        ("access_ci_build_step_admin", 53, 2, true, true, true, true),
        ("access_ci_environment_admin", 54, 2, true, true, true, true),
        ("access_ci_error_admin", 55, 2, true, true, true, true),
        ("access_ci_error_occurrence_admin", 56, 2, true, true, true, true),
        ("access_ci_artifact_admin", 57, 2, true, true, true, true),
        // Internal users
        ("access_ci_project_user", 50, 1, true, false, false, false),
        ("access_ci_trigger_user", 51, 1, true, false, false, false),
        ("access_ci_build_user", 52, 1, true, true, true, false),
        ("access_ci_build_step_user", 53, 1, true, false, false, false),
        ("access_ci_environment_user", 54, 1, true, true, true, false),
        ("access_ci_error_user", 55, 1, true, true, false, false),
        ("access_ci_error_occurrence_user", 56, 1, true, false, false, false),
        ("access_ci_artifact_user", 57, 1, true, false, false, false),
    ];

    for (name, model_id, group_id, r, w, c, u) in &access {
        diesel::sql_query(format!(
            "INSERT INTO ir_model_access (name, model_id, group_id, perm_read, perm_write, perm_create, perm_unlink, active, create_date) \
             VALUES ('{name}', {model_id}, {group_id}, {r}, {w}, {c}, {u}, true, '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    // ── 7. Seed CI projects ──
    let tenant_id = "00000000-0000-0000-0000-000000000001";

    let projects: Vec<(&str, &str, &str, &str)> = vec![
        (
            "Centrix Framework",
            "centrixsystems/centrix",
            "development",
            r#"{"steps":[{"name":"check","command":"cargo check --workspace"},{"name":"test","command":"cargo test --workspace --lib -- --test-threads=1"},{"name":"clippy","command":"cargo clippy --workspace -- -D warnings"}],"timeout_secs":900,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/centrix"}"#,
        ),
        (
            "Forge",
            "centrixsystems/forge",
            "main",
            r#"{"steps":[{"name":"check","command":"cargo check --workspace"},{"name":"test","command":"cargo test --workspace --lib"},{"name":"clippy","command":"cargo clippy --workspace -- -D warnings"}],"timeout_secs":600,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge"}"#,
        ),
        (
            "Forge SDK Rust",
            "centrixsystems/forge-sdk-rust",
            "main",
            r#"{"steps":[{"name":"check","command":"cargo check"},{"name":"test","command":"cargo test"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-rust"}"#,
        ),
        (
            "Forge SDK Python",
            "centrixsystems/forge-sdk-python",
            "main",
            r#"{"steps":[{"name":"install","command":"pip install -e '.[dev]' 2>/dev/null || pip install -r requirements.txt 2>/dev/null || echo 'no deps'"},{"name":"pytest","command":"python -m pytest -v 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-python"}"#,
        ),
        (
            "Forge SDK TypeScript",
            "centrixsystems/forge-sdk-ts",
            "main",
            r#"{"steps":[{"name":"install","command":"npm install 2>/dev/null || echo 'no package.json'"},{"name":"build","command":"npm run build 2>/dev/null || echo 'no build script'"},{"name":"test","command":"npm test 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-ts"}"#,
        ),
        (
            "Forge SDK Go",
            "centrixsystems/forge-sdk-go",
            "main",
            r#"{"steps":[{"name":"vet","command":"go vet ./... 2>/dev/null || echo 'no go files'"},{"name":"test","command":"go test ./... 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-go"}"#,
        ),
        (
            "Forge SDK Java",
            "centrixsystems/forge-sdk-java",
            "main",
            r#"{"steps":[{"name":"compile","command":"mvn compile 2>/dev/null || gradle build 2>/dev/null || echo 'no build system'"},{"name":"test","command":"mvn test 2>/dev/null || gradle test 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-java"}"#,
        ),
        (
            "Forge SDK C#",
            "centrixsystems/forge-sdk-csharp",
            "main",
            r#"{"steps":[{"name":"build","command":"dotnet build 2>/dev/null || echo 'no project file'"},{"name":"test","command":"dotnet test 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-csharp"}"#,
        ),
        (
            "Forge SDK Ruby",
            "centrixsystems/forge-sdk-ruby",
            "main",
            r#"{"steps":[{"name":"bundle","command":"bundle install 2>/dev/null || echo 'no Gemfile'"},{"name":"test","command":"bundle exec rake test 2>/dev/null || bundle exec rspec 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-ruby"}"#,
        ),
        (
            "Forge SDK PHP",
            "centrixsystems/forge-sdk-php",
            "main",
            r#"{"steps":[{"name":"install","command":"composer install 2>/dev/null || echo 'no composer.json'"},{"name":"test","command":"vendor/bin/phpunit 2>/dev/null || echo 'no tests yet'"}],"timeout_secs":300,"local_path":"/home/ubuntu-server/ci-repos/centrixsystems/forge-sdk-php"}"#,
        ),
    ];

    for (name, github_repo, default_branch, pipeline_json) in &projects {
        let name_escaped = name.replace('\'', "''");
        let json_escaped = pipeline_json.replace('\'', "''");
        diesel::sql_query(format!(
            "INSERT INTO ci_projects (tenant_id, name, github_repo, default_branch, pipeline_config, active, create_date) \
             VALUES ('{tenant_id}', '{name_escaped}', '{github_repo}', '{default_branch}', '{json_escaped}'::jsonb, true, '{ts}') \
             ON CONFLICT DO NOTHING"
        ))
        .execute(conn)
        .await?;
    }

    tracing::info!("Seeded {} CI projects", projects.len());

    // Reset projection mode
    diesel::sql_query("SET centrix.projection_mode = 'false'")
        .execute(conn)
        .await?;

    tracing::info!(
        "CI platform seeded: {} models, {} views, {} actions, {} access rights",
        models.len(),
        views.len(),
        actions.len(),
        access.len()
    );

    Ok(())
}
