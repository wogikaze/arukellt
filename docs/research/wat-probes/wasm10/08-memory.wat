;; Wasm 1.0 — linear memory load/store/grow + active data segment
;; Success: (invoke "test") => i32.const 42
(module
  (memory 1)
  (data (i32.const 0) "\2A") ;; 42
  (func (export "test") (result i32)
    (i32.store (i32.const 4) (i32.const 42))
    (drop (memory.grow (i32.const 1)))
    (i32.load (i32.const 4))))
