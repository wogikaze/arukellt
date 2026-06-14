;; Minimal stdout adapt stub (no local memory) for component-new smoke tests.
(module
  (type $write (func (param i32 i32 i32 i32) (result i32)))
  (func (export "write") (type $write) (param i32 i32 i32 i32) (result i32)
    (i32.const 0)
  )
)
