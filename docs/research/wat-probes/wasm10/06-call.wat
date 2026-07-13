;; Wasm 1.0 — direct call
;; Success: (invoke "test") => i32.const 42
(module
  (func $add (param i32 i32) (result i32)
    (i32.add (local.get 0) (local.get 1)))
  (func (export "test") (result i32)
    (call $add (i32.const 20) (i32.const 22))))
