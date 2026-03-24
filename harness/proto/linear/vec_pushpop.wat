;; Case 3: Vec push/pop 10k (linear memory version)
;; Uses linear memory with bump allocator

(module
  ;; Import WASI fd_write for output
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  ;; Memory layout:
  ;; 0-63: scratch space for fd_write
  ;; 64-127: Vec header (ptr, len, cap)
  ;; 1024+: heap area
  (memory (export "memory") 2)

  ;; Heap pointer (bump allocator)
  (global $heap_ptr (mut i32) (i32.const 1024))

  ;; Allocate n bytes
  (func $alloc (param $size i32) (result i32)
    (local $ptr i32)
    (local.set $ptr (global.get $heap_ptr))
    (global.set $heap_ptr (i32.add (global.get $heap_ptr) (local.get $size)))
    (local.get $ptr))

  ;; Vec structure at fixed address 64:
  ;; 64: data pointer (i32)
  ;; 68: length (i32)
  ;; 72: capacity (i32)

  ;; Create a new Vec
  (func $vec_new
    ;; Allocate initial array (16 * 4 = 64 bytes)
    (i32.store (i32.const 64) (call $alloc (i32.const 64)))
    (i32.store (i32.const 68) (i32.const 0))   ;; len = 0
    (i32.store (i32.const 72) (i32.const 16))) ;; cap = 16

  ;; Push value to Vec
  (func $vec_push (param $val i32)
    (local $len i32)
    (local $cap i32)
    (local $data i32)
    (local $new_data i32)
    (local $i i32)
    
    (local.set $len (i32.load (i32.const 68)))
    (local.set $cap (i32.load (i32.const 72)))
    (local.set $data (i32.load (i32.const 64)))
    
    ;; Check if need to grow
    (if (i32.ge_u (local.get $len) (local.get $cap))
      (then
        ;; Double capacity
        (local.set $cap (i32.mul (local.get $cap) (i32.const 2)))
        (local.set $new_data (call $alloc (i32.mul (local.get $cap) (i32.const 4))))
        
        ;; Copy old data
        (block $break
          (loop $loop
            (br_if $break (i32.ge_u (local.get $i) (local.get $len)))
            (i32.store 
              (i32.add (local.get $new_data) (i32.mul (local.get $i) (i32.const 4)))
              (i32.load (i32.add (local.get $data) (i32.mul (local.get $i) (i32.const 4)))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $loop)))
        
        (i32.store (i32.const 64) (local.get $new_data))
        (i32.store (i32.const 72) (local.get $cap))
        (local.set $data (local.get $new_data))))
    
    ;; Store value
    (i32.store 
      (i32.add (local.get $data) (i32.mul (local.get $len) (i32.const 4)))
      (local.get $val))
    
    ;; Increment length
    (i32.store (i32.const 68) (i32.add (local.get $len) (i32.const 1))))

  ;; Pop value from Vec
  (func $vec_pop (result i32)
    (local $len i32)
    (local $val i32)
    
    (local.set $len (i32.sub (i32.load (i32.const 68)) (i32.const 1)))
    (local.set $val 
      (i32.load 
        (i32.add 
          (i32.load (i32.const 64)) 
          (i32.mul (local.get $len) (i32.const 4)))))
    
    (i32.store (i32.const 68) (local.get $len))
    (local.get $val))

  ;; Get Vec length
  (func $vec_len (result i32)
    (i32.load (i32.const 68)))

  ;; Print i32 as decimal
  (func $print_i32 (param $n i32)
    (local $offset i32)
    (local $temp i32)
    
    ;; Write digits backwards starting at offset 60
    (local.set $offset (i32.const 60))
    (local.set $temp (local.get $n))
    
    (block $done
      (loop $loop
        (local.set $offset (i32.sub (local.get $offset) (i32.const 1)))
        (i32.store8 (local.get $offset)
          (i32.add (i32.const 48) (i32.rem_u (local.get $temp) (i32.const 10))))
        (local.set $temp (i32.div_u (local.get $temp) (i32.const 10)))
        (br_if $loop (i32.gt_u (local.get $temp) (i32.const 0)))))
    
    ;; Add newline at 60
    (i32.store8 (i32.const 60) (i32.const 10))
    
    ;; Set up iovec at 0
    (i32.store (i32.const 0) (local.get $offset))
    (i32.store (i32.const 4) (i32.sub (i32.const 61) (local.get $offset)))
    
    ;; Write
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8))))

  ;; Main function
  (func $main (export "_start")
    (local $i i32)
    (local $sum i32)
    
    (call $vec_new)
    
    ;; Push 0..10000
    (local.set $i (i32.const 0))
    (block $break1
      (loop $loop1
        (br_if $break1 (i32.ge_u (local.get $i) (i32.const 10000)))
        (call $vec_push (local.get $i))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop1)))
    
    ;; Pop all and sum
    (local.set $sum (i32.const 0))
    (block $break2
      (loop $loop2
        (br_if $break2 (i32.eqz (call $vec_len)))
        (local.set $sum (i32.add (local.get $sum) (call $vec_pop)))
        (br $loop2)))
    
    ;; Print sum (should be 49995000)
    (call $print_i32 (local.get $sum)))
)
