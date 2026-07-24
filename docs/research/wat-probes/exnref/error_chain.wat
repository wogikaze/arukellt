;; exnref/try_table micro-benchmark mirroring bench_compute_error_chain.ark.
;; Functions throw on error; the loop catches via try_table and accumulates.
;; Compile:  wat2wasm ...  or  wasmtime run ... directly (wat is accepted).
;; Run:     wasmtime run --wasm exceptions --dir=. benchmarks/probe_exnref_error_chain.wat --invoke main

(module
  ;; Tags carry a sentinel payload (-1) so the catch label can produce
  ;; the same i32 result type as the normal path.
  (tag $div_by_zero (param i32))
  (tag $negative (param i32))

  (func $safe_div (param i32 i32) (result i32)
    local.get 1
    i32.eqz
    if
      i32.const -1
      throw $div_by_zero
    end
    local.get 0
    local.get 1
    i32.div_s
  )

  (func $validate (param i32) (result i32)
    local.get 0
    i32.const 0
    i32.lt_s
    if
      i32.const -1
      throw $negative
    end
    local.get 0
  )

  ;; compute_step(i) returns the same value as bench_compute_error_chain's Ok path.
  (func $compute_step (param i32) (result i32)
    (local $divisor i32)
    (local $q i32)
    (local $v i32)

    ;; divisor = i - (i/5)*5
    local.get 0
    local.get 0
    i32.const 5
    i32.div_s
    i32.const 5
    i32.mul
    i32.sub
    local.set $divisor

    ;; q = safe_div(i*3+7, divisor)
    local.get 0
    i32.const 3
    i32.mul
    i32.const 7
    i32.add
    local.get $divisor
    call $safe_div
    local.set $q

    ;; v = validate(q - 10)
    local.get $q
    i32.const 10
    i32.sub
    call $validate
    local.set $v

    ;; v - (v/1000)*1000
    local.get $v
    local.get $v
    i32.const 1000
    i32.div_s
    i32.const 1000
    i32.mul
    i32.sub
  )

  (func (export "main") (result i32)
    (local $i i32)
    (local $sum i32)
    (local $errors i32)
    (local $val i32)

    i32.const 1
    local.set $i

    block $done
      loop $loop
        local.get $i
        i32.const 50000
        i32.gt_s
        br_if $done

        block $catch (result i32)
          try_table (result i32)
            (catch $div_by_zero $catch)
            (catch $negative $catch)
            local.get $i
            call $compute_step
          end
          br $catch
        end
        local.set $val

        ;; accumulate
        local.get $val
        i32.const 0
        i32.lt_s
        if
          local.get $errors
          i32.const 1
          i32.add
          local.set $errors
        else
          local.get $sum
          local.get $val
          i32.add
          local.set $sum
          local.get $sum
          i32.const 1000000
          i32.gt_s
          if
            local.get $sum
            i32.const 1000000
            i32.sub
            local.set $sum
          end
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end

    ;; Return sum (errors are thrown away like the original prints them).
    local.get $sum
  )
)
