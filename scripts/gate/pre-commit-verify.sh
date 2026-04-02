#!/usr/bin/env bash
# Fast pre-commit hook that only checks formatting and docs consistency.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

echo -e "${YELLOW}Running pre-commit checks (quick harness)...${NC}"

if ! bash scripts/run/verify-harness.sh --quick; then
    echo -e "${RED}verify-harness quick check failed.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ All pre-commit checks passed!${NC}"