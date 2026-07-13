# WAT probe results

Date: 2026-07-13

## Toolchain versions

| Tool | Version |
|------|---------|
| wasm-tools | `wasm-tools 1.245.1` |
| wat2wasm | `1.0.34` |
| wasm-validate | `1.0.34` |
| wasmtime | `wasmtime 44.0.0 (af382d7d9 2026-04-20)` |
| iwasm | `iwasm 2.4.3` |
| node | `v25.2.1` |
| jco | `npx @bytecodealliance/jco@1.25.2 => 1.25.2` |
| chrome | `/home/wogikaze/.cache/puppeteer/chrome/linux-148.0.7778.97/chrome-linux64/chrome` |

## Matrix

| Probe | Expected | wasm-tools.parse | wasm-tools.validate | wabt.wat2wasm | wabt.validate | wasmtime | iwasm | node | browser | jco.transpile | custom-section.inject | wasm-tools.validate+custom | wasm-tools.annotation |
|-------|----------|---|---|---|---|---|---|---|---|---|---|---|---|
| `wasm10/01-arithmetic.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/02-locals.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/03-globals.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/04-drop-select.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/05-control-flow.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/06-call.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/07-call-indirect.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/08-memory.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/09-start.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/10-custom-section.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| `wasm10/11-trap-unreachable.wat` | `trap` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm10/12-convert-reinterpret.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/01-multi-value.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/02-reference-types.wat` | `1` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/03-typed-select.wat` | `1` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/04-multiple-tables.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/05-bulk-memory.wat` | `65` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/06-simd.wat` | `3` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/07-sign-extension.wat` | `-1` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/08-trunc-sat-scalar.wat` | `2147483647` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/09-trunc-sat-simd.wat` | `2147483647` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/10-js-bigint-i64.wat` | `js` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm20/11-table-ops.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/01-extended-const.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: illegal opcode or constant expressi` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/02-memory64.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: invalid limits flags` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/03-table64.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/03-t` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: invalid limits flags(table64 flag w` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/04-multiple-memories.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: multiple memories` | ✅ | ✅ | ❌ `transpile: ComponentError: unsupported section found in modu` | — | — | — |
| `wasm30/05-tail-call.wat` | `param` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: unsupported opcode 12` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/06-typed-func-ref.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/06-t` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: unsupported opcode 14` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/07-br-on-null.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/07-b` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: unsupported opcode d5` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/08-gc-struct.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/08-g` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: invalid type flag` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/09-gc-array.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/09-g` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: invalid type flag` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/10-i31.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/10-i` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: unsupported opcode fb` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/11-eh-try-table.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/11-e` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: invalid section id` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/12-relaxed-simd.wat` | `validate` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: invalid opcode 0xfd 101.` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/13-custom-annotations.wat` | `tooling` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | ✅ |
| `wasm30/14-return-call-ref.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/14-r` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: unsupported opcode 15` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/15-recursive-types.wat` | `42` | ✅ | ✅ | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/15-r` | ❌ `skipped (parse failed)` | ✅ | ❌ `WASM module load failed: invalid type flag` | ✅ | ✅ | ✅ | — | — | — |
| `wasm30/16-js-string-builtins.wat` | `js` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `skipped: host js-string imports (not a pure core export prob` | — | — | — |
| `experimental/legacy-eh-try-catch.wat` | `42` | ❌ `error: 'catch' found outside 'LegacyTry' block (at offset 0x` | ❌ `skipped (parse failed)` | ❌ `/home/wogikaze/arukellt/docs/research/wat-probes/experimenta` | ❌ `skipped (parse failed)` | ❌ `no binary` | ❌ `no binary` | ❌ `no binary` | ❌ `no binary` | ❌ `no binary` | — | — | — |
| `experimental/threads-atomics.wat` | `42` | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ `WASM module load failed: invalid limits flags` | ✅ | ✅ | ✅ | — | — | — |

## Failure details

### `wasm30/01-extended-const.wat`

- **iwasm**: `WASM module load failed: illegal opcode or constant expression required or type mismatch`

### `wasm30/02-memory64.wat`

- **iwasm**: `WASM module load failed: invalid limits flags`

### `wasm30/03-table64.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/03-table64.wat:6:17: error: unexpected token funcref, expected (. /   (table $T i64 funcref (elem $f)...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: invalid limits flags(table64 flag was found, please enable memory64)`

### `wasm30/04-multiple-memories.wat`

- **iwasm**: `WASM module load failed: multiple memories`
- **jco.transpile**: `transpile: ComponentError: unsupported section found in module using multiple memories /     at generate (file:///home/wogikaze/.npm/_npx/c2122ac3d87d3ef3/no...`

### `wasm30/05-tail-call.wat`

- **iwasm**: `WASM module load failed: unsupported opcode 12`

### `wasm30/06-typed-func-ref.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/06-typed-func-ref.wat:8:6: error: opcode not allowed: call_ref /     (call_ref $t (ref.func $f)))) / ...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: unsupported opcode 14`

### `wasm30/07-br-on-null.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/07-br-on-null.wat:10:7: error: unexpected token (, expected ). /       (br_on_null $is_null) /       ...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: unsupported opcode d5`

### `wasm30/08-gc-struct.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/08-gc-struct.wat:4:13: error: struct not allowed /   (type $S (struct (field i32))) /             ^^^...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: invalid type flag`

### `wasm30/09-gc-array.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/09-gc-array.wat:4:13: error: array type not allowed /   (type $A (array (mut i32))) /             ^^^...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: invalid type flag`

### `wasm30/10-i31.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/10-i31.wat:5:6: error: unexpected token "i31.get_s", expected an instr. /     (i31.get_s (ref.i31 (i3...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: unsupported opcode fb`

### `wasm30/11-eh-try-table.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/11-eh-try-table.wat:4:3: error: tag not allowed /   (tag $e (param i32)) /   ^ / /home/wogikaze/aruke...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: invalid section id`

### `wasm30/12-relaxed-simd.wat`

- **iwasm**: `WASM module load failed: invalid opcode 0xfd 101.`

### `wasm30/14-return-call-ref.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/14-return-call-ref.wat:8:6: error: unexpected token "return_call_ref", expected an instr. /     (retu...`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: unsupported opcode 15`

### `wasm30/15-recursive-types.wat`

- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/wasm30/15-recursive-types.wat:4:4: error: unexpected token "rec", expected a module field. /   (rec /    ^^^`
- **wabt.validate**: `skipped (parse failed)`
- **iwasm**: `WASM module load failed: invalid type flag`

### `wasm30/16-js-string-builtins.wat`

- **jco.transpile**: `skipped: host js-string imports (not a pure core export probe)`

### `experimental/legacy-eh-try-catch.wat`

- **wasm-tools.parse**: `error: 'catch' found outside 'LegacyTry' block (at offset 0x30)`
- **wasm-tools.validate**: `skipped (parse failed)`
- **wabt.wat2wasm**: `/home/wogikaze/arukellt/docs/research/wat-probes/experimental/legacy-eh-try-catch.wat:6:3: error: tag not allowed /   (tag $e (param i32)) /   ^ / /home/wogi...`
- **wabt.validate**: `skipped (parse failed)`
- **wasmtime**: `no binary`
- **iwasm**: `no binary`
- **node**: `no binary`
- **browser**: `no binary`
- **jco.transpile**: `no binary`

### `experimental/threads-atomics.wat`

- **iwasm**: `WASM module load failed: invalid limits flags`
