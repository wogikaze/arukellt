;; Experimental — threads/atomics (NOT part of Wasm 3.0 Core)
;; Success: (invoke "test") => i32.const 42 under shared-memory-enabled runtime
(module
  (memory 1 1 shared)
  (func (export "test") (result i32)
    (i32.atomic.store (i32.const 0) (i32.const 42))
    (i32.atomic.load (i32.const 0))))
