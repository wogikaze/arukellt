// Generated from docs/data/project-state.toml by scripts/gen/generate-docs.py.
// Do not edit by hand.
module.exports = {
  "aliases": [
    {
      "canonical_target": "wasm32",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p1",
      "input": "wasm32-wasi-p1",
      "policy": "warning",
      "replacement": "--target wasm32 --wasi-version wasi-p1",
      "warning_id": "W0002"
    },
    {
      "canonical_target": "wasm32",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p1",
      "input": "wasm32-wasi",
      "policy": "warning",
      "replacement": "--target wasm32 --wasi-version wasi-p1",
      "warning_id": "W0002"
    },
    {
      "canonical_target": "wasm32-gc",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p2",
      "input": "wasm32-wasi-p2",
      "policy": "warning",
      "replacement": "--target wasm32-gc --wasi-version wasi-p2",
      "warning_id": "W0002"
    },
    {
      "canonical_target": "wasm32-gc",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p2",
      "input": "wasm-gc",
      "policy": "warning",
      "replacement": "--target wasm32-gc --wasi-version wasi-p2",
      "warning_id": "W0002"
    },
    {
      "canonical_target": "wasm32-gc",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p2",
      "input": "wasm-gc-wasi-p2",
      "policy": "warning",
      "replacement": "--target wasm32-gc --wasi-version wasi-p2",
      "warning_id": "W0002"
    },
    {
      "canonical_target": "wasm32-gc",
      "compatibility_status": "deprecated",
      "host_profile": "wasi-p3",
      "input": "wasm32-wasi-p3",
      "policy": "warning",
      "replacement": "--target wasm32-gc --wasi-version wasi-p3",
      "warning_id": "W0002"
    },
    {
      "compatibility_status": "rejected-ambiguous",
      "input": "native",
      "policy": "error",
      "replacement": "use --target native-cpp or --target native-llvm"
    },
    {
      "compatibility_status": "rejected",
      "input": "wasm32-freestanding",
      "policy": "error",
      "replacement": "choose a canonical target; freestanding is not supported"
    }
  ],
  "canonicalTargets": [
    "wasm32",
    "wasm32-gc",
    "native-cpp",
    "native-llvm"
  ],
  "componentTarget": "wasm32-gc",
  "defaultTarget": "wasm32-gc"
}
