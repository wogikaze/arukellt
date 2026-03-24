;; Case 1: Hello World (Wasm GC version)
;; Uses GC types for string representation

(module
  ;; Import WASI fd_write
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  ;; GC type definitions
  ;; String as GC array of i8
  (type $string (array (mut i8)))

  ;; Memory for iovec (still needed for fd_write)
  (memory (export "memory") 1)

  ;; Create "Hello, World!\n" string using GC array
  (global $hello (ref $string)
    (array.new_fixed $string 14
      (i32.const 72)   ;; H
      (i32.const 101)  ;; e
      (i32.const 108)  ;; l
      (i32.const 108)  ;; l
      (i32.const 111)  ;; o
      (i32.const 44)   ;; ,
      (i32.const 32)   ;; (space)
      (i32.const 87)   ;; W
      (i32.const 111)  ;; o
      (i32.const 114)  ;; r
      (i32.const 108)  ;; l
      (i32.const 100)  ;; d
      (i32.const 33)   ;; !
      (i32.const 10))) ;; \n

  ;; Copy GC string to linear memory for fd_write
  (func $copy_to_linear (param $str (ref $string)) (param $offset i32) (result i32)
    (local $i i32)
    (local $len i32)
    (local.set $len (array.len (local.get $str)))
    (block $break
      (loop $loop
        (br_if $break (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $offset) (local.get $i))
          (array.get $string (local.get $str) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $len))

  ;; Main function
  (func $main (export "_start")
    (local $len i32)
    ;; Copy string to memory at offset 100
    (local.set $len (call $copy_to_linear (global.get $hello) (i32.const 100)))
    
    ;; Set up iovec at offset 0
    ;; iovec.buf = 100
    (i32.store (i32.const 0) (i32.const 100))
    ;; iovec.len = string length
    (i32.store (i32.const 4) (local.get $len))
    
    ;; Call fd_write(stdout=1, iovs=0, iovs_len=1, nwritten=50)
    (drop (call $fd_write
      (i32.const 1)   ;; stdout
      (i32.const 0)   ;; iovs pointer
      (i32.const 1)   ;; iovs count
      (i32.const 50))));; nwritten pointer
)
