#!/usr/bin/env bash
# scripts/gate/pre-commit-verify.sh — Pre-commit gate: repo structure check + quick verification + markdownlint (staged files only).
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
if bash scripts/check/check-repo-structure.sh; then
  step "OK"
else
  echo "FAIL: repository structure check failed." >&2
  FAIL=1
fi

# ── 1b. selfhost wasm rebuild (when compiler/stdlib source is staged) ────────
# The T3 WASM validation gate (run in verify --quick below) tests the prebuilt
# .build/selfhost/arukellt-s2.wasm.  If the staged commit changes compiler or
# stdlib source but the wasm is not rebuilt, the gate tests a STALE wasm and
# can pass even though the new source breaks the compiler.  This step rebuilds
# the wasm from the working-tree source before running verify so the gate
# exercises the actual code being committed.
banner "selfhost wasm freshness check"
NEEDS_REBUILD=0
while IFS= read -r -d '' path; do
  case "$path" in
    src/compiler/*|std/*)
      NEEDS_REBUILD=1
      break
      ;;
  esac
done < <(staged_files)

if [ "$NEEDS_REBUILD" -ne 0 ]; then
  step "staged compiler/stdlib source detected — rebuilding selfhost wasm"
  S2_WASM=".build/selfhost/arukellt-s2.wasm"
  set +e
  BUILD_OUTPUT=$(python3 scripts/manager.py selfhost fixpoint --build 2>&1)
  BUILD_RC=$?
  set -e
  # Exit codes: 0=fixpoint reached, 1=fixpoint not yet reached (s2 built OK),
  #             2=prereqs missing or build failed.
  if [ "$BUILD_RC" -eq 2 ]; then
    echo "FAIL: selfhost wasm rebuild failed (exit $BUILD_RC)." >&2
    echo "$BUILD_OUTPUT" | tail -20 >&2
    echo "  Fix the build error above, then re-stage and retry." >&2
    FAIL=1
  elif [ ! -f "$S2_WASM" ]; then
    echo "FAIL: selfhost wasm rebuild finished but $S2_WASM not found." >&2
    FAIL=1
  else
    step "selfhost wasm rebuilt OK (fixpoint exit $BUILD_RC)"
  fi
else
  step "no compiler/stdlib source staged — skipping wasm rebuild"
fi

# ── 2. quick verification ──────────────────────────────────────────────────
banner "manager verify --quick"
if python3 scripts/manager.py verify --quick; then
  step "OK"
else
  echo "FAIL: quick verification failed." >&2
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
