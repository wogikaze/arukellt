;; Wasm 3.0 — extended constant expressions (i32.add in global init)
;; Success: (invoke "test") => i32.const 42
(module
  (global $g i32 (i32.add (i32.const 20) (i32.const 22)))
  (func (export "test") (result i32)
    (global.get $g)))
