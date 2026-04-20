(module
  (import "wasi:cli/stdout@0.2.0" "get_stdout" (func $get_stdout (result i32)))
  (import "wasi:cli/stdout@0.2.0" "blocking_write_and_flush" (func $write (param i32 i32 i32) (result i32)))
  (import "wasi:cli/stdout@0.2.0" "drop" (func $drop (param i32)))
  (import "wasi:cli/exit@0.2.0" "exit" (func $exit (param i32)))

  (memory (export "memory") 1)
  (data (i32.const 0) "Hello from P2!\n")

  (func (export "run")
    (local $stdout i32)
    (local $result i32)

    (call $get_stdout)
    local.set $stdout

    (local.get $stdout)
    (i32.const 0)
    (i32.const 13)
    (call $write)
    drop

    (local.get $stdout)
    (call $drop)

    (i32.const 0)
    (call $exit)
  )
)
