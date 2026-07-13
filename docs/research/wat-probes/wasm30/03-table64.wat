;; Wasm 3.0 — Table64 (i64 index type)
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (func $f (result i32) (i32.const 42))
  (table $T i64 funcref (elem $f))
  (func (export "test") (result i32)
    (call_indirect $T (type $t) (i64.const 0))))
