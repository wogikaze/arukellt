;; Wasm 2.0 — typed select on references
;; Success: (invoke "test") => i32.const 1  (selected null is null)
(module
  (func (export "test") (result i32)
    (ref.is_null
      (select (result externref)
        (ref.null extern)
        (ref.null extern)
        (i32.const 1)))))
