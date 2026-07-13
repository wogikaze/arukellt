;; Wasm 3.0 — recursive types + subtyping smoke
;; Success: validate + (invoke "test") => i32.const 42
(module
  (rec
    (type $A (struct (field (ref null $B))))
    (type $B (struct (field (ref null $A)))))
  (type $S (struct (field i32)))
  (func (export "test") (result i32)
    (struct.get $S 0 (struct.new $S (i32.const 42)))))
