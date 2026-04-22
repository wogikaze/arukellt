#!/usr/bin/env bash
# scripts/gate/install-git-hooks.sh — Install (or remove) repository-managed git hooks.
# Usage:
#   bash scripts/gate/install-git-hooks.sh           # install
#   bash scripts/gate/install-git-hooks.sh --remove  # remove
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"
PRE_COMMIT_TARGET="$HOOKS_DIR/pre-commit"
PRE_PUSH_TARGET="$HOOKS_DIR/pre-push"

if [ "${1:-}" = "--remove" ]; then
  removed=0
  for target in "$PRE_COMMIT_TARGET" "$PRE_PUSH_TARGET"; do
    if [ -f "$target" ]; then
      rm "$target"
      echo "Removed $target"
      removed=1
    fi
  done
  if [ "$removed" -eq 0 ]; then
    echo "No repository-managed hooks installed in $HOOKS_DIR"
  fi
  exit 0
fi

if [ ! -d "$HOOKS_DIR" ]; then
  echo "ERROR: $HOOKS_DIR not found. Are you in a git repository?" >&2
  exit 1
fi

cat > "$PRE_COMMIT_TARGET" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec bash "$(git rev-parse --show-toplevel)/scripts/gate/pre-commit-verify.sh"
EOF
chmod +x "$PRE_COMMIT_TARGET"
echo "Installed pre-commit hook -> $PRE_COMMIT_TARGET"

cat > "$PRE_PUSH_TARGET" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec bash "$(git rev-parse --show-toplevel)/scripts/gate/pre-push-branch-policy.sh" "$@"
EOF
chmod +x "$PRE_PUSH_TARGET"
echo "Installed pre-push hook -> $PRE_PUSH_TARGET"
