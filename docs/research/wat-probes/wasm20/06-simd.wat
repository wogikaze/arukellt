;; Wasm 2.0 — SIMD v128 splat + extract_lane
;; Success: (invoke "test") => i32.const 3
(module
  (func (export "test") (result i32)
    (i32x4.extract_lane 0 (i32x4.splat (i32.const 3)))))
