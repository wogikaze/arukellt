;; Wasm 3.0 — Memory64 (i64 address)
;; Success: (invoke "test") => i32.const 42
(module
  (memory i64 1)
  (func (export "test") (result i32)
    (i32.store (i64.const 0) (i32.const 42))
    (i32.load (i64.const 0))))
