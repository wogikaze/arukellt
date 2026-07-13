;; Wasm 3.0 — multiple memories + cross-memory copy
;; Success: (invoke "test") => i32.const 42
(module
  (memory $m0 1)
  (memory $m1 1)
  (func (export "test") (result i32)
    (i32.store $m0 (i32.const 0) (i32.const 42))
    (memory.copy $m1 $m0 (i32.const 0) (i32.const 0) (i32.const 4))
    (i32.load $m1 (i32.const 0))))
