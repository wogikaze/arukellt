;; Wasm 1.0 — drop / select
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (i32.const 99)
    (drop)
    (select (i32.const 42) (i32.const 0) (i32.const 1))))
