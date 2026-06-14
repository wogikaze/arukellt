;; WASI P2 stdout adapt: guest write -> get-stdout + blocking-write-and-flush.
;; Memory-less: wasm-tools component new wires guest memory for canon lower.
(module
  (type $write (func (param i32 i32 i32 i32) (result i32)))
  (type $get_stdout (func (result i32)))
  (type $flush (func (param i32 i32 i32 i32)))
  (import "wasi:cli/stdout-host@0.2.0" "get-stdout" (func $get_stdout (type $get_stdout)))
  (import "wasi:io/streams@0.2.0" "[method]output-stream.blocking-write-and-flush" (func $flush (type $flush)))
  (export "write" (func $write))
  (export "cabi_post_write" (func $cabi_post_write))
  (export "_initialize" (func $_initialize))
  (func $write (type $write) (param $ret i32) (param $ptr i32) (param $len i32) (param $_ i32) (result i32)
    (call $flush
      (call $get_stdout)
      (local.get $ptr)
      (local.get $len)
      (local.get $ret))
    (i32.const 0)
  )
  (func $cabi_post_write (param i32))
  (func $_initialize)
)
