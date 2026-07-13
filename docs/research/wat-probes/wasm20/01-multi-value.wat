;; Wasm 2.0 — multi-value return + block params
;; Success: (invoke "test") => i32.const 42  (uses both results: 20+22)
(module
  (func $pair (result i32 i32)
    (i32.const 20) (i32.const 22))
  (func (export "test") (result i32)
    (call $pair)
    (i32.add)))
