;; P2 stdout bridge: reads list<u8> from imported guest memory, forwards to host streams.
(module
  (type $write (func (param i32 i32 i32 i32) (result i32)))
  (type $get_stdout (func (result i32)))
  (type $flush (func (param i32 i32 i32 i32)))

  (import "host" "memory" (memory 1))
  (import "env" "get-stdout" (func $get_stdout (type $get_stdout)))
  (import "env" "blocking-write-and-flush" (func $flush (type $flush)))

  (func (export "write") (type $write) (param $ret i32) (param $a i32) (param $b i32) (param $c i32) (result i32)
    (local $ptr i32)
    (local $len i32)
    (local.set $ptr (i32.load (i32.const 0)))
    (local.set $len (i32.load (i32.const 4)))
    (call $flush
      (call $get_stdout)
      (local.get $ptr)
      (local.get $len)
      (local.get $ret))
    i32.const 0
  )
)
