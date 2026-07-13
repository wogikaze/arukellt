;; Wasm 3.0 — reference narrowing: br_on_null
;; Success: (invoke "test") => i32.const 42
(module
  (type $t (func (result i32)))
  (elem declare func $f)
  (func $f (type $t) (i32.const 42))
  (func (export "test") (result i32)
    (block $is_null
      (ref.func $f)
      (br_on_null $is_null)
      (return (call_ref $t)))
    (i32.const 0)))
