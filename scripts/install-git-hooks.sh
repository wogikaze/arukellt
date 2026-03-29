#!/usr/bin/env bash
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR=$(git rev-parse --git-path hooks)
HOOK_PATH="$HOOKS_DIR/pre-commit"
MANAGED_BEGIN="# arukellt-managed-pre-commit-begin"
MANAGED_END="# arukellt-managed-pre-commit-end"
SNIPPET=$(cat <<'EOF'
# arukellt-managed-pre-commit-begin
repo_root=$(git rev-parse --show-toplevel)
"$repo_root/scripts/pre-commit-verify.sh"
# arukellt-managed-pre-commit-end
EOF
)

remove_managed_block() {
    if [ ! -f "$HOOK_PATH" ]; then
        return
    fi
    python3 - <<'PY' "$HOOK_PATH" "$MANAGED_BEGIN" "$MANAGED_END"
from pathlib import Path
import sys
path = Path(sys.argv[1])
begin = sys.argv[2]
end = sys.argv[3]
text = path.read_text()
start = text.find(begin)
if start == -1:
    raise SystemExit(0)
finish = text.find(end, start)
if finish == -1:
    raise SystemExit(0)
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

if [ "${1:-}" = "--remove" ]; then
    remove_managed_block
    echo "Removed repository-managed pre-commit hook block from $HOOK_PATH"
    exit 0
fi

mkdir -p "$HOOKS_DIR"
if [ ! -f "$HOOK_PATH" ]; then
    cat > "$HOOK_PATH" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
EOF
fi

remove_managed_block
printf '\n%s\n' "$SNIPPET" >> "$HOOK_PATH"
chmod +x "$HOOK_PATH"

echo "Installed repository-managed pre-commit hook in $HOOK_PATH"
echo "It runs scripts/pre-commit-verify.sh before commit."
