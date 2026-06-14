;; Stub adapter for wasi:cli/exit@0.2.0 guest imports.
(module
  (type $exit (func (param i32)))
  (func (export "exit") (type $exit) (param i32))
)
