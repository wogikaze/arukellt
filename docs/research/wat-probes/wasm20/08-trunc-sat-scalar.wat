;; Wasm 2.0 — scalar non-trapping float-to-int (NOT SIMD)
;; Success: (invoke "test") => i32.const 2147483647  (+inf saturates)
(module
  (func (export "test") (result i32)
    (i32.trunc_sat_f32_s (f32.const inf))))
