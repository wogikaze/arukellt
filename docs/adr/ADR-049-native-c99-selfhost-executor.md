# ADR-049: Native C99 Selfhost Executor（セルフホスト native executor）

ステータス: **ACCEPTED** — `native-cpp` を C99 生成による experimental セルフホスト executor とする

決定日: 2026-07-22
廃止: [ADR-045](ADR-045-llvm-scope-withdrawn.md)

---

## 文脈

正規のセルフホスト fixpoint は、Stage-2 の Wasm コンパイラを Wasmtime で実行して
Stage-3 を生成する。この経路は検証の信頼契約として必要だが、日常のコンパイラ編集で
反復するには実行時間が長い。既存の `native-cpp` は target 名と scaffold だけを持ち、
MIR、ABI、runtime、実行経路は未決定だった。

[ADR-045](ADR-045-llvm-scope-withdrawn.md) は native / LLVM の判断を保留し、具体的な
必要性が再提起された時点で後継 ADR を作ることを要求していた。本 ADR はその後継として、
セルフホストコンパイラを高速に実行する限定用途だけを採択する。公開 native 製品や
LLVM IR backend の判断を同時に再開しない。

## 決定

### 1. 役割と対象環境

`native-cpp` は Arukellt コンパイラ自身を最初の利用者とする **experimental な
セルフホスト compiler executor** である。将来同じ backend を一般化できるが、MVP は
一般ユーザー向け native 製品の互換性、完全性、配布を保証しない。

対象環境は Linux x86-64、LP64、little endian、libc ありとする。基準 C compiler は
clang とする。現行実装は引き続き scaffold であり、本 ADR の採択は実装完了を意味しない。
現行状態は [current-state](../current-state.md) を正とする。

### 2. 生成方式と責務

- selfhost MIR から portable C99 source を生成する。
- C++ の例外、RTTI、template、destructor その他の C++ 機能を使用しない。
- LLVM IR を直接生成しない。native machine code 生成と最適化は clang に委ねる。
- backend は MIR から単一の C source を生成するところまでを所有する。
- compile、link、runtime 結合、cache、receipt は `scripts/manager.py` が所有する。
- MIR と C の間に新しい汎用 BackendIR を設けず、必要な機械的 lowering を直接行う。

### 3. private ABI

generated C と同梱 native runtime の境界は compiler-private ABI とする。compiler
version 間の互換性は保証せず、runtime 全体に整数 ABI version を持たせる。公開 C ABI、
`extern C` 相当の公開 FFI、他言語からの link 契約は提供しない。

関数 symbol は `FunctionId` と完全な `SignatureRegistry` signature から決定的に生成する。
backend は callee 名の文字列から意味、型、呼び先を推測せず、`FunctionId`、`TypeId`、
`CoreOpId`、`SignatureRegistry` を正本とする。詳細 layout と runtime ABI は
[RFC-008](../rfcs/008-native-cpp-c99-backend-runtime-abi.md) が所有する。

### 4. メモリ管理

MVP は non-moving process-lifetime arena を使用する。arena は chunked allocation とし、
最低 16-byte alignment を保証する。個別 free、phase reset、reference counting、weak
reference、finalizer は使用しない。セルフホストは一プロセス一ジョブであり、確保した
memory はプロセス終了時に OS へ返す。

これは [ADR-002](ADR-002-memory-model.md) の単一 GC 言語意味論を変更しない。循環参照を
拒否せず、object address は安定する。weak reference と finalizer を観測可能な機能として
導入しない限り、到達不能 object を実行中に回収しないことを native 独自の ownership
model として公開しない。

### 5. host access

native executor は内部 tool として ambient host access を持つ。WASI sandbox と同じ
security model は提供しない。MVP runtime はセルフホストに必要な filesystem、environment、
standard I/O、process、clock だけを提供する。一般 native product の capability model は
本 ADR と分離して将来判断する。

### 6. selfhost verification

[ADR-029](ADR-029-selfhost-native-verification-contract.md) の正規 fixpoint を維持する。

```text
pinned wasm -> s2.wasm
s2.wasm     -> s3.wasm
require: sha256(s2.wasm) == sha256(s3.wasm)
```

native executor は別の高速 lane とする。

```text
s2.wasm
  -> compiler sourceをnative-cpp C99へ生成
  -> clangでarukellt-nativeを生成
  -> arukellt-nativeがcompiler sourceからs3.wasmを生成
  -> require: sha256(s2.wasm) == sha256(s3.wasm)
```

日常開発は native executor lane を利用できる。CI、release、bootstrap 更新では正規 fixpoint
を維持する。native lane は当面正規 fixpoint の代替ではなく、正規契約を変更する場合は
ADR-029 を別途置換または改訂する。

### 7. 非目標

MVP は次を対象外とする。

- Windows、macOS、non-Linux host、cross compile
- 一般ユーザー向け native 製品、public C ABI、external FFI
- LLVM IR backend、C++ backend
- exact GC、reference counting
- SIMD、async / future、WIT / Component Model、network runtime

未対応の MIR opcode と CoreOp は capability validation で target diagnostic とし、stub、
偽値、黙った `unreachable` で成功扱いしない。

## 却下した代替案

### LLVM IR を直接生成する

LLVM 型、GC、ABI、IR verifier まで backend が所有し、S3 高速化に必要な範囲を超える。
C99 を clang へ渡せば optimizer と machine code generator を利用できるため却下する。

### C++ を生成する

例外、destructor、RTTI、template instantiation と Ark の panic、GC、monomorphization の
境界を増やす。MIR CFG の機械的変換には C99 で足りるため却下する。

### reference counting または conservative GC

reference counting は ADR-002 が許す循環参照を回収できず、第二の ownership model を
導入する。conservative GC は外部依存、偽 root、stack scan、再現性を新たな信頼境界へ
加える。いずれも限定された一プロセス executor の初期方式として採用しない。

### 初期段階から exact GC を実装する

root tracking、marking、sweep、GC safe point が Phase 1 の C lowering を阻害する。
process-lifetime arena の実測で成立しないと判明してから導入する。

### public C ABI または正規 fixpoint の即時置換

公開 ABI は内部 layout を安定化してしまい [ADR-006](ADR-006-abi-policy.md) に反する。
native lane は Wasm executor 自身による Stage-3 生成を証明しないため、ADR-029 を変更せず
正規 fixpoint を置換しない。

## 帰結

- ADR-045 の保留を終了し、本 ADR が限定された native-cpp 判断を所有する。
- `native-llvm`、公開 FFI、一般 native product は引き続き未決定である。
- native-cpp の詳細仕様は RFC-008、実装順は native-cpp MVP plan、対応状態は
  machine-readable capability registry が所有する。
- target の support tier と implementation state は実装が完了するまで scaffold のままとする。

## 再検討条件

exact non-moving mark-sweep は次のいずれかで再検討する。

- `executor_peak_rss_bytes` が現行 Wasm executor の基準約 2.4 GiB を超える。
- 大規模入力で process-lifetime arena が実用上成立しない。
- 一般ユーザー向け native product へ昇格する。
- weak reference または finalizer が言語仕様へ入る。
- 長寿命 native process を正式対応する。

性能基準の測定区間と receipt schema は RFC-008、実装時の gate は implementation plan を
正本とし、本 ADR に生きた測定値を追記しない。

## 関連

- [ADR-002: GC vs non-GC](ADR-002-memory-model.md)
- [ADR-006: 公開 ABI 境界の分類](ADR-006-abi-policy.md)
- [ADR-007: コンパイルターゲット整理](ADR-007-targets.md)
- [ADR-029: セルフホストネイティブ検証契約](ADR-029-selfhost-native-verification-contract.md)
- [ADR-042: Intrinsic Layer Separation](ADR-042-intrinsic-layer-separation.md)
- [ADR-045: 旧 LLVM 役割方針を撤回](ADR-045-llvm-scope-withdrawn.md)
- [ADR-048: 設計原則の適用順序](ADR-048-design-heuristics-application-order.md)
- [RFC-008: native-cpp C99 backend と runtime ABI](../rfcs/008-native-cpp-c99-backend-runtime-abi.md)
- [native-cpp MVP implementation plan](../plans/native-cpp-mvp-implementation.md)
- [native-cpp capability registry](../../data/native-cpp-capabilities.toml)
