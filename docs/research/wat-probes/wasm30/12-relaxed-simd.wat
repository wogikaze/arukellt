;; Wasm 3.0 — relaxed SIMD (opcode recognition smoke)
;; Success: validate + (invoke "test") returns some i32 (exact lane result may vary)
(module
  (func (export "test") (result i32)
    (i32x4.extract_lane 0
      (i32x4.relaxed_trunc_f32x4_s (f32x4.splat (f32.const 1.5))))))
