;; Wasm 2.0 — reference types: funcref / externref / ref.null / ref.is_null
;; Success: (invoke "test") => i32.const 1
(module
  (func (export "test") (result i32)
    (ref.is_null (ref.null extern))))
