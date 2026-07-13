;; Wasm 2.0 — multiple tables (active elem on table 1; no typed-ref cast)
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (func $f (result i32) (i32.const 42))
  (table $t0 1 funcref)
  (table $t1 1 funcref)
  (elem (table $t1) (i32.const 0) func $f)
  (func (export "test") (result i32)
    (call_indirect $t1 (type $t) (i32.const 0))))
