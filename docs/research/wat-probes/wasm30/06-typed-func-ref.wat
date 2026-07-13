;; Wasm 3.0 — typed function references + call_ref
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (elem declare func $f)
  (func $f (type $t) (i32.const 42))
  (func (export "test") (result i32)
    (call_ref $t (ref.func $f))))
