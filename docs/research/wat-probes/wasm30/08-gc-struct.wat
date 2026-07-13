;; Wasm 3.0 — GC struct
;; Success: (invoke "test") => i32.const 42
(module
  (type $S (struct (field i32)))
  (func (export "test") (result i32)
    (struct.get $S 0 (struct.new $S (i32.const 42)))))
