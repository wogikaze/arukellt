#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source_dir="$repo_root/codex-skills"
codex_home="${CODEX_HOME:-$HOME/.codex}"
target_dir="$codex_home/skills"
link_from_codex="${LINK_FROM_CODEX_SKILLS:-}"

if [[ "${1:-}" == "--link-from-codex" ]]; then
  link_from_codex="${2:-$HOME/.codex/skills}"
fi

mkdir -p "$target_dir"

for skill_dir in "$source_dir"/*; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  if [[ "$skill_name" == arukellt-* ]]; then
    install_name="$skill_name"
  else
    install_name="arukellt-$skill_name"
  fi
  rm -rf "$target_dir/$install_name"
  if [[ -n "$link_from_codex" ]]; then
    ln -s "$link_from_codex/$install_name" "$target_dir/$install_name"
  else
    cp -R "$skill_dir" "$target_dir/$install_name"
  fi
done

if [[ -n "$link_from_codex" ]]; then
  echo "Linked Arukellt Codex skills into $target_dir from $link_from_codex"
else
  echo "Installed Arukellt Codex skills to $target_dir"
fi
