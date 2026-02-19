use dagger_sdk::{Directory, Query};

use crate::containers;

/// Validate module manifests, XML data files, and code patterns.
pub async fn run(client: &Query, source: Directory) -> eyre::Result<String> {
    let script = r#"
set -euo pipefail

ERRORS=0
WARNINGS=0

echo "=== Module Lint ==="

# 1. Manifest validation
echo "[1/5] Checking module manifests..."
for manifest in modules/*/manifest.toml; do
    module_dir=$(dirname "$manifest")

    if ! grep -q '^\[module\]' "$manifest"; then
        echo "ERROR: $manifest missing [module] section"
        ERRORS=$((ERRORS + 1))
    fi
    if ! grep -q 'name\s*=' "$manifest"; then
        echo "ERROR: $manifest missing 'name' key"
        ERRORS=$((ERRORS + 1))
    fi

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

# 2. XML validation
echo "[2/5] Checking XML data files..."
for xmlfile in modules/*/data/*.xml modules/*/views/*.xml modules/*/security/*.xml; do
    [ -f "$xmlfile" ] || continue
    if ! xmllint --noout "$xmlfile" 2>/dev/null; then
        echo "ERROR: $xmlfile is not well-formed XML"
        ERRORS=$((ERRORS + 1))
    fi
done

# 3. Duplicate record IDs
echo "[3/5] Checking for duplicate record IDs..."
for module_dir in modules/*/; do
    [ -d "$module_dir" ] || continue
    module_name=$(basename "$module_dir")
    ids=$(grep -roh 'id="[^"]*"' "$module_dir" 2>/dev/null | sort | uniq -d)
    if [ -n "$ids" ]; then
        echo "WARNING: Duplicate record IDs in $module_name: $ids"
        WARNINGS=$((WARNINGS + 1))
    fi
done

# 4. Unsafe SQL patterns
echo "[4/5] Checking for unsafe SQL patterns..."
for rsfile in $(find modules/ erp_core/src/ -name '*.rs' 2>/dev/null); do
    [ -f "$rsfile" ] || continue
    if grep -Pn 'format!\s*\(\s*"[^"]*(?:SELECT|INSERT|UPDATE|DELETE)' "$rsfile" 2>/dev/null | grep -v 'bind\|\.execute\|sql_query' | head -3; then
        echo "WARNING: Possible unparameterized SQL in $rsfile"
        WARNINGS=$((WARNINGS + 1))
    fi
done

# 5. Panic patterns
echo "[5/5] Checking for panic patterns..."
if find modules/ erp_core/src/ -name '*.rs' -exec grep -ln 'panic!\|todo!\|unimplemented!' {} \; 2>/dev/null | head -5 | grep -q .; then
    echo "WARNING: Found panic!/todo!/unimplemented! macros in source code"
    WARNINGS=$((WARNINGS + 1))
fi

echo ""
echo "=== Module Lint Complete ==="
echo "Errors: $ERRORS, Warnings: $WARNINGS"

if [ $ERRORS -gt 0 ]; then
    exit 1
fi
"#;

    let output = containers::rust_base(client, source)
        .with_exec(vec!["apt-get", "install", "-y", "libxml2-utils"])
        .with_exec(vec!["bash", "-c", script])
        .stdout()
        .await?;

    Ok(format!("[module-lint] {output}"))
}
