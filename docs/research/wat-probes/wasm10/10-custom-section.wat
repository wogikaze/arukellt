;; Wasm 1.0 — custom section via binary (no @custom annotation)
;; Text form without annotations; section injected by run-probes.py when needed.
;; This WAT itself is plain Wasm 1.0; companion binary with custom section is
;; produced in the harness. For standalone use, export test => 42.
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (i32.const 42)))
