;; Wasm 3.0 — exception handling (try_table / throw) — NOT legacy try/catch
;; Success: (invoke "test") => i32.const 42
(module
  (tag $e (param i32))
  (func (export "test") (result i32)
    (block $handler (result i32)
      (try_table (result i32) (catch $e $handler)
        (throw $e (i32.const 42))
        (i32.const 0)))))
