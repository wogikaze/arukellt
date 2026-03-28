#!/bin/bash
# Performance gate: detect compile-time, execution-time, and binary-size regressions.
#
# Usage:  scripts/perf-gate.sh [--update]
#   --update   Write current measurements as new baselines (no comparison).
#
# Thresholds (vs baseline):
#   compile time  +20%  → failure
#   execution time +10% → failure
#   binary size   +15%  → failure

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

BASELINE_FILE="tests/baselines/perf/baselines.json"
UPDATE_MODE=false
if [[ "${1:-}" == "--update" ]]; then
    UPDATE_MODE=true
fi

# ── Locate compiler binary ────────────────────────────────────────────────────
ARUKELLT_BIN="./target/debug/arukellt"
if [ ! -x "$ARUKELLT_BIN" ]; then
    ARUKELLT_BIN="./target/release/arukellt"
fi
if [ ! -x "$ARUKELLT_BIN" ]; then
    echo -e "${RED}✗ arukellt binary not found. Run: cargo build -p arukellt${NC}"
    exit 1
fi

if ! command -v wasmtime >/dev/null 2>&1; then
    echo -e "${RED}✗ wasmtime not found in PATH${NC}"
    exit 1
fi

# ── Run measurements and comparison via Python ────────────────────────────────
exec python3 - "$ARUKELLT_BIN" "$BASELINE_FILE" "$UPDATE_MODE" <<'PYEOF'
import json, os, subprocess, sys, statistics, time

arukellt = sys.argv[1]
baseline_path = sys.argv[2]
update_mode = sys.argv[3].lower() == "true"

RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
NC = "\033[0m"

COMPILE_THRESHOLD = 20   # percent
RUN_THRESHOLD     = 10   # percent
SIZE_THRESHOLD    = 15   # percent
ITERATIONS        = 5    # median of N runs

FIXTURES = {
    "hello":        "tests/fixtures/hello/hello.ark",
    "fibonacci":    "tests/fixtures/integration/fibonacci.ark",
    "higher_order": "tests/fixtures/functions/higher_order.ark",
}

def measure(name, src):
    """Measure compile time (ms), binary size (bytes), and run time (ms)."""
    wasm_out = f"_perfgate_{name}.wasm"
    compile_times = []
    run_times = []

    for _ in range(ITERATIONS):
        # Compile
        t0 = time.monotonic()
        r = subprocess.run([arukellt, "compile", src, "-o", wasm_out],
                           capture_output=True)
        compile_times.append((time.monotonic() - t0) * 1000)
        if r.returncode != 0:
            print(f"{RED}✗ compilation failed for {name}: {r.stderr.decode()[:200]}{NC}")
            try: os.unlink(wasm_out)
            except FileNotFoundError: pass
            return None

        # Execute
        t0 = time.monotonic()
        subprocess.run(["wasmtime", wasm_out], capture_output=True)
        run_times.append((time.monotonic() - t0) * 1000)

    binary_bytes = os.path.getsize(wasm_out)
    os.unlink(wasm_out)

    return {
        "compile_ms":   int(statistics.median(compile_times)),
        "binary_bytes": binary_bytes,
        "run_ms":       int(statistics.median(run_times)),
    }


def check_threshold(label, current, baseline, pct):
    """Return True if within threshold, False if regression detected."""
    limit = int(baseline * (1 + pct / 100))
    if current <= limit:
        print(f"  {GREEN}✓{NC} {label}: {current} (baseline {baseline}, limit {limit}, +{pct}%)")
        return True
    else:
        delta = int((current - baseline) / baseline * 100) if baseline > 0 else 999
        print(f"  {RED}✗{NC} {label}: {current} (baseline {baseline}, limit {limit}, +{delta}% > +{pct}%)")
        return False


# ── Main ──────────────────────────────────────────────────────────────────────
print(f"{YELLOW}Performance gate ({ITERATIONS}-iteration median)...{NC}\n")

# Measure all fixtures
results = {}
for name, src in FIXTURES.items():
    print(f"{YELLOW}Measuring {name} ({src})...{NC}")
    m = measure(name, src)
    if m is None:
        sys.exit(1)
    results[name] = m
    print(f"  compile_ms={m['compile_ms']}  binary_bytes={m['binary_bytes']}  run_ms={m['run_ms']}")

# ── Update mode ───────────────────────────────────────────────────────────────
if update_mode:
    os.makedirs(os.path.dirname(baseline_path), exist_ok=True)
    with open(baseline_path, "w") as f:
        json.dump(results, f, indent=2)
        f.write("\n")
    print(f"\n{GREEN}✓ Baselines written to {baseline_path}{NC}")
    sys.exit(0)

# ── Comparison mode ───────────────────────────────────────────────────────────
if not os.path.isfile(baseline_path):
    print(f"{RED}✗ Baseline file not found: {baseline_path}{NC}")
    print(f"  Run: scripts/update-perf-baselines.sh")
    sys.exit(1)

with open(baseline_path) as f:
    baselines = json.load(f)

print(f"\n{YELLOW}Comparing against baselines ({baseline_path})...{NC}\n")

failures = 0
passes = 0
for name in FIXTURES:
    if name not in baselines:
        print(f"{RED}✗ No baseline for '{name}' — run: scripts/update-perf-baselines.sh{NC}")
        failures += 1
        continue

    bl = baselines[name]
    cur = results[name]
    print(f"[{name}]")

    if check_threshold("compile time", cur["compile_ms"], bl["compile_ms"], COMPILE_THRESHOLD):
        passes += 1
    else:
        failures += 1

    if check_threshold("binary size",  cur["binary_bytes"], bl["binary_bytes"], SIZE_THRESHOLD):
        passes += 1
    else:
        failures += 1

    if check_threshold("run time",     cur["run_ms"], bl["run_ms"], RUN_THRESHOLD):
        passes += 1
    else:
        failures += 1

    print()

total = passes + failures
print(f"Perf gate: {passes}/{total} checks passed, {failures} failed")

if failures > 0:
    print(f"\n{RED}✗ Performance regression detected!{NC}")
    print(f"  If intentional, update baselines: scripts/update-perf-baselines.sh")
    sys.exit(1)
else:
    print(f"\n{GREEN}✓ All performance checks passed{NC}")
    sys.exit(0)
PYEOF
