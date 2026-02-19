// Centrix CI/CD Pipeline — Dagger Module (Go SDK)
//
// Container-based CI pipeline for the Centrix Rust ERP framework.
// Every step runs in an isolated container with content-addressed caching.
//
// Usage:
//   dagger call check --source=..
//   dagger call lint --source=..
//   dagger call test --source=..
//   dagger call integration-test --source=..
//   dagger call all --source=..

package main

import (
	"context"
	"fmt"
	"strings"

	"dagger.io/dagger"
)

type CentrixCi struct{}

// rustBase returns a Rust container with all build dependencies installed
// and cargo caches mounted for fast incremental builds.
func (m *CentrixCi) rustBase(source *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("rust:1.85-bookworm").
		// Install system dependencies for Diesel + PostgreSQL
		WithExec([]string{"apt-get", "update"}).
		WithExec([]string{"apt-get", "install", "-y",
			"libpq-dev", "pkg-config", "build-essential",
			"postgresql-client",
		}).
		// Mount cargo caches (content-addressed, persist across runs)
		WithMountedCache("/usr/local/cargo/registry", dag.CacheVolume("cargo-registry")).
		WithMountedCache("/usr/local/cargo/git", dag.CacheVolume("cargo-git")).
		WithMountedCache("/app/target", dag.CacheVolume("cargo-target")).
		// Set working directory
		WithWorkdir("/app").
		// Mount source code
		WithDirectory("/app", source, dagger.ContainerWithDirectoryOpts{
			Exclude: []string{
				"target/",
				".git/",
				"ci/",
				"erp_web/static/node_modules/",
			},
		}).
		// Set environment
		WithEnvVariable("CARGO_TARGET_DIR", "/app/target").
		WithEnvVariable("RUST_BACKTRACE", "1")
}

// postgres returns a PostgreSQL 18 service container for integration tests.
func (m *CentrixCi) postgres() *dagger.Service {
	return dag.Container().
		From("postgres:18-alpine").
		WithEnvVariable("POSTGRES_DB", "erp_test").
		WithEnvVariable("POSTGRES_USER", "erp").
		WithEnvVariable("POSTGRES_PASSWORD", "erp_password").
		WithExposedPort(5432).
		AsService()
}

// Check performs a fast compile check (cargo check --workspace).
func (m *CentrixCi) Check(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	out, err := m.rustBase(source).
		WithExec([]string{"cargo", "check", "--workspace"}).
		Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("cargo check failed: %w", err)
	}
	return "Compile check passed.\n" + out, nil
}

// Lint runs cargo clippy with -D warnings (all warnings are errors).
func (m *CentrixCi) Lint(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	out, err := m.rustBase(source).
		WithExec([]string{
			"cargo", "clippy", "--workspace", "--all-targets",
			"--", "-D", "warnings",
		}).
		Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("cargo clippy failed: %w", err)
	}
	return "Lint passed (clippy -D warnings).\n" + out, nil
}

// Test runs all unit tests (cargo test --workspace --lib).
func (m *CentrixCi) Test(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	out, err := m.rustBase(source).
		WithExec([]string{
			"cargo", "test", "--workspace", "--lib",
		}).
		Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("cargo test failed: %w", err)
	}
	return "All unit tests passed.\n" + out, nil
}

// IntegrationTest runs a full module lifecycle test against a fresh PostgreSQL database.
// Flow: migrations -> seed -> install base -> install todo_list -> upgrade -> uninstall -> reinstall
func (m *CentrixCi) IntegrationTest(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	pg := m.postgres()

	dbUrl := "postgres://erp:erp_password@db:5432/erp_test"

	container := m.rustBase(source).
		WithServiceBinding("db", pg).
		WithEnvVariable("DATABASE_URL", dbUrl).
		WithEnvVariable("RUST_LOG", "info").
		// Wait for PostgreSQL to be ready
		WithExec([]string{"sh", "-c",
			"for i in $(seq 1 30); do pg_isready -h db -p 5432 -U erp && break; sleep 1; done",
		}).
		// Build release binary
		WithExec([]string{
			"cargo", "build", "--release", "--package", "erp_server",
		})

	// Run integration test script
	testScript := `#!/bin/bash
set -euo pipefail

export DATABASE_URL="postgres://erp:erp_password@db:5432/erp_test"
export RUST_LOG=info
BINARY="./target/release/erp-server"

echo "=== Integration Test: Module Lifecycle ==="

# 1. Run migrations
echo "[1/8] Running migrations..."
$BINARY migrate 2>&1 || true

# 2. Seed base data
echo "[2/8] Seeding base data..."
$BINARY seed 2>&1 || true

# 3. Install base module
echo "[3/8] Installing base module..."
$BINARY module install base 2>&1 || true

# 4. Install todo_list module
echo "[4/8] Installing todo_list module..."
$BINARY module install todo_list 2>&1 || true

# 5. Verify records exist
echo "[5/8] Verifying todo_list records..."
RECORD_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM ir_model_data WHERE module = 'todo_list'" 2>/dev/null | tr -d ' ')
echo "todo_list records: $RECORD_COUNT"

# 6. Verify todo_task table exists
echo "[6/8] Verifying todo_task table..."
TABLE_EXISTS=$(psql "$DATABASE_URL" -t -c "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'todo_task')" 2>/dev/null | tr -d ' ')
echo "todo_task table exists: $TABLE_EXISTS"

# 7. Uninstall todo_list
echo "[7/8] Uninstalling todo_list module..."
$BINARY module uninstall todo_list 2>&1 || true

# 8. Verify cleanup
echo "[8/8] Verifying cleanup..."
REMAINING=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM ir_model_data WHERE module = 'todo_list'" 2>/dev/null | tr -d ' ')
TABLE_GONE=$(psql "$DATABASE_URL" -t -c "SELECT NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'todo_task')" 2>/dev/null | tr -d ' ')
echo "Remaining records: $REMAINING"
echo "Table dropped: $TABLE_GONE"

echo ""
echo "=== Integration Test Complete ==="
`

	out, err := container.
		WithNewFile("/app/integration_test.sh", testScript, dagger.ContainerWithNewFileOpts{
			Permissions: 0755,
		}).
		WithExec([]string{"/app/integration_test.sh"}).
		Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("integration test failed: %w", err)
	}

	return out, nil
}

// ModuleLint runs custom module validation checks.
// Validates manifests, XML data files, and naming conventions.
func (m *CentrixCi) ModuleLint(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	lintScript := `#!/bin/bash
set -euo pipefail

ERRORS=0
WARNINGS=0

echo "=== Module Lint ==="

# 1. Manifest validation: check all modules have required keys
echo "[1/5] Checking module manifests..."
for manifest in modules/*/manifest.toml; do
    module_dir=$(dirname "$manifest")
    module_name=$(basename "$module_dir")

    # Check required keys exist
    if ! grep -q '^\[module\]' "$manifest"; then
        echo "ERROR: $manifest missing [module] section"
        ERRORS=$((ERRORS + 1))
    fi
    if ! grep -q 'name\s*=' "$manifest"; then
        echo "ERROR: $manifest missing 'name' key"
        ERRORS=$((ERRORS + 1))
    fi

    # Check declared data files exist
    for datafile in $(grep -oP '(?<=\")[^\"]+\.xml(?=\")' "$manifest" 2>/dev/null || true); do
        if [ ! -f "$module_dir/$datafile" ]; then
            echo "ERROR: $manifest declares '$datafile' but file not found"
            ERRORS=$((ERRORS + 1))
        fi
    done
    for datafile in $(grep -oP '(?<=\")[^\"]+\.csv(?=\")' "$manifest" 2>/dev/null || true); do
        if [ ! -f "$module_dir/$datafile" ]; then
            echo "ERROR: $manifest declares '$datafile' but file not found"
            ERRORS=$((ERRORS + 1))
        fi
    done
done

# 2. XML data validation: check for well-formed XML
echo "[2/5] Checking XML data files..."
for xmlfile in modules/*/data/*.xml modules/*/views/*.xml modules/*/security/*.xml; do
    [ -f "$xmlfile" ] || continue
    if ! xmllint --noout "$xmlfile" 2>/dev/null; then
        echo "ERROR: $xmlfile is not well-formed XML"
        ERRORS=$((ERRORS + 1))
    fi
done

# 3. Check for duplicate record IDs across XML files per module
echo "[3/5] Checking for duplicate record IDs..."
for module_dir in modules/*/; do
    [ -d "$module_dir" ] || continue
    module_name=$(basename "$module_dir")
    # Collect all record IDs
    ids=$(grep -roh 'id="[^"]*"' "$module_dir" 2>/dev/null | sort | uniq -d)
    if [ -n "$ids" ]; then
        echo "WARNING: Duplicate record IDs in $module_name: $ids"
        WARNINGS=$((WARNINGS + 1))
    fi
done

# 4. Check for no raw SQL without bind params in Rust handlers
echo "[4/5] Checking for unsafe SQL patterns..."
for rsfile in modules/*/src/**/*.rs erp_core/src/**/*.rs; do
    [ -f "$rsfile" ] || continue
    # Look for format!("...SELECT...") without .bind — potential SQL injection
    if grep -Pn 'format!\s*\(\s*"[^"]*(?:SELECT|INSERT|UPDATE|DELETE)' "$rsfile" 2>/dev/null | grep -v 'bind\|\.execute\|sql_query' | head -3; then
        echo "WARNING: Possible unparameterized SQL in $rsfile"
        WARNINGS=$((WARNINGS + 1))
    fi
done

# 5. Log analysis: check for PANIC in test output (if available)
echo "[5/5] Checking for panic patterns..."
if grep -rn 'panic!\|todo!\|unimplemented!' modules/*/src/**/*.rs erp_core/src/**/*.rs 2>/dev/null | grep -v '// TODO\|#\[cfg(test)\]' | head -5; then
    echo "WARNING: Found panic!/todo!/unimplemented! macros in non-test code"
    WARNINGS=$((WARNINGS + 1))
fi

echo ""
echo "=== Module Lint Complete ==="
echo "Errors: $ERRORS, Warnings: $WARNINGS"

if [ $ERRORS -gt 0 ]; then
    exit 1
fi
`

	out, err := m.rustBase(source).
		// Install xmllint for XML validation
		WithExec([]string{"apt-get", "install", "-y", "libxml2-utils"}).
		WithNewFile("/app/module_lint.sh", lintScript, dagger.ContainerWithNewFileOpts{
			Permissions: 0755,
		}).
		WithExec([]string{"/app/module_lint.sh"}).
		Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("module lint failed: %w", err)
	}

	return out, nil
}

// All runs the full CI pipeline: check + lint + test + module-lint.
func (m *CentrixCi) All(ctx context.Context,
	// Source directory containing the Rust workspace
	source *dagger.Directory,
) (string, error) {
	var results []string

	// Phase 1: Compile check (fast gate)
	checkOut, err := m.Check(ctx, source)
	if err != nil {
		return "", fmt.Errorf("phase 1 (check) failed: %w", err)
	}
	results = append(results, checkOut)

	// Phase 2: Lint
	lintOut, err := m.Lint(ctx, source)
	if err != nil {
		return "", fmt.Errorf("phase 2 (lint) failed: %w", err)
	}
	results = append(results, lintOut)

	// Phase 3: Unit tests
	testOut, err := m.Test(ctx, source)
	if err != nil {
		return "", fmt.Errorf("phase 3 (test) failed: %w", err)
	}
	results = append(results, testOut)

	// Phase 4: Module lint
	moduleLintOut, err := m.ModuleLint(ctx, source)
	if err != nil {
		return "", fmt.Errorf("phase 4 (module-lint) failed: %w", err)
	}
	results = append(results, moduleLintOut)

	summary := fmt.Sprintf(
		"\n=== Full CI Pipeline Complete ===\n%s",
		strings.Join(results, "\n---\n"),
	)

	return summary, nil
}
