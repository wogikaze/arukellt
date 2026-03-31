#!/usr/bin/env bash
set -euo pipefail

HOOKS_DIR=$(git rev-parse --git-path hooks)
REPO_ROOT=$(git rev-parse --show-toplevel)

# Managed block configuration
MANAGED_BEGIN_PREFIX="# arukellt-managed-"
MANAGED_END_PREFIX="# arukellt-managed-"
MANAGED_SUFFIX="-begin"
MANAGED_END_SUFFIX="-end"

remove_managed_block() {
    local hook_path="$1"
    local hook_name="$2"
    local begin="${MANAGED_BEGIN_PREFIX}${hook_name}${MANAGED_SUFFIX}"
    local end="${MANAGED_END_PREFIX}${hook_name}${MANAGED_END_SUFFIX}"

    if [ ! -f "$hook_path" ]; then
        return
    fi

    python3 - <<'PY' "$hook_path" "$begin" "$end"
from pathlib import Path
import sys
path = Path(sys.argv[1])
begin = sys.argv[2]
end = sys.argv[3]
text = path.read_text()
start = text.find(begin)
if start == -1:
    sys.exit(0)
finish = text.find(end, start)
if finish == -1:
    sys.exit(0)
finish = text.find('\n', finish)
if finish == -1:
    finish = len(text)
else:
    finish += 1
new = text[:start].rstrip('\n')
if new:
    new += '\n'
new += text[finish:].lstrip('\n')
path.write_text(new)
PY
}

install_hook() {
    local hook_name="$1"
    local script_path="$2"
    local hook_path="$HOOKS_DIR/$hook_name"
    local begin="${MANAGED_BEGIN_PREFIX}${hook_name}${MANAGED_SUFFIX}"
    local end="${MANAGED_END_PREFIX}${hook_name}${MANAGED_END_SUFFIX}"

    mkdir -p "$HOOKS_DIR"
    if [ ! -f "$hook_path" ]; then
        cat > "$hook_path" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
EOF
    fi

    remove_managed_block "$hook_path" "$hook_name"

    printf '\n%s\nrepo_root=$(git rev-parse --show-toplevel)\n"$repo_root/%s"\n%s\n' \
        "$begin" "$script_path" "$end" >> "$hook_path"
    chmod +x "$hook_path"

    echo "Installed repository-managed $hook_name hook in $hook_path"
    echo "It runs $script_path before $hook_name."
}

if [ "${1:-}" = "--remove" ]; then
    remove_managed_block "$HOOKS_DIR/pre-push" "pre-push"
    remove_managed_block "$HOOKS_DIR/pre-commit" "pre-commit"
    echo "Removed repository-managed hooks from $HOOKS_DIR"
    exit 0
fi

# Pre-commit: format check and docs consistency
install_hook "pre-commit" "scripts/pre-commit-verify.sh"

# Pre-push: full harness verification
install_hook "pre-push" "scripts/pre-push-verify.sh"

echo ""
echo "Git hooks installed successfully."
echo "Format and docs are checked on commit; full verification happens on push."
