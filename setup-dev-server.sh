#!/bin/bash
# Setup Dagger CI on the dev server (192.168.3.148)
#
# Run this once on the dev server to install Dagger + Go:
#   ssh ubuntu-server@192.168.3.148
#   cd ~/rust-erp-dev/rust-erp/ci
#   bash setup-dev-server.sh

set -euo pipefail

echo "=== Centrix CI/CD Setup ==="

# 1. Install Dagger CLI
echo "[1/3] Installing Dagger CLI..."
if ! command -v dagger &>/dev/null; then
    curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
    # Add to PATH if not already there
    export PATH="$HOME/.local/bin:$PATH"
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
    echo "Dagger installed: $(dagger version)"
else
    echo "Dagger already installed: $(dagger version)"
fi

# 2. Install Go (needed for Dagger Go SDK)
echo "[2/3] Installing Go..."
if ! command -v go &>/dev/null; then
    GO_VERSION="1.23.6"
    wget -q "https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz" -O /tmp/go.tar.gz
    sudo rm -rf /usr/local/go
    sudo tar -C /usr/local -xzf /tmp/go.tar.gz
    rm /tmp/go.tar.gz
    export PATH="/usr/local/go/bin:$PATH"
    echo 'export PATH="/usr/local/go/bin:$PATH"' >> ~/.bashrc
    echo "Go installed: $(go version)"
else
    echo "Go already installed: $(go version)"
fi

# 3. Initialize Dagger module (generates go.sum, internal/ directory)
echo "[3/3] Initializing Dagger module..."
cd "$(dirname "$0")"
if [ ! -f "go.sum" ]; then
    dagger develop
    echo "Dagger module initialized."
else
    echo "Dagger module already initialized."
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Run the full pipeline:"
echo "  cd ci && dagger call all --source=.."
echo ""
echo "Run individual steps:"
echo "  dagger call check --source=.."
echo "  dagger call lint --source=.."
echo "  dagger call test --source=.."
echo "  dagger call integration-test --source=.."
echo "  dagger call module-lint --source=.."
