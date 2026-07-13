;; Wasm 2.0 — bulk memory: passive data + memory.init
;; Success: (invoke "test") => i32.const 65  (ASCII 'A')
(module
  (memory 1)
  (data $d "A")
  (func (export "test") (result i32)
    (memory.init $d (i32.const 0) (i32.const 0) (i32.const 1))
    (i32.load8_u (i32.const 0))))
