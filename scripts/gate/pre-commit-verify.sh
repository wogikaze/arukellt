#!/usr/bin/env bash
# scripts/gate/pre-commit-verify.sh — Pre-commit gate: cargo fmt check + markdownlint.
# Run directly or via .git/hooks/pre-commit (installed by scripts/gate/install-git-hooks.sh).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

FAIL=0

step() {
  echo "  >> $*"
}

banner() {
  echo ""
  echo "==> $*"
}

# ── 1. cargo fmt check ──────────────────────────────────────────────────────
banner "cargo fmt --check"
if cargo fmt --all --check; then
  step "OK"
else
  echo "FAIL: cargo fmt check failed. Run 'cargo fmt --all' to fix." >&2
  FAIL=1
fi

# ── 2. markdownlint ─────────────────────────────────────────────────────────
banner "markdownlint"
if command -v npx >/dev/null 2>&1; then
  if npx --yes markdownlint-cli2 '**/*.md' --config .markdownlint.json \
       --ignore node_modules --ignore target; then
    step "OK"
  else
    echo "FAIL: markdownlint found issues. Run 'mise run fmt:docs' to auto-fix." >&2
    FAIL=1
  fi
else
  echo "SKIP: npx not found, skipping markdownlint" >&2
fi

# ── result ──────────────────────────────────────────────────────────────────
echo ""
if [ "$FAIL" -ne 0 ]; then
  echo "pre-commit: FAILED — fix the above errors before committing."
  exit 1
fi
echo "pre-commit: all checks passed."
