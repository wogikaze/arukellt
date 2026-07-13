;; Wasm 1.0 — mutable/immutable globals
;; Success: (invoke "test") => i32.const 42
(module
  (global $imm i32 (i32.const 20))
  (global $mut (mut i32) (i32.const 0))
  (func (export "test") (result i32)
    (global.set $mut (i32.const 22))
    (i32.add (global.get $imm) (global.get $mut))))
