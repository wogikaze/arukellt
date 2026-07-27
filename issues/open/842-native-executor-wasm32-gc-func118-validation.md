# native-executor wasm32-gc S3 validation drift (func 118)

## Summary
When the native executor lane historically forced `--target wasm32-gc`, Stage-3
validation failed with a nominal reference-type mismatch in `func 118`.

## Reproduction (historical receipt)
- Command: `python3 scripts/manager.py selfhost native-executor --build` with S3 target hardcoded to `wasm32-gc`
- Receipt validation_error:
  `invalid compiler wasm arukellt-s3-native.wasm: error: func 118 failed to validate`
  `type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x452d7)`
- Saved artifacts (origin workspace snapshot):
  - `.recovery-snapshot/raw/.build/selfhost/native/native-executor-receipt.json`
  - `.recovery-snapshot/raw/.build/native-cpp-executor/selfhost/native-cpp/arukellt-s3-native-debug.wasm`

## Impact on native-cpp MVP
Not a blocker for the canonical native executor lane, which inherits the
comparison S2 build profile (`wasm32` + `wasi-p1` today). Handle under a
separate `s2-gc[wasm32-gc] -> s3-gc[wasm32-gc]` validation lane.

## Acceptance
- Reproducible under an explicit wasm32-gc profile lane
- Root-caused type canonicalization / function 118 body drift fixed or waived with ADR-backed scope
