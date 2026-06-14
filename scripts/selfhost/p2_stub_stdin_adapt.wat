;; Stub adapter for wasi:cli/stdin@0.2.0 guest imports.
(module
  (type $read (func (param i32 i32 i32 i32) (result i32)))
  (func (export "read") (type $read) (param i32 i32 i32 i32) (result i32) (i32.const 0))
)
