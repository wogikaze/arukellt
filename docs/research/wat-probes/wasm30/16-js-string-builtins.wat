;; Wasm 3.0 JS String Builtins — JS embedding probe (not core-only)
;; Import a js-string builtin; host must compile with {builtins:['js-string']}
;; Minimal: cast string to externref length via wasm:js-string
(module
  (type $t (func (param externref) (result i32)))
  (import "wasm:js-string" "length" (func $len (type $t)))
  (func (export "test") (param externref) (result i32)
    (call $len (local.get 0))))
