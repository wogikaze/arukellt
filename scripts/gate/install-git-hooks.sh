#!/usr/bin/env bash
# scripts/gate/install-git-hooks.sh — Install (or remove) repository-managed git hooks.
# Usage:
#   bash scripts/gate/install-git-hooks.sh           # install
#   bash scripts/gate/install-git-hooks.sh --remove  # remove
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"
TARGET="$HOOKS_DIR/pre-commit"

if [ "${1:-}" = "--remove" ]; then
  if [ -f "$TARGET" ]; then
    rm "$TARGET"
    echo "Removed $TARGET"
  else
    echo "No hook installed at $TARGET"
  fi
  exit 0
fi

if [ ! -d "$HOOKS_DIR" ]; then
  echo "ERROR: $HOOKS_DIR not found. Are you in a git repository?" >&2
  exit 1
fi

cat > "$TARGET" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec bash "$(git rev-parse --show-toplevel)/scripts/gate/pre-commit-verify.sh"
EOF
chmod +x "$TARGET"
echo "Installed pre-commit hook -> $TARGET"
