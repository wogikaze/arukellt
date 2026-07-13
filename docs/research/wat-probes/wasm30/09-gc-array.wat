;; Wasm 3.0 — GC array
;; Success: (invoke "test") => i32.const 42
(module
  (type $A (array (mut i32)))
  (func (export "test") (result i32)
    (local $a (ref $A))
    (local.set $a (array.new $A (i32.const 0) (i32.const 1)))
    (array.set $A (local.get $a) (i32.const 0) (i32.const 42))
    (array.get $A (local.get $a) (i32.const 0))))
