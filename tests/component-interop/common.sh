#!/usr/bin/env bash
# Shared compiler resolution for component interop gates (#667).
#
# Forces recompile through the current-source selfhost wasm (s2/s3), not
# target/debug/arukellt or the pinned bootstrap reference.
set -euo pipefail

interop_repo_root() {
    if [[ -n "${REPO_ROOT:-}" ]]; then
        return 0
    fi
    local here
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    REPO_ROOT="$(cd "$here/../.." && pwd)"
}

interop_setup_s2_compiler() {
    interop_repo_root
    local strict="${INTEROP_STRICT:-1}"
    local build_flag=()
    if [[ "${INTEROP_BUILD_S2:-1}" == "1" ]]; then
        build_flag=(--ensure)
    fi

    ARUKELLT="${ARUKELLT_BIN:-$REPO_ROOT/scripts/run/arukellt-selfhost.sh}"
    if [[ ! -f "$ARUKELLT" ]]; then
        if [[ "$strict" == "1" ]]; then
            echo "FAIL: missing selfhost wrapper at $ARUKELLT" >&2
            exit 1
        fi
        echo "SKIP: arukellt selfhost wrapper missing"
        exit 0
    fi

    local wasm
    if ! wasm="$(python3 "$REPO_ROOT/scripts/lib/selfhost_s2.py" "${build_flag[@]}" --print-path 2>&1)"; then
        if [[ "$strict" == "1" ]]; then
            echo "FAIL: $wasm" >&2
            exit 1
        fi
        echo "SKIP: current selfhost wasm unavailable"
        exit 0
    fi
    export ARUKELLT_SELFHOST_WASM="$wasm"
    export ARUKELLT_BIN="$ARUKELLT"
}

interop_compile_component() {
    local source_rel="$1"
    local out_rel="$2"
    shift 2 || true
    bash "$ARUKELLT" compile \
        --emit component \
        --target wasm32-gc \
        "$@" \
        "$source_rel" \
        -o "$out_rel"
}

interop_compile_wit() {
    local source_rel="$1"
    local out_rel="$2"
    shift 2 || true
    bash "$ARUKELLT" compile \
        --target wasm32-gc \
        --emit wit \
        "$@" \
        "$source_rel" \
        -o "$out_rel"
}
