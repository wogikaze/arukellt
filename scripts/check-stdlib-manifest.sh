#!/usr/bin/env bash
# Verify stdlib manifest matches resolve/typecheck/prelude.ark
# Exits non-zero if any drift detected.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$REPO_ROOT/std/manifest.toml"
RESOLVE="$REPO_ROOT/crates/ark-resolve/src/resolve.rs"
CHECKER="$REPO_ROOT/crates/ark-typecheck/src/checker.rs"
PRELUDE="$REPO_ROOT/std/prelude.ark"

errors=0

# ── Helpers ──────────────────────────────────────────────────────────

# Extract function names from manifest (only from [[functions]] sections).
# Excludes names starting with __ (intrinsics listed for completeness only).
manifest_fn_names() {
    awk '/^\[\[functions\]\]/{in_fn=1; next} /^\[\[/{in_fn=0} in_fn && /^name = /{gsub(/^name = "|"$/,"",$0); print}' "$MANIFEST" \
        | grep -v '^__intrinsic_' \
        | sort -u
}

# All names from manifest [[functions]] (including __intrinsic_ entries)
# plus all intrinsic= values (the backing __intrinsic_* names).
manifest_all_names() {
    {
        awk '/^\[\[functions\]\]/{in_fn=1; next} /^\[\[/{in_fn=0} in_fn && /^name = /{gsub(/^name = "|"$/,"",$0); print}' "$MANIFEST"
        grep '^intrinsic = ' "$MANIFEST" | sed 's/^intrinsic = "\(.*\)"/\1/'
    } | sort -u
}

# Non-intrinsic entries from PRELUDE_FUNCTIONS in resolve.rs
resolve_public_names() {
    sed -n '/^const PRELUDE_FUNCTIONS/,/^\];/p' "$RESOLVE" \
        | grep '"' \
        | sed 's/.*"\(.*\)".*/\1/' \
        | grep -v '^__intrinsic_' \
        | sort -u
}

# Intrinsic entries from PRELUDE_FUNCTIONS in resolve.rs
resolve_intrinsic_names() {
    sed -n '/^const PRELUDE_FUNCTIONS/,/^\];/p' "$RESOLVE" \
        | grep '"' \
        | sed 's/.*"\(.*\)".*/\1/' \
        | grep '^__intrinsic_' \
        | sort -u
}

# fn_sigs.insert names from register_builtins() in checker.rs
# We extract the section between `pub fn register_builtins` and the closing `}`
# at the same indent level, then grab the insert key strings.
checker_fnsig_names() {
    # Pattern in checker.rs:
    #   self.fn_sigs.insert(
    #       "name".into(),
    # Grab the line after each fn_sigs.insert( and extract the quoted name.
    sed -n '/pub fn register_builtins/,/^    }/p' "$CHECKER" \
        | grep -A1 'fn_sigs.insert(' \
        | grep '\.into()' \
        | sed 's/.*"\(.*\)"\.into().*/\1/' \
        | sort -u
}

# Public fn names from prelude.ark
prelude_fn_names() {
    grep '^pub fn ' "$PRELUDE" \
        | sed 's/^pub fn \([a-zA-Z0-9_]*\).*/\1/' \
        | sort -u
}

# ── Check 1: Every public name in PRELUDE_FUNCTIONS is in the manifest ──

echo -e "${YELLOW}[1/4] Checking resolve.rs public names vs manifest...${NC}"
diff_out=$(diff <(resolve_public_names) <(manifest_fn_names) || true)
if [ -n "$diff_out" ]; then
    # Lines starting with < are in resolve.rs but not in manifest
    missing_from_manifest=$(echo "$diff_out" | grep '^< ' | sed 's/^< //' || true)
    # Lines starting with > are in manifest but not in resolve.rs
    extra_in_manifest=$(echo "$diff_out" | grep '^> ' | sed 's/^> //' || true)
    if [ -n "$missing_from_manifest" ]; then
        echo -e "${RED}  Functions in PRELUDE_FUNCTIONS but NOT in manifest:${NC}"
        echo "$missing_from_manifest" | sed 's/^/    /'
        errors=$((errors + 1))
    fi
    if [ -n "$extra_in_manifest" ]; then
        echo -e "${YELLOW}  Functions in manifest but NOT in PRELUDE_FUNCTIONS (may be intentional):${NC}"
        echo "$extra_in_manifest" | sed 's/^/    /'
        # Not an error — manifest may include checker-only entries like f32_to_string
    fi
else
    echo -e "${GREEN}  ✓ All PRELUDE_FUNCTIONS public names present in manifest${NC}"
fi

# ── Check 2: Every intrinsic in PRELUDE_FUNCTIONS is backed by a manifest entry ──

echo -e "${YELLOW}[2/4] Checking resolve.rs intrinsic names vs manifest...${NC}"
manifest_intrinsics=$(grep '^intrinsic = ' "$MANIFEST" | sed 's/^intrinsic = "\(.*\)"/\1/' | sort -u)
# Also include any directly-listed __intrinsic_ names
manifest_intrinsics=$(printf '%s\n%s' "$manifest_intrinsics" "$(manifest_all_names | grep '^__intrinsic_')" | sort -u)
resolve_intrinsics=$(resolve_intrinsic_names)

diff_intr=$(diff <(echo "$resolve_intrinsics") <(echo "$manifest_intrinsics") || true)
if [ -n "$diff_intr" ]; then
    missing=$(echo "$diff_intr" | grep '^< ' | sed 's/^< //' || true)
    if [ -n "$missing" ]; then
        echo -e "${RED}  Intrinsics in PRELUDE_FUNCTIONS but NOT in manifest:${NC}"
        echo "$missing" | sed 's/^/    /'
        errors=$((errors + 1))
    else
        echo -e "${GREEN}  ✓ All PRELUDE_FUNCTIONS intrinsic names accounted for in manifest${NC}"
    fi
else
    echo -e "${GREEN}  ✓ All PRELUDE_FUNCTIONS intrinsic names accounted for in manifest${NC}"
fi

# ── Check 3: Every prelude.ark function is in the manifest ──

echo -e "${YELLOW}[3/4] Checking prelude.ark fn names vs manifest...${NC}"
prelude_names=$(prelude_fn_names)
manifest_names=$(manifest_fn_names)

missing_prelude=""
while IFS= read -r name; do
    if ! echo "$manifest_names" | grep -qx "$name"; then
        missing_prelude="$missing_prelude $name"
    fi
done <<< "$prelude_names"

if [ -n "$missing_prelude" ]; then
    echo -e "${RED}  Functions in prelude.ark but NOT in manifest:${NC}"
    echo "$missing_prelude" | tr ' ' '\n' | grep -v '^$' | sed 's/^/    /'
    errors=$((errors + 1))
else
    echo -e "${GREEN}  ✓ All prelude.ark functions present in manifest${NC}"
fi

# ── Check 4: Every checker.rs FnSig name is in the manifest ──

echo -e "${YELLOW}[4/4] Checking checker.rs FnSig names vs manifest...${NC}"
checker_names=$(checker_fnsig_names)
all_manifest=$(manifest_all_names)

# Exclude Some/Ok/Err — these are variant constructors, tracked as values
missing_checker=""
while IFS= read -r name; do
    case "$name" in Some|Ok|Err) continue ;; esac
    if ! echo "$all_manifest" | grep -qx "$name"; then
        missing_checker="$missing_checker $name"
    fi
done <<< "$checker_names"

if [ -n "$missing_checker" ]; then
    echo -e "${RED}  FnSigs in checker.rs but NOT in manifest:${NC}"
    echo "$missing_checker" | tr ' ' '\n' | grep -v '^$' | sed 's/^/    /'
    errors=$((errors + 1))
else
    echo -e "${GREEN}  ✓ All checker.rs FnSig names present in manifest${NC}"
fi

# ── Summary ──────────────────────────────────────────────────────────

echo ""
if [ "$errors" -gt 0 ]; then
    echo -e "${RED}✗ Stdlib manifest check failed ($errors error(s))${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Stdlib manifest is in sync${NC}"
    exit 0
fi
