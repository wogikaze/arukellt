;; Wasm 3.0 — tail call (return_call)
;; Success: deep recursion completes without stack overflow; returns 0
;; Stress: invoke with large n (e.g. 1_000_000) in host harness.
(module
  (func $countdown (export "test") (param $n i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (i32.const 0))
      (else
        (return_call $countdown
          (i32.sub (local.get $n) (i32.const 1)))))))
