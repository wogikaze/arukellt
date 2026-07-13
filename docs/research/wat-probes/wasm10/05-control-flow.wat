;; Wasm 1.0 — block / loop / if / br / br_if / br_table / return
;; Success: (invoke "test") => i32.const 42
(module
  (func (export "test") (result i32)
    (block $exit (result i32)
      (block $a
        (block $b
          (br_table $a $b $a (i32.const 0)))
        ;; $b fallthrough
        (br $exit (i32.const 0)))
      ;; $a fallthrough
      (if (result i32) (i32.const 1)
        (then
          (loop $L (result i32)
            (br_if $exit (i32.const 42) (i32.const 1))
            (br $L)))
        (else (i32.const 0))))))
