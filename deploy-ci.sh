#!/usr/bin/env bash
# Deploy Centrix CI Server to dev server (192.168.3.148)
set -euo pipefail

SERVER="ubuntu-server@192.168.3.148"
SSHPASS="sshpass -p 123"
SSH="$SSHPASS ssh -o StrictHostKeyChecking=no $SERVER"
SCP="$SSHPASS scp -o StrictHostKeyChecking=no"

CI_DIR="$(cd "$(dirname "$0")" && pwd)"
REMOTE_SRC="~/ci-build"
INSTALL_DIR="/opt/centrix-ci"
STATIC_DIR="/opt/rust-erp/erp_web/static"

echo "=== Centrix CI Deploy ==="

# 1. Sync source to server (exclude target/, .git/)
echo "[1/6] Syncing source..."
$SSHPASS rsync -az --delete \
  --exclude 'target/' \
  --exclude '.git/' \
  -e "ssh -o StrictHostKeyChecking=no" \
  "$CI_DIR/" "$SERVER:$REMOTE_SRC/"

# 2. Build on server
echo "[2/6] Building release binary (this may take a while)..."
$SSH "cd $REMOTE_SRC && source ~/.cargo/env && cargo build --release -p centrix-ci-server 2>&1 | tail -5"

# 3. Install binary + create directories
echo "[3/6] Installing binary..."
$SSH "sudo mkdir -p $INSTALL_DIR && sudo cp $REMOTE_SRC/target/release/centrix-ci $INSTALL_DIR/centrix-ci && sudo chmod +x $INSTALL_DIR/centrix-ci"

# 4. Create systemd service
echo "[4/6] Setting up systemd service..."
$SSH "sudo tee /etc/systemd/system/centrix-ci.service > /dev/null" <<'UNIT'
[Unit]
Description=Centrix CI Server
After=network.target postgresql.service
Wants=network.target

[Service]
Type=simple
User=ubuntu-server
Group=ubuntu-server
WorkingDirectory=/opt/centrix-ci
ExecStart=/opt/centrix-ci/centrix-ci
Environment=DATABASE_URL=postgres://erp:erp_password@localhost:5433/erp
Environment=CI_PORT=9090
Environment=RUST_LOG=info
Environment=STATIC_DIR=/opt/rust-erp/erp_web/static
Environment=CI_MAX_CONCURRENT=1
Environment=CI_WORKSPACE_DIR=/tmp/ci-workspace
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
UNIT

# 5. Restart service
echo "[5/6] Starting service..."
$SSH "sudo systemctl daemon-reload && sudo systemctl enable centrix-ci && sudo systemctl restart centrix-ci"
sleep 2

# 6. Health check
echo "[6/6] Health check..."
if $SSH "curl -sf http://localhost:9090/health > /dev/null 2>&1"; then
  echo "OK - Centrix CI Server is running on port 9090"
else
  echo "WARN - Server may still be starting. Check: systemctl status centrix-ci"
  $SSH "sudo journalctl -u centrix-ci --no-pager -n 20"
fi

echo "=== Deploy complete ==="
