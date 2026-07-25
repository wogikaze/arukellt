(module
  ;; Benchmark prototype: call_ref with a typed function reference.
  ;; The ref.func is materialized once outside the loop, matching how a real
  ;; fn argument would be passed to a hot call site.
  (type $cb (func (param i32) (result i32)))
  (type $apply (func (param (ref $cb)) (param i32) (result i32)))

  ;; Declare the function so ref.func is valid without a table.
  (elem declare func $double)

  (func $double (type $cb) (param i32) (result i32)
    local.get 0
    i32.const 1
    i32.add)

  (func $apply (type $apply) (param (ref $cb) i32) (result i32)
    local.get 1
    local.get 0
    call_ref $cb)

  (func $bench (export "main") (result i32)
    (local $i i32) (local $acc i32) (local $f_ref (ref $cb))
    ref.func $double
    local.set $f_ref
    i32.const 0
    local.set $i
    i32.const 0
    local.set $acc
    loop $l
      local.get $f_ref
      local.get $acc
      call $apply
      local.set $acc
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      i32.const 10000000
      i32.lt_s
      br_if $l
    end
    local.get $acc)
)
