;; Wasm 1.0 — start function runs at instantiate
;; Success: after instantiate, (invoke "test") => i32.const 42
(module
  (global $g (mut i32) (i32.const 0))
  (func $start
    (global.set $g (i32.const 42)))
  (start $start)
  (func (export "test") (result i32)
    (global.get $g)))
