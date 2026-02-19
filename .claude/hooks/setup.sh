#!/bin/bash
# Run once after cloning to install git hooks
REPO_ROOT=$(git rev-parse --show-toplevel)
cp "$REPO_ROOT/.claude/hooks/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
cp "$REPO_ROOT/.claude/hooks/prepare-commit-msg" "$REPO_ROOT/.git/hooks/prepare-commit-msg"
chmod +x "$REPO_ROOT/.git/hooks/pre-commit" "$REPO_ROOT/.git/hooks/prepare-commit-msg"
git config user.name "centrix-luke"
git config user.email "44260364+centrix-luke@users.noreply.github.com"
echo "Git hooks installed and author configured."
