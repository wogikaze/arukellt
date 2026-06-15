#!/usr/bin/env bash
# Shared helpers for examples/*/run.sh scripts.
# Source from a run script:  source "$(dirname "$0")/../common.sh"

examples_repo_root() {
    local dir
    dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    cd "$dir/.." && pwd
}

examples_find_arukellt() {
    local root="$1"
    if [[ -n "${ARUKELLT_BIN:-}" ]] && [[ -x "${ARUKELLT_BIN}" ]]; then
        echo "${ARUKELLT_BIN}"
        return 0
    fi
    local wrapper="$root/scripts/run/arukellt-selfhost.sh"
    if [[ -f "$wrapper" ]]; then
        echo "$wrapper"
        return 0
    fi
    if [[ -x "$root/target/release/arukellt" ]]; then
        echo "$root/target/release/arukellt"
        return 0
    fi
    return 1
}

examples_resolve_selfhost_wasm() {
    local root="$1"
    local profile="${2:-legacy}"
    if [[ -n "${ARUKELLT_SELFHOST_WASM:-}" ]] && [[ -f "${ARUKELLT_SELFHOST_WASM}" ]]; then
        echo "${ARUKELLT_SELFHOST_WASM}"
        return 0
    fi
    local -a order
    if [[ "$profile" == "modern" ]]; then
        order=(
            "$root/.build/selfhost/arukellt-s2.wasm"
            "$root/.bootstrap-build/arukellt-s2.wasm"
            "$root/bootstrap/arukellt-selfhost.wasm"
        )
    else
        order=(
            "$root/bootstrap/arukellt-selfhost.wasm"
            "$root/.bootstrap-build/arukellt-s2.wasm"
            "$root/.build/selfhost/arukellt-s2.wasm"
        )
    fi
    local cand
    for cand in "${order[@]}"; do
        if [[ -f "$cand" ]]; then
            echo "$cand"
            return 0
        fi
    done
    return 1
}

examples_compile() {
    local arukellt="$1"
    shift
    local profile="legacy"
    if [[ "${1:-}" == "modern" || "${1:-}" == "legacy" ]]; then
        profile="$1"
        shift
    fi
    if [[ -z "${ARUKELLT_SELFHOST_WASM:-}" ]]; then
        local root wasm
        root="$(examples_repo_root)"
        wasm="$(examples_resolve_selfhost_wasm "$root" "$profile" || true)"
        if [[ -n "$wasm" ]]; then
            export ARUKELLT_SELFHOST_WASM="$wasm"
        fi
    fi
    if [[ "$(basename "$arukellt")" == "arukellt-selfhost.sh" ]]; then
        bash "$arukellt" "$@"
    else
        "$arukellt" "$@"
    fi
}

examples_find_wasmtime() {
    if [[ -n "${WASMTIME_BIN:-}" ]] && [[ -x "${WASMTIME_BIN}" ]]; then
        echo "${WASMTIME_BIN}"
        return 0
    fi
    command -v wasmtime 2>/dev/null || return 1
}

examples_find_wasm_tools() {
    if [[ -n "${WASM_TOOLS_BIN:-}" ]] && [[ -x "${WASM_TOOLS_BIN}" ]]; then
        echo "${WASM_TOOLS_BIN}"
        return 0
    fi
    if [[ -x "$HOME/.cargo/bin/wasm-tools" ]]; then
        echo "$HOME/.cargo/bin/wasm-tools"
        return 0
    fi
    command -v wasm-tools 2>/dev/null || return 1
}

examples_ensure_wasi_adapter() {
    local out="$1"
    if [[ -f "$out" ]]; then
        return 0
    fi
    mkdir -p "$(dirname "$out")"
    curl -fsSL -o "$out" \
        "https://github.com/bytecodealliance/wasmtime/releases/download/v39.0.1/wasi_snapshot_preview1.reactor.wasm"
}
