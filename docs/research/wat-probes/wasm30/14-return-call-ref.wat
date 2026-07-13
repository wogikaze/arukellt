;; Wasm 3.0 — return_call_ref (typed indirect tail call)
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (elem declare func $f)
  (func $f (type $t) (i32.const 42))
  (func $g (result i32)
    (return_call_ref $t (ref.func $f)))
  (func (export "test") (result i32)
    (call $g)))
