;; Wasm 2.0 — JS BigInt <-> i64 (JS embedding probe)
;; Success (Node/Chrome): WebAssembly.instantiate + test(1n) === 1n
;; Note: core wasm alone cannot verify JS BigInt integration.
(module
  (func (export "test") (param i64) (result i64)
    (local.get 0)))
