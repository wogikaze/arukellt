;; Wasm 1.0 — table + call_indirect + active element segment
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (func $f (result i32) (i32.const 42))
  (table 1 funcref)
  (elem (i32.const 0) $f)
  (func (export "test") (result i32)
    (call_indirect (type $t) (i32.const 0))))
