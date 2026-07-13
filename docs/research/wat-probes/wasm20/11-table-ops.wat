;; Wasm 2.0 — table.copy / table.fill / table.grow / table.size / elem.drop
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (func $f (result i32) (i32.const 42))
  (table $t0 1 funcref)
  (elem $e funcref (ref.func $f))
  (func (export "test") (result i32)
    (table.grow $t0 (ref.null func) (i32.const 1))
    (drop)
    (table.init $t0 $e (i32.const 0) (i32.const 0) (i32.const 1))
    (elem.drop $e)
    (call_indirect $t0 (type $t) (i32.const 0))))
