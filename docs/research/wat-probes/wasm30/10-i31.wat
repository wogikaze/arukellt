;; Wasm 3.0 — i31ref
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (i31.get_s (ref.i31 (i32.const 42)))))
