;; Legacy EH (pre-Wasm-3.0) — try/catch — NOT the same as try_table
;; Older text form used (try (do ...) (catch ...)).
;; Modern wasm-tools may reject this; kept as historical probe.
;; Success under legacy EH tooling: (invoke "test") => i32.const 42
(module
  (tag $e (param i32))
  (func (export "test") (result i32)
    (try (result i32)
      (throw $e (i32.const 42))
      (catch $e))))
