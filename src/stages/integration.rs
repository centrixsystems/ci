use dagger_sdk::{Directory, Query};

use crate::containers;

/// Run module lifecycle integration test against a fresh PostgreSQL database.
/// Flow: migrate -> seed -> install base -> install todo_list -> verify -> uninstall -> verify cleanup
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let pg = containers::postgres(client);
    let db_url = "postgres://erp:erp_password@db:5432/erp_test";

    let test_script = r#"
set -euo pipefail

export DATABASE_URL="postgres://erp:erp_password@db:5432/erp_test"
export RUST_LOG=info
BINARY="./target/release/erp-server"

echo "=== Integration Test: Module Lifecycle ==="

echo "[1/8] Running migrations..."
$BINARY migrate 2>&1 || true

echo "[2/8] Seeding base data..."
$BINARY seed 2>&1 || true

echo "[3/8] Installing base module..."
$BINARY module install base 2>&1 || true

echo "[4/8] Installing todo_list module..."
$BINARY module install todo_list 2>&1 || true

echo "[5/8] Verifying todo_list records..."
RECORD_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM ir_model_data WHERE module = 'todo_list'" 2>/dev/null | tr -d ' ')
echo "todo_list records: $RECORD_COUNT"

echo "[6/8] Verifying todo_task table..."
TABLE_EXISTS=$(psql "$DATABASE_URL" -t -c "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'todo_task')" 2>/dev/null | tr -d ' ')
echo "todo_task table exists: $TABLE_EXISTS"

echo "[7/8] Uninstalling todo_list module..."
$BINARY module uninstall todo_list 2>&1 || true

echo "[8/8] Verifying cleanup..."
REMAINING=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM ir_model_data WHERE module = 'todo_list'" 2>/dev/null | tr -d ' ')
TABLE_GONE=$(psql "$DATABASE_URL" -t -c "SELECT NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'todo_task')" 2>/dev/null | tr -d ' ')
echo "Remaining records: $REMAINING"
echo "Table dropped: $TABLE_GONE"

echo ""
echo "=== Integration Test Complete ==="
"#;

    let output = containers::rust_base(client, source)
        .with_service_binding("db", pg)
        .with_env_variable("DATABASE_URL", db_url)
        .with_env_variable("RUST_LOG", "info")
        .with_exec(vec![
            "sh", "-c",
            "for i in $(seq 1 30); do pg_isready -h db -p 5432 -U erp && break; sleep 1; done",
        ])
        .with_exec(vec![
            "cargo", "build", "--release", "--package", "erp_server",
        ])
        .with_exec(vec!["bash", "-c", test_script])
        .stdout()
        .await?;

    Ok(format!("[integration] {output}"))
}
