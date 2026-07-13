#!/usr/bin/env bash
# scripts/gate/pre-commit-verify.sh — Pre-commit gate: staged .ark fmt + lint,
# repo structure check + quick verification + markdownlint (staged files only).
#
# Content-hash result cache: if the staged index, selfhost wasm, and check
# scripts have not changed since the last run, the entire gate is skipped and
# the cached result is returned in <100ms.  Set PRE_COMMIT_NO_CACHE=1 to bypass.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

FAIL=0
SELFHOST_CLI="$REPO_ROOT/scripts/run/arukellt-selfhost.sh"

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

# ── 0. Content-hash result cache ──────────────────────────────────────────────
# Fingerprint: git index tree + s2.wasm hash + check-script hashes.
# On hit: skip the entire gate, return cached exit code.
CACHE_FILE=".build/pre-commit-cache.json"
NO_CACHE="${PRE_COMMIT_NO_CACHE:-0}"

compute_fingerprint() {
  local idx_tree s2_sha scripts_sha
  idx_tree=$(git write-tree 2>/dev/null || echo "no-index")
  if [ -f ".build/selfhost/arukellt-s2.wasm" ]; then
    s2_sha=$(sha256sum .build/selfhost/arukellt-s2.wasm | cut -d' ' -f1)
  else
    s2_sha="no-s2"
  fi
  # Hash all check scripts + gate scripts + manager.py that define the gate logic.
  scripts_sha=$({
    find scripts/check scripts/gate -type f \( -name '*.py' -o -name '*.sh' \) 2>/dev/null
    echo scripts/manager.py
    echo scripts/selfhost/checks.py
  } | sort | xargs sha256sum 2>/dev/null | sha256sum | cut -d' ' -f1)
  echo "${idx_tree}:${s2_sha}:${scripts_sha}"
}

if [ "$NO_CACHE" != "1" ] && [ -f "$CACHE_FILE" ]; then
  CACHED_FP=$(python3 -c "
import json, sys
try:
    d = json.load(open('$CACHE_FILE'))
    print(d.get('fingerprint', ''))
except Exception:
    pass
" 2>/dev/null || true)
  CURRENT_FP=$(compute_fingerprint)
  if [ -n "$CACHED_FP" ] && [ "$CACHED_FP" = "$CURRENT_FP" ]; then
    CACHED_RC=$(python3 -c "
import json
d = json.load(open('$CACHE_FILE'))
print(d.get('exit_code', 1))
" 2>/dev/null || echo "1")
    echo "[pre-commit cache: hit — fingerprint unchanged, skipping gate]"
    if [ "$CACHED_RC" = "0" ]; then
      echo "pre-commit: all checks passed (cached)."
      exit 0
    else
      echo "pre-commit: FAILED (cached) — fix errors then re-run with PRE_COMMIT_NO_CACHE=1." >&2
      exit 1
    fi
  fi
fi

# ── 0b. arukellt fmt --check (staged .ark only) ──────────────────────────────
banner "arukellt fmt --check (staged .ark only)"
mapfile -d '' ARK_FILES < <(
  staged_files | while IFS= read -r -d '' path; do
    case "$path" in
      *.ark) printf '%s\0' "$path" ;;
    esac
  done
)

if [ "${#ARK_FILES[@]}" -eq 0 ]; then
  step "No staged .ark files"
elif [ ! -x "$SELFHOST_CLI" ]; then
  echo "FAIL: $SELFHOST_CLI not executable; required for staged .ark fmt check." >&2
  FAIL=1
else
  FMT_FAIL=0
  for ark in "${ARK_FILES[@]}"; do
    set +e
    FMT_OUTPUT=$("$SELFHOST_CLI" fmt --check "$ark" 2>&1)
    FMT_RC=$?
    set -e
    if [ "$FMT_RC" -ne 0 ]; then
      echo "FAIL: $ark is not formatted." >&2
      echo "$FMT_OUTPUT" | tail -20 >&2
      echo "  Run: scripts/run/arukellt-selfhost.sh fmt $ark, then re-stage." >&2
      FMT_FAIL=1
    fi
  done
  if [ "$FMT_FAIL" -ne 0 ]; then
    FAIL=1
  else
    step "OK"
  fi
fi

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
#
# The fixpoint build itself uses a content-hash cache (in run_fixpoint) so it
# is skipped when the source hasn't changed — see ARUKELLT_FIXPOINT_NO_CACHE=1
# to bypass that inner cache.
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

# ── 1c. arukellt lint (staged .ark only, after wasm rebuild) ─────────────────
# Two lint tiers (ADR-047):
#   - package modules (src/compiler, std): `lint --local` (parse + AST rules)
#   - standalone programs: full `lint` (resolve/typecheck + local rules)
# Prefer-else-if is denied so nested else { if } on staged files fails the commit.
banner "arukellt lint (staged .ark only)"
if [ "${#ARK_FILES[@]}" -eq 0 ]; then
  step "No staged .ark files"
elif [ ! -x "$SELFHOST_CLI" ]; then
  echo "FAIL: $SELFHOST_CLI not executable; required for staged .ark lint." >&2
  FAIL=1
else
  LINT_FAIL=0
  LINT_LOCAL=0
  LINT_FULL=0
  for ark in "${ARK_FILES[@]}"; do
    LINT_ARGS=(lint --deny prefer-else-if)
    case "$ark" in
      src/compiler/*|std/*)
        LINT_ARGS=(lint --local --deny prefer-else-if)
        LINT_LOCAL=$((LINT_LOCAL + 1))
        ;;
      *)
        LINT_FULL=$((LINT_FULL + 1))
        ;;
    esac
    set +e
    LINT_OUTPUT=$("$SELFHOST_CLI" "${LINT_ARGS[@]}" "$ark" 2>&1)
    LINT_RC=$?
    set -e
    if [ "$LINT_RC" -ne 0 ]; then
      echo "FAIL: lint failed for $ark (exit $LINT_RC)." >&2
      echo "$LINT_OUTPUT" | tail -40 >&2
      echo "  Fix W0011 (else if) or other denied/errors, then re-stage." >&2
      LINT_FAIL=1
    elif [ -n "$LINT_OUTPUT" ]; then
      # Surface remaining warnings without failing the hook.
      echo "$LINT_OUTPUT" | head -20
    fi
  done
  if [ "$LINT_FAIL" -ne 0 ]; then
    FAIL=1
  else
    step "OK (local=$LINT_LOCAL full=$LINT_FULL)"
  fi
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
      echo "FAIL: markdownlint found issues in staged markdown files. Run 'npx markdownlint-cli2 \"**/*.md\" --fix' or fix them manually." >&2
      FAIL=1
    fi
  fi
else
  echo "SKIP: npx not found, skipping markdownlint" >&2
fi

# ── result + cache write ─────────────────────────────────────────────────────
echo ""
if [ "$FAIL" -ne 0 ]; then
  echo "pre-commit: FAILED — fix the above errors before committing."
  if [ "$NO_CACHE" != "1" ]; then
    FP=$(compute_fingerprint)
    mkdir -p .build
    python3 -c "
import json
json.dump({'fingerprint': '$FP', 'exit_code': 1, 'ts': 0}, open('$CACHE_FILE', 'w'))
"
  fi
  exit 1
fi
echo "pre-commit: all checks passed."
if [ "$NO_CACHE" != "1" ]; then
  FP=$(compute_fingerprint)
  mkdir -p .build
  python3 -c "
import json, time
json.dump({'fingerprint': '$FP', 'exit_code': 0, 'ts': time.time()}, open('$CACHE_FILE', 'w'))
"
fi
