(module
  ;; Baseline: untyped funcref table + call_indirect.
  (type $callback (func (param i32) (result i32)))
  (type $apply_t (func (param i32) (param i32) (result i32)))

  (table 1 funcref)
  (elem (i32.const 0) $double)

  (func $double (type $callback) (param i32) (result i32)
    local.get 0
    i32.const 2
    i32.mul)

  (func $apply (type $apply_t) (param i32 i32) (result i32)
    ;; f is table index (param 0), x is param 1
    local.get 1
    local.get 0
    call_indirect (type $callback))

  (func $main (export "main") (result i32)
    ;; apply(double_table_index=0, x=21)
    i32.const 0
    i32.const 21
    call $apply)
)
