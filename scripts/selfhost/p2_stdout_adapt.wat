;; Adapter: satisfies guest import wasi:cli/stdout@0.2.0::write via host streams.
(module
  (type $write (func (param i32 i32 i32 i32) (result i32)))
  (type $get_stdout (func (result i32)))
  (type $flush (func (param i32 i32 i32 i32)))

  (import "wasi:cli/stdout@0.2.0" "get-stdout" (func $get_stdout (type $get_stdout)))
  (import "wasi:io/streams@0.2.0" "[method]output-stream.blocking-write-and-flush" (func $flush (type $flush)))

  (func (export "write") (type $write) (param $ret i32) (param $ptr i32) (param $len i32) (param $unused i32) (result i32)
    (call $flush
      (call $get_stdout)
      (local.get $ptr)
      (local.get $len)
      (local.get $ret))
    (i32.const 0)
  )
)
