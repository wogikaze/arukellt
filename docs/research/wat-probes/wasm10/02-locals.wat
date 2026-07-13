;; Wasm 1.0 — local.get / local.set / local.tee
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (local $x i32)
    (local.set $x (i32.const 40))
    (drop (local.tee $x (i32.add (local.get $x) (i32.const 1))))
    (i32.add (local.get $x) (i32.const 1))))
