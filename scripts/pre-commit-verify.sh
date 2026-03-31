#!/usr/bin/env bash
# Fast pre-commit hook that only checks formatting and docs consistency.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

# Use mise to ensure the correct toolchain version if available.
MISE=""
if command -v mise &>/dev/null; then
  MISE="mise x --"
fi

echo -e "${YELLOW}Running pre-commit checks (format and docs)...${NC}"

# 1. Cargo fmt check
echo "Checking cargo fmt..."
if ! $MISE cargo fmt --all --check; then
    echo -e "${RED}cargo fmt check failed. Run 'cargo fmt --all' to fix.${NC}"
    exit 1
fi

# 2. Markdownlint check
echo "Checking markdownlint..."
# Use npx for markdownlint-cli2
if ! npx markdownlint-cli2 '**/*.md' --config .markdownlint.json; then
    echo -e "${RED}markdownlint check failed. Run 'npx markdownlint-cli2 --fix' to fix.${NC}"
    exit 1
fi

# 3. Docs consistency check
echo "Checking docs consistency..."
if ! $MISE python3 scripts/check-docs-consistency.py; then
    echo -e "${RED}Docs consistency check failed. Run 'python3 scripts/generate-docs.py' to update.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ All pre-commit checks passed!${NC}"
