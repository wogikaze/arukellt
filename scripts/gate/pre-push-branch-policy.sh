#!/usr/bin/env bash
# scripts/gate/pre-push-branch-policy.sh — Prevent accidental non-master branch pushes.
set -euo pipefail

remote_name="${1:-origin}"
remote_url="${2:-}"

if [ "${ARUKELLT_ALLOW_BRANCH_PUSH:-}" = "1" ]; then
  echo "pre-push: ARUKELLT_ALLOW_BRANCH_PUSH=1 set; allowing branch push policy bypass." >&2
  exit 0
fi

fail=0

while read -r local_ref local_oid remote_ref remote_oid; do
  case "$local_ref" in
    refs/heads/master)
      ;;
    refs/heads/*)
      branch="${local_ref#refs/heads/}"
      echo "FAIL: refusing to push branch '$branch' to $remote_name ($remote_url)." >&2
      echo "      Arukellt keeps GitHub branch state centered on master." >&2
      echo "      Merge locally, then push master. If this is intentional, rerun with ARUKELLT_ALLOW_BRANCH_PUSH=1." >&2
      fail=1
      ;;
  esac
done

if [ "$fail" -ne 0 ]; then
  exit 1
fi

exit 0
