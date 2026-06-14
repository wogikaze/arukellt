;; P2 stdout bridge: forwards canonical list write args to host streams.
(module
  (type $write (func (param i32 i32 i32 i32) (result i32)))
  (type $get_stdout (func (result i32)))
  (type $flush (func (param i32 i32 i32 i32)))

  (import "env" "get-stdout" (func $get_stdout (type $get_stdout)))
  (import "env" "blocking-write-and-flush" (func $flush (type $flush)))

  (func (export "write") (type $write) (param $ret i32) (param $ptr i32) (param $len i32) (param $unused i32) (result i32)
    (call $flush
      (call $get_stdout)
      (local.get $ptr)
      (local.get $len)
      (local.get $ret))
    (i32.const 0)
  )
)
