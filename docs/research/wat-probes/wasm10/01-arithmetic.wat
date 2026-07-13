;; Wasm 1.0 — numeric types + arithmetic
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (i32.add (i32.const 20) (i32.const 22))))
