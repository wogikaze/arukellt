(module
  ;; Benchmark baseline: call_indirect through an untyped funcref table.
  ;; The table index is loaded once outside the loop, matching how a real
  ;; fn parameter would be passed into a hot call site.
  (type $cb (func (param i32) (result i32)))
  (type $apply (func (param i32 i32) (result i32)))

  (table 1 funcref)
  (elem (i32.const 0) $double)

  (func $double (type $cb) (param i32) (result i32)
    local.get 0
    i32.const 1
    i32.add)

  (func $apply (type $apply) (param i32 i32) (result i32)
    local.get 1
    local.get 0
    call_indirect (type $cb))

  (func $bench (export "main") (result i32)
    (local $i i32) (local $acc i32) (local $f_idx i32)
    i32.const 0
    local.set $f_idx
    i32.const 0
    local.set $i
    i32.const 0
    local.set $acc
    loop $l
      local.get $f_idx
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
