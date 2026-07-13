;; Wasm 2.0 — SIMD trunc_sat (separate from scalar)
;; Success: (invoke "test") => i32.const 2147483647
(module
  (func (export "test") (result i32)
    (i32x4.extract_lane 0
      (i32x4.trunc_sat_f32x4_s (f32x4.splat (f32.const inf))))))
