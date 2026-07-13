;; Wasm 3.0 — custom annotations (@name / @custom) — tooling, not runtime opcode
;; Success: WAT parser/formatter preserves annotations (not "runs correctly")
(module
  (@custom "my-section" "payload")
  (func $foo (@name "renamed") (export "test") (result i32)
    (i32.const 42)))
