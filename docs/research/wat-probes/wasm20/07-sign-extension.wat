;; Wasm 2.0 — sign-extension operators
;; Success: (invoke "test") => i32.const -1
(module
  (func (export "test") (result i32)
    (i32.extend8_s (i32.const 255))))
