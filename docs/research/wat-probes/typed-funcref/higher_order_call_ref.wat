(module
  ;; Phase A prototype: typed function references + call_ref.
  ;; The fn parameter becomes (ref $callback) instead of an i32 table index.
  (type $callback (func (param i32) (result i32)))
  (type $apply_t (func (param (ref $callback)) (param i32) (result i32)))

  ;; Declare the function so ref.func can reference it without a table.
  (elem declare func $double)

  (func $double (type $callback) (param i32) (result i32)
    local.get 0
    i32.const 2
    i32.mul)

  (func $apply (type $apply_t) (param (ref $callback) i32) (result i32)
    ;; Push arg, then push ref, then call_ref with the function's type.
    local.get 1
    local.get 0
    call_ref $callback)

  (func $main (export "main") (result i32)
    ;; Pass double as a ref.func and 21 as the argument.
    ref.func $double
    i32.const 21
    call $apply)
)
