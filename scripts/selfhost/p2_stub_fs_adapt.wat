;; Stub adapter for wasi:filesystem/types@0.2.0 guest imports.
(module
  (type $open (func (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))
  (type $close (func (param i32) (result i32)))
  (func (export "open-at") (type $open) (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32) (i32.const 0))
  (func (export "close") (type $close) (param i32) (result i32) (i32.const 0))
)
