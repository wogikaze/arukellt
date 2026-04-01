#!/bin/bash
# Root verification entry point for the repository harness.
#
# Default behavior is a fast local deterministic gate intended to finish quickly.
# Heavier checks are opt-in via explicit flags and are also wired into CI / optional hooks.

set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
NC=$'\033[0m'

# Use mise to ensure the correct Rust toolchain version if available.
MISE=""
if command -v mise &>/dev/null; then
  MISE="mise x --"
fi

RUN_CARGO=false
RUN_FIXTURES=false
RUN_BASELINE=false
RUN_SIZE=false
RUN_WAT=false
RUN_DOCS=false
RUN_COMPONENT=false
RUN_OPT_EQUIV=false
PERF_GATE=false

usage() {
    cat <<'EOF'
Usage: bash scripts/verify-harness.sh [options]

No options:
  Run the fast local verification gate.

Options:
  --quick      Alias for the default fast local gate
  --cargo      Run cargo fmt, clippy, and workspace tests
  --fixtures   Run the manifest-driven fixture harness
  --baseline   Run baseline collection
  --size       Run the hello.wasm size gate
  --wat        Run the WAT roundtrip gate
  --docs       Run markdownlint in addition to default docs checks
  --component  Run the optional component interop smoke test
  --opt-equiv  Run optimization equivalence tests (O0 vs O1)
  --full       Run all heavy local verification groups
  --perf-gate  Run the perf regression gate (still opt-in)
  --help       Show this help message
EOF
}

for arg in "$@"; do
    case "$arg" in
        --quick) ;;
        --cargo) RUN_CARGO=true ;;
        --fixtures) RUN_FIXTURES=true ;;
        --baseline) RUN_BASELINE=true ;;
        --size) RUN_SIZE=true ;;
        --wat) RUN_WAT=true ;;
        --docs) RUN_DOCS=true ;;
        --component) RUN_COMPONENT=true ;;
        --opt-equiv) RUN_OPT_EQUIV=true ;;
        --full)
            RUN_CARGO=true
            RUN_FIXTURES=true
            RUN_BASELINE=true
            RUN_SIZE=true
            RUN_WAT=true
            RUN_DOCS=true
            RUN_COMPONENT=true
            RUN_OPT_EQUIV=true
            ;;
        --perf-gate) PERF_GATE=true ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}error: unknown option: $arg${NC}"
            usage
            exit 1
            ;;
    esac
done

echo -e "${YELLOW}Running harness verification...${NC}"
if [ "$RUN_CARGO" = false ] && [ "$RUN_FIXTURES" = false ] && [ "$RUN_BASELINE" = false ] && [ "$RUN_SIZE" = false ] && [ "$RUN_WAT" = false ] && [ "$RUN_DOCS" = false ] && [ "$RUN_COMPONENT" = false ] && [ "$RUN_OPT_EQUIV" = false ] && [ "$PERF_GATE" = false ]; then
    echo -e "${YELLOW}Mode: fast local gate${NC}"
else
    selected=()
    [ "$RUN_CARGO" = true ] && selected+=(cargo)
    [ "$RUN_FIXTURES" = true ] && selected+=(fixtures)
    [ "$RUN_BASELINE" = true ] && selected+=(baseline)
    [ "$RUN_SIZE" = true ] && selected+=(size)
    [ "$RUN_WAT" = true ] && selected+=(wat)
    [ "$RUN_DOCS" = true ] && selected+=(docs)
    [ "$RUN_COMPONENT" = true ] && selected+=(component)
    [ "$RUN_OPT_EQUIV" = true ] && selected+=(opt-equiv)
    [ "$PERF_GATE" = true ] && selected+=(perf-gate)
    echo -e "${YELLOW}Mode: fast local gate + ${selected[*]}${NC}"
fi

TOTAL_CHECKS=0
PASSED_CHECKS=0
SKIPPED_CHECKS=0

check_pass() {
    echo -e "${GREEN}✓ $1${NC}"
    PASSED_CHECKS=$((PASSED_CHECKS + 1))
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

check_skip() {
    echo -e "${YELLOW}⊙ $1 (skipped)${NC}"
    SKIPPED_CHECKS=$((SKIPPED_CHECKS + 1))
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

check_fail() {
    echo -e "${RED}✗ $1${NC}"
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
}

run_check() {
    local label="$1"
    local command="$2"
    local output
    if output=$(bash -lc "$command" 2>&1); then
        check_pass "$label"
    else
        check_fail "$label"
        printf '%s\n' "$output" | tail -30
    fi
}

FIXTURE_COUNT=$(python3 - <<'PY'
from pathlib import Path
manifest = Path('tests/fixtures/manifest.txt')
count = sum(1 for line in manifest.read_text().splitlines() if line.strip() and not line.strip().startswith('#'))
print(count)
PY
)

# ── Background jobs ───────────────────────────────────────────────────────────
_BG_DIR=$(mktemp -d)
trap 'rm -rf "$_BG_DIR"' EXIT

_bg_run() {
    local id="$1"; shift
    local label="$1"; shift
    printf '%s\n' "$label" > "$_BG_DIR/$id.label"
    if bash -lc "$*" > "$_BG_DIR/$id.out" 2>&1; then
        echo 0 > "$_BG_DIR/$id.rc"
    else
        echo 1 > "$_BG_DIR/$id.rc"
    fi
}

_bg_collect() {
    local id="$1"
    local label rc
    label=$(cat "$_BG_DIR/$id.label")
    rc=$(cat "$_BG_DIR/$id.rc" 2>/dev/null || echo 1)
    if [ "$rc" = "0" ]; then
        check_pass "$label"
    else
        check_fail "$label"
        cat "$_BG_DIR/$id.out" | tail -30
    fi
}

_bg_run docs_struct "Documentation structure OK" \
    "test -f AGENTS.md && test -f docs/process/agent-harness.md && test -d docs/adr && test -d issues/open && test -d issues/done && test -d docs/language && test -d docs/platform && test -d docs/stdlib && test -d docs/process" &
_bg_run adrs "All required ADRs decided" \
    "for f in docs/adr/ADR-002-memory-model.md docs/adr/ADR-003-generics-strategy.md docs/adr/ADR-004-trait-strategy.md docs/adr/ADR-005-llvm-scope.md docs/adr/ADR-006-abi-policy.md; do test -f \"\$f\" || exit 1; grep -q 'DECIDED\|決定' \"\$f\" || exit 1; done" &
_bg_run lang_spec "Language specification OK" \
    "test -f docs/language/memory-model.md && test -f docs/language/type-system.md && test -f docs/language/syntax.md" &
_bg_run platform_spec "Platform specification OK" \
    "test -f docs/platform/wasm-features.md && test -f docs/platform/abi.md && test -f docs/platform/wasi-resource-model.md" &
_bg_run stdlib_spec "Stdlib specification OK" \
    "test -f docs/stdlib/README.md && test -f docs/stdlib/core.md && test -f docs/stdlib/io.md" &
_bg_run docs_consistency "docs consistency (${FIXTURE_COUNT} fixtures)" \
    "python3 scripts/check-docs-consistency.py" &
_bg_run docs_freshness "docs freshness (project-state.toml vs manifest.txt)" \
    "python3 scripts/check-docs-freshness.py" &
_bg_run stdlib_manifest "stdlib manifest check" \
    "bash scripts/check-stdlib-manifest.sh" &
_bg_run done_issues_checkboxes "issues/done/ has no unchecked checkboxes" \
    "files=\$(grep -rl '\\- \\[ \\]' issues/done/ 2>/dev/null | grep '\\.md\$' || true); if [ -n \"\$files\" ]; then echo 'Files in done/ with unchecked items:'; printf '%s\n' \"\$files\"; exit 1; fi" &
_bg_run no_panic_audit "no panic/unwrap in user-facing crates" \
    "bash scripts/check-panic-audit.sh" &
_bg_run asset_naming "asset naming convention (snake_case)" \
    "bash scripts/check-asset-naming.sh" &
_bg_run generated_boundary "generated file boundary check" \
    "bash scripts/check-generated-files.sh" &
if [ "$RUN_DOCS" = true ]; then
    _bg_run markdownlint "markdownlint-cli2 **/*.md --fix --config .markdownlint.json" \
        "npx markdownlint-cli2 '**/*.md' --fix --config .markdownlint.json" &
fi

# ── Optional heavy groups ────────────────────────────────────────────────────
if [ "$RUN_CARGO" = true ]; then
    printf '\n%s\n' "${YELLOW}[cargo] Running cargo verification...${NC}"
    run_check "cargo fmt --all --check" "$MISE cargo fmt --all --check"
    run_check "cargo clippy --workspace -- -D warnings" "$MISE cargo clippy --workspace --exclude ark-llvm -- -D warnings"
    run_check "cargo test --workspace" "$MISE cargo test --workspace --exclude ark-llvm --quiet -- --skip fixture_harness"
fi

if [ "$RUN_FIXTURES" = true ]; then
    printf '\n%s\n' "${YELLOW}[fixtures] Running fixture harness...${NC}"
    local_output=$(ARUKELLT_BIN="${ARUKELLT_BIN:-}" $MISE bash -lc "cargo test -p arukellt --test harness -- --nocapture 2>&1") || true
    if printf '%s\n' "$local_output" | grep -q "FAIL: 0"; then
        summary=$(printf '%s\n' "$local_output" | grep "PASS:")
        check_pass "fixture harness (${summary})"
    else
        check_fail "fixture harness"
        printf '%s\n' "$local_output" | grep -E "^(PASS:|FAIL )" | head -30
    fi
fi

# ── Fast default checks ──────────────────────────────────────────────────────
printf '\n%s\n' "${YELLOW}[manifest] Checking fixture manifest completeness...${NC}"
manifest_ok=true
manifest_file="tests/fixtures/manifest.txt"
if [ ! -f "$manifest_file" ]; then
    check_fail "Fixture manifest not found: $manifest_file"
    manifest_ok=false
else
    disk_entries=$(python3 - <<'PY'
from pathlib import Path
root = Path('tests/fixtures')
entries = []
for ark in sorted(root.rglob('*.ark')):
    rel = ark.relative_to(root)
    if ark.name != 'main.ark' and (ark.parent / 'main.ark').exists():
        continue
    entries.append(str(rel))
print('\n'.join(entries))
PY
)
    manifest_entries=$(python3 - <<'PY'
from pathlib import Path
manifest = Path('tests/fixtures/manifest.txt')
rows = set()
for line in manifest.read_text().splitlines():
    line = line.strip()
    if not line or line.startswith('#'):
        continue
    kind, path = line.split(':', 1)
    if kind == 'bench':
        continue
    rows.add(path)
print('\n'.join(sorted(rows)))
PY
)
    diff_result=$(diff <(printf '%s\n' "$disk_entries") <(printf '%s\n' "$manifest_entries") || true)
    if [ -n "$diff_result" ]; then
        check_fail "Fixture manifest out of sync with disk"
        printf '%s\n' "$diff_result" | head -20
        manifest_ok=false
    fi
fi
if [ "$manifest_ok" = true ]; then
    check_pass "Fixture manifest completeness (${FIXTURE_COUNT} entries)"
fi

if [ "$RUN_SIZE" = true ]; then
    printf '\n%s\n' "${YELLOW}[size] Checking hello.wasm binary size gate...${NC}"
    ARUKELLT_BIN="${ARUKELLT_BIN:-./target/debug/arukellt}"
    if [ ! -x "$ARUKELLT_BIN" ] && [ "$ARUKELLT_BIN" = "./target/debug/arukellt" ]; then
        ARUKELLT_BIN="./target/release/arukellt"
    fi
    HELLO_WASM_OUT="hello_perfgate.wasm"
    HELLO_SIZE_MAX=5120
    if "$ARUKELLT_BIN" compile tests/fixtures/hello/hello.ark --target wasm32-wasi-p2 --opt-level 1 -o "$HELLO_WASM_OUT" 2>/dev/null; then
        HELLO_SIZE=$(wc -c < "$HELLO_WASM_OUT")
        rm -f "$HELLO_WASM_OUT"
        if [ "$HELLO_SIZE" -le "$HELLO_SIZE_MAX" ]; then
            check_pass "hello.wasm binary size: ${HELLO_SIZE} bytes (<= ${HELLO_SIZE_MAX})"
        else
            check_fail "hello.wasm binary size: ${HELLO_SIZE} bytes (> ${HELLO_SIZE_MAX} threshold)"
        fi
    else
        rm -f "$HELLO_WASM_OUT"
        check_fail "hello.wasm compilation failed"
    fi
fi

if [ "$RUN_BASELINE" = true ]; then
    printf '\n%s\n' "${YELLOW}[baseline] Collecting baseline snapshots...${NC}"
    run_check "baseline collection" "python3 scripts/collect-baseline.py"
fi

printf '\n%s\n' "${YELLOW}[bg] Collecting background check results...${NC}"
wait
_bg_collect docs_struct
_bg_collect adrs
_bg_collect lang_spec
_bg_collect platform_spec
_bg_collect stdlib_spec
_bg_collect docs_consistency
_bg_collect docs_freshness
_bg_collect stdlib_manifest
_bg_collect done_issues_checkboxes
_bg_collect no_panic_audit
_bg_collect asset_naming
_bg_collect generated_boundary
if [ "$RUN_DOCS" = true ]; then
    _bg_collect markdownlint
fi

check_pass "Perf policy documented (check<=10%, compile<=20%; heavy perf separated)"

stdlib_fixture_dirs=$(find tests/fixtures -type d -name 'stdlib_*' 2>/dev/null)
stdlib_missing=0
for dir in $stdlib_fixture_dirs; do
    for ark in "$dir"/*.ark; do
        [ -f "$ark" ] || continue
        rel_path="${ark#tests/fixtures/}"
        if ! grep -qF "$rel_path" tests/fixtures/manifest.txt 2>/dev/null; then
            printf '  Missing from manifest.txt: %s\n' "$rel_path"
            stdlib_missing=$((stdlib_missing + 1))
        fi
    done
done
if [ "$stdlib_missing" -eq 0 ]; then
    check_pass "all stdlib fixtures registered in manifest.txt"
else
    check_fail "stdlib fixtures missing from manifest.txt ($stdlib_missing)"
fi

stdlib_fixture_count=$(grep -c 'stdlib_' tests/fixtures/manifest.txt 2>/dev/null || echo "0")
if [ "$stdlib_fixture_count" -ge 5 ]; then
    check_pass "v3 stdlib fixtures registered ($stdlib_fixture_count entries in manifest)"
else
    check_fail "v3 stdlib fixtures insufficient ($stdlib_fixture_count < 5)"
fi

# Hygiene checks
if [ -f scripts/check-links.sh ]; then
    if bash scripts/check-links.sh >/dev/null 2>&1; then
        check_pass "internal link integrity"
    else
        check_fail "broken internal links detected (run scripts/check-links.sh)"
    fi
fi

if [ -f scripts/check-diagnostic-codes.sh ]; then
    if bash scripts/check-diagnostic-codes.sh >/dev/null 2>&1; then
        check_pass "diagnostic codes aligned"
    else
        check_fail "diagnostic codes out of sync (run scripts/check-diagnostic-codes.sh)"
    fi
fi

if [ "$RUN_COMPONENT" = true ]; then
    printf '\n%s\n' "${YELLOW}[component] Component interop smoke test...${NC}"
    if ! command -v wasmtime >/dev/null 2>&1; then
        check_skip "component interop (wasmtime not found)"
    else
        for INTEROP_SCRIPT in tests/component-interop/jco/*/run.sh; do
            if [ ! -f "$INTEROP_SCRIPT" ]; then
                check_skip "component interop scripts not found"
                break
            fi
            fixture_name=$(basename "$(dirname "$INTEROP_SCRIPT")")
            run_check "component interop: $fixture_name (wasmtime)" "bash $INTEROP_SCRIPT"
        done
    fi
fi

if [ "$RUN_WAT" = true ]; then
    printf '\n%s\n' "${YELLOW}[wat] Running WAT roundtrip verification...${NC}"
    run_check "WAT roundtrip (wasm2wat ⇄ wat2wasm)" "bash scripts/wat-roundtrip.sh"
fi

if [ "$PERF_GATE" = true ]; then
    printf '\n%s\n' "${YELLOW}[perf] Running performance regression gate...${NC}"
    run_check "perf gate (compile time / binary size / run time)" "bash scripts/perf-gate.sh"
fi

if [ "$RUN_OPT_EQUIV" = true ]; then
    printf '\n%s\n' "${YELLOW}[opt-equiv] Running optimization equivalence tests (O0 vs O1)...${NC}"
    run_check "optimization equivalence (O0 vs O1)" "bash scripts/test-opt-equivalence.sh --quick"
fi

printf '\n%s\n' "${YELLOW}========================================${NC}"
printf '%s\n' "${YELLOW}Summary${NC}"
printf '%s\n' "${YELLOW}========================================${NC}"
printf 'Total checks: %s\n' "$TOTAL_CHECKS"
printf 'Passed: %b%s%b\n' "$GREEN" "$PASSED_CHECKS" "$NC"
printf 'Skipped: %b%s%b\n' "$YELLOW" "$SKIPPED_CHECKS" "$NC"
printf 'Failed: %b%s%b\n' "$RED" "$((TOTAL_CHECKS - PASSED_CHECKS - SKIPPED_CHECKS))" "$NC"

if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ] || [ $((PASSED_CHECKS + SKIPPED_CHECKS)) -eq $TOTAL_CHECKS ]; then
    printf '\n%b✓ All selected harness checks passed%b\n' "$GREEN" "$NC"
    exit 0
else
    printf '\n%b✗ Some selected harness checks failed%b\n' "$RED" "$NC"
    exit 1
fi
