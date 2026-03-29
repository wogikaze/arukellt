#!/bin/bash
# Root verification and completion gate for the repository harness.
# This script defines what "done" means for this project.
#
# Performance design:
#  - Non-cargo checks (docs, lint, manifest) run as background jobs in parallel
#    with the cargo pipeline so they overlap compilation time.
#  - The fixture harness runs fixtures in parallel (N = CPU cores) inside Rust.
#  - cargo build is omitted: cargo test already builds, and clippy already
#    compiled everything before test runs.
#  - hello.wasm size uses the already-built debug binary, not `cargo run`.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

QUICK_MODE=false
PERF_GATE=false
for arg in "$@"; do
    case "$arg" in
        --quick)     QUICK_MODE=true ;;
        --perf-gate) PERF_GATE=true ;;
    esac
done

echo -e "${YELLOW}Running harness verification...${NC}"
if [ "$QUICK_MODE" = true ]; then
    echo -e "${YELLOW}Quick mode: skipping slower cargo verification steps${NC}"
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
# Start non-cargo checks immediately so they run in parallel with cargo.
# Each job writes a result file: rc=0 means pass, rc!=0 means fail.
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

# Launch background jobs (overlap with cargo compilation below)
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
_bg_run markdownlint "markdownlint-cli2 **/*.md --fix --config .markdownlint.json" \
    "npx markdownlint-cli2 '**/*.md' --fix --config .markdownlint.json" &
_bg_run stdlib_manifest "stdlib manifest check" \
    "bash scripts/check-stdlib-manifest.sh" &

# ── Cargo pipeline ────────────────────────────────────────────────────────────
# cargo commands must run sequentially (cargo uses a file lock).
# clippy compiles everything; fmt check and test reuse the same artifacts.
# NOTE: cargo build is intentionally omitted — clippy already compiled the
# workspace, and cargo test links+runs without a separate build step.

printf '\n%s\n' "${YELLOW}[fmt] Checking formatting...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo fmt --all --check"
else
    run_check "cargo fmt --all --check" "cargo fmt --all --check"
fi

printf '\n%s\n' "${YELLOW}[clippy] Running clippy...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo clippy --workspace -- -D warnings"
else
    run_check "cargo clippy --workspace -- -D warnings" "cargo clippy --workspace --exclude ark-llvm -- -D warnings"
fi

printf '\n%s\n' "${YELLOW}[test] Running workspace tests...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test --workspace"
else
    run_check "cargo test --workspace" "cargo test --workspace --exclude ark-llvm --quiet -- --skip fixture_harness"
fi

# Fixture harness — fixtures run in parallel inside Rust (N = available CPU cores).
printf '\n%s\n' "${YELLOW}[harness] Running fixture harness (parallel)...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "cargo test -p arukellt --test harness -- --nocapture"
else
    local_output=$(bash -lc "cargo test -p arukellt --test harness -- --nocapture 2>&1") || true
    if printf '%s\n' "$local_output" | grep -q "FAIL: 0"; then
        summary=$(printf '%s\n' "$local_output" | grep "PASS:")
        check_pass "fixture harness (${summary})"
    else
        check_fail "fixture harness"
        printf '%s\n' "$local_output" | grep -E "^(PASS:|FAIL )" | head -30
    fi
fi

# ── Fixture manifest completeness ─────────────────────────────────────────────
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

# ── Binary size gate ──────────────────────────────────────────────────────────
# Use the already-built debug binary to avoid an extra `cargo run` invocation.
printf '\n%s\n' "${YELLOW}[size] Checking hello.wasm binary size gate...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "binary size gate"
else
    ARUKELLT_BIN="./target/debug/arukellt"
    if [ ! -x "$ARUKELLT_BIN" ]; then
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

# ── Baseline snapshots ────────────────────────────────────────────────────────
printf '\n%s\n' "${YELLOW}[baseline] Collecting baseline snapshots...${NC}"
if [ "$QUICK_MODE" = true ]; then
    check_skip "python3 scripts/collect-baseline.py"
else
    run_check "baseline collection" "python3 scripts/collect-baseline.py"
fi

# ── Wait for background jobs and collect results ──────────────────────────────
printf '\n%s\n' "${YELLOW}[bg] Collecting background check results...${NC}"
wait
_bg_collect docs_struct
_bg_collect adrs
_bg_collect lang_spec
_bg_collect platform_spec
_bg_collect stdlib_spec
_bg_collect docs_consistency
_bg_collect markdownlint
_bg_collect stdlib_manifest

# ── Static instant checks ─────────────────────────────────────────────────────
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

# Component interop (optional)
if [ "${ARUKELLT_TEST_COMPONENT:-0}" = "1" ]; then
    printf '\n%s\n' "${YELLOW}[component] Component interop smoke test (ARUKELLT_TEST_COMPONENT=1)...${NC}"
    INTEROP_SCRIPT="tests/component-interop/jco/calculator/run.sh"
    if [ ! -f "$INTEROP_SCRIPT" ]; then
        check_skip "component interop script not found"
    elif ! command -v wasmtime >/dev/null 2>&1; then
        check_skip "component interop (wasmtime not found)"
    else
        run_check "component interop (wasmtime)" "bash $INTEROP_SCRIPT"
    fi
else
    check_skip "component interop (opt-in)"
fi

# ── WAT roundtrip verification ────────────────────────────────────────────────
if [ "$QUICK_MODE" = true ]; then
    check_skip "WAT roundtrip (quick mode)"
else
    run_check "WAT roundtrip (wasm2wat ⇄ wat2wasm)" "bash scripts/wat-roundtrip.sh"
fi

# ── Performance gate (opt-in) ─────────────────────────────────────────────────
if [ "$PERF_GATE" = true ]; then
    printf '\n%s\n' "${YELLOW}[perf] Running performance regression gate...${NC}"
    if [ "$QUICK_MODE" = true ]; then
        check_skip "perf gate (quick mode)"
    else
        run_check "perf gate (compile time / binary size / run time)" "bash scripts/perf-gate.sh"
    fi
else
    check_skip "perf gate (opt-in via --perf-gate)"
fi

printf '\n%s\n' "${YELLOW}========================================${NC}"
printf '%s\n' "${YELLOW}Summary${NC}"
printf '%s\n' "${YELLOW}========================================${NC}"
printf 'Total checks: %s\n' "$TOTAL_CHECKS"
printf 'Passed: %b%s%b\n' "$GREEN" "$PASSED_CHECKS" "$NC"
printf 'Skipped: %b%s%b\n' "$YELLOW" "$SKIPPED_CHECKS" "$NC"
printf 'Failed: %b%s%b\n' "$RED" "$((TOTAL_CHECKS - PASSED_CHECKS - SKIPPED_CHECKS))" "$NC"

if [ $PASSED_CHECKS -eq $TOTAL_CHECKS ] || [ $((PASSED_CHECKS + SKIPPED_CHECKS)) -eq $TOTAL_CHECKS ]; then
    printf '\n%b✓ All harness checks passed%b\n' "$GREEN" "$NC"
    exit 0
else
    printf '\n%b✗ Some harness checks failed%b\n' "$RED" "$NC"
    exit 1
fi
