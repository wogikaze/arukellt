;; Wasm 1.0 — unreachable trap
;; Success: (invoke "test") traps (does not return)
(module
  (func (export "test") (result i32)
    (unreachable)
    (i32.const 0)))
