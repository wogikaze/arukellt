#!/usr/bin/env bash
# scripts/gate/pre-commit-verify.sh — Pre-commit gate: scripts root check + cargo fmt check + markdownlint (staged files only).
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

# Return staged files, NUL-delimited.
staged_files() {
  git diff --cached --name-only --diff-filter=ACMR -z
}

# ── 1. scripts root check ────────────────────────────────────────────────────
banner "scripts root directory check"
if python3 scripts/check/check-scripts-root.py; then
  step "OK"
else
  echo "FAIL: scripts/ root directory contains unexpected files. Only manager.py and README.md are allowed." >&2
  FAIL=1
fi

# ── 2. cargo fmt check ──────────────────────────────────────────────────────
banner "cargo fmt --check"
if cargo fmt --all --check; then
  step "OK"
else
  echo "FAIL: cargo fmt check failed. Run 'cargo fmt --all' to fix." >&2
  FAIL=1
fi

# ── 3. markdownlint (staged .md only) ───────────────────────────────────────
banner "markdownlint (staged .md only)"
if command -v npx >/dev/null 2>&1; then
  mapfile -d '' MD_FILES < <(
    staged_files | while IFS= read -r -d '' path; do
      case "$path" in
        *.md)
          case "$path" in
            node_modules/*|target/*|wt/*) ;;
            *) printf '%s\0' "$path" ;;
          esac
          ;;
      esac
    done
  )

  if [ "${#MD_FILES[@]}" -eq 0 ]; then
    step "No staged markdown files"
  else
    if npx --yes markdownlint-cli2 --config .markdownlint-cli2.jsonc "${MD_FILES[@]}"; then
      step "OK"
    else
      echo "FAIL: markdownlint found issues in staged markdown files. Run 'mise run fmt:docs' or fix them manually." >&2
      FAIL=1
    fi
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