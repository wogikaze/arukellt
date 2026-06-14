;; Stub adapter for wasi:cli/environment@0.2.0 guest imports.
(module
  (type $bin (func (param i32 i32) (result i32)))
  (func (export "args-sizes") (type $bin) (param i32 i32) (result i32) (i32.const 0))
  (func (export "arguments") (type $bin) (param i32 i32) (result i32) (i32.const 0))
)
