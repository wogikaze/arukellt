;; Wasm 1.0 — conversions + reinterpret
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (i32.trunc_f32_s (f32.const 41.9))
    (i32.add (i32.const 1))
    (drop)
    ;; reinterpret round-trip: i32 42 -> f32 bits -> i32
    (i32.reinterpret_f32 (f32.reinterpret_i32 (i32.const 42)))))
