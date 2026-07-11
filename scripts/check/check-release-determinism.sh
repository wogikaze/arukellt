#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
compiler="$repo_root/scripts/run/arukellt-selfhost.sh"
source_file="docs/examples/hello.ark"
output_dir=".build/release-checks"
first="$output_dir/determinism-1.wasm"
second="$output_dir/determinism-2.wasm"

cd "$repo_root"
mkdir -p "$output_dir"
"$compiler" compile "$source_file" --target wasm32-gc -o "$first"
"$compiler" compile "$source_file" --target wasm32-gc -o "$second"
cmp "$first" "$second"
