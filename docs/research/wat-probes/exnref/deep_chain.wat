;; Deep call-stack exnref benchmark: 10 nested fallible wrappers.
;; Mirrors probe_deep_chain.ark: may_fail throws, all other steps propagate via
;; the zero-cost try_table success path.

(module
  (tag $fail (param i32))

  (func $may_fail (param i32) (result i32)
    local.get 0
    i32.const 10
    i32.rem_s
    i32.eqz
    if
      i32.const 0
      throw $fail
    end
    local.get 0
    i32.const 1
    i32.add
  )

  (func $step1 (param i32) (result i32) local.get 0 call $may_fail i32.const 1 i32.add)
  (func $step2 (param i32) (result i32) local.get 0 call $step1 i32.const 1 i32.add)
  (func $step3 (param i32) (result i32) local.get 0 call $step2 i32.const 1 i32.add)
  (func $step4 (param i32) (result i32) local.get 0 call $step3 i32.const 1 i32.add)
  (func $step5 (param i32) (result i32) local.get 0 call $step4 i32.const 1 i32.add)
  (func $step6 (param i32) (result i32) local.get 0 call $step5 i32.const 1 i32.add)
  (func $step7 (param i32) (result i32) local.get 0 call $step6 i32.const 1 i32.add)
  (func $step8 (param i32) (result i32) local.get 0 call $step7 i32.const 1 i32.add)
  (func $step9 (param i32) (result i32) local.get 0 call $step8 i32.const 1 i32.add)
  (func $step10 (param i32) (result i32) local.get 0 call $step9 i32.const 1 i32.add)

  (func (export "main") (result i32)
    (local $i i32) (local $sum i32) (local $val i32)
    i32.const 1
    local.set $i
    block $done
      loop $loop
        local.get $i
        i32.const 10000
        i32.gt_s
        br_if $done

        block $catch (result i32)
          try_table (result i32) (catch $fail $catch)
            local.get $i
            call $step10
          end
          br $catch
        end
        local.set $val

        local.get $val
        i32.const 1
        i32.ge_s
        if
          local.get $sum
          local.get $val
          i32.add
          local.set $sum
        end

        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $sum
  )
)
