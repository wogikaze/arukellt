;; Case 1: Hello World (linear memory version)
;; Uses linear memory for string representation

(module
  ;; Import WASI fd_write
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  ;; Memory layout:
  ;; 0-7: iovec structure
  ;; 8-11: nwritten
  ;; 100+: string data
  (memory (export "memory") 1)

  ;; Data section: "Hello, World!\n"
  (data (i32.const 100) "Hello, World!\n")

  ;; Main function
  (func $main (export "_start")
    ;; Set up iovec at offset 0
    ;; iovec.buf = 100
    (i32.store (i32.const 0) (i32.const 100))
    ;; iovec.len = 14
    (i32.store (i32.const 4) (i32.const 14))
    
    ;; Call fd_write(stdout=1, iovs=0, iovs_len=1, nwritten=8)
    (drop (call $fd_write
      (i32.const 1)   ;; stdout
      (i32.const 0)   ;; iovs pointer
      (i32.const 1)   ;; iovs count
      (i32.const 8))));; nwritten pointer
)
