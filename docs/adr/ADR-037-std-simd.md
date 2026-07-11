# ADR-037: std::simd — Explicit SIMD Library API

ステータス: **PROPOSED** — 明示的 SIMD ライブラリ API と v128 第一級型の導入を提案

決定日: 2026-06-26

---

## 文脈

Arukellt には SIMD サポートが存在しない。`std::wasm::valtype_v128` の定数バイトのみが
experimental で存在する。本 ADR は明示的 SIMD ライブラリ API としての導入入口を定義する。

Issue #107 (ループベクトル化ヒント) は reject 済みであり、本 ADR はその代替位置づけである。
ただし「autovectorization を採用する」のではなく、「SIMD 導入の最初の入口を compiler hint
ではなく explicit library API にする」という代替である。

### 関連 ADR

- ADR-002: Wasm GC 前提 — v128 は値型だが GC struct/array フィールドに保持可能
- ADR-045: 旧 LLVM 方針撤回 — native SIMD は本 ADR のスコープ外（後継 native ADR で決定）
- ADR-006: 公開 ABI 3層 — SIMD 値の ABI 表現を追加
- ADR-007: ターゲット `wasm32` / `wasm32-gc` / `native` — ターゲット別 SIMD 可否
- ADR-014: stability labels — 初期は experimental

### Wasm 3.0 仕様上の前提

v128 は Wasm 3.0 の `valtype` の一種 (`valtype ::= numtype | vectype | reftype ...`) である。
GC の `struct` / `array` のフィールド型は `storagetype` で、`storagetype ::= valtype | packtype`
である。したがって v128 は struct フィールドにも array 要素にも入る。`struct.new` は
`unpack(storagetype)` を引数に取り、v128 は packed type ではないので `unpack(v128) = v128` のまま
扱われる。`struct.get` / `struct.set` / `array.get` / `array.set` も同様に v128 をそのまま
読み書きする。

参照: [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html),
[WebAssembly 3.0 Spec — Instructions](https://webassembly.github.io/spec/core/valid/instructions.html)

---

## 提案する決定

### 1. v128 第一級型

v128 を言語型システムの第一級型として追加する。typechecker / MIR / emitter の全面対応を行う。
facade (Vec<i32> 等での scalar エミュレーション) は採用しない。

### 2. v128 のメモリモデル

v128 は Wasm 値型であり GC 管理外だが、GC struct のフィールドおよび GC array の要素として
保持できる (Wasm 3.0 仕様準拠)。ADR-002 (Wasm GC 前提) と矛盾しない。

- `struct` フィールド型として v128 を許可
- `array` 要素型として v128 を許可
- `struct.new` / `struct.get` / `struct.set` は v128 をそのまま扱う
- `array.get` / `array.set` / `array.new` も同様

### 3. 型と構文

#### lane 型モジュール

`std::simd` 配下に lane 型ごとのモジュールを設ける:

```
std::simd::i8x16
std::simd::u8x16
std::simd::i16x8
std::simd::u16x8
std::simd::i32x4
std::simd::u32x4
std::simd::i64x2
std::simd::u64x2
std::simd::f32x4
std::simd::f64x2
std::simd::v128        // 低レベル raw 型
```

#### 構文例

lane 型モジュールの関数呼び出し:

```
let a = f32x4::new(1.0, 2.0, 3.0, 4.0)
let b = f32x4::splat(2.0)
let c = f32x4::add(a, b)
let x = f32x4::extract_lane(c, 3)
let d = f32x4::replace_lane(c, 3, x)
```

配列リテラル + 演算子:

```
let a: f32x4 = [1.0, 2.0, 3.0, 4.0]
let b = f32x4::splat(2.0)
let c = a + b
let d = f32x4::replace_lane(c, 3, f32x4::extract_lane(c, 0))
```

#### 低レベル v128 型

`v128` は「表現力のため」ではなく「Wasm に直接落とす低レベル境界のため」に存在する。
`v128.load` / `v128.store` / `v128.bitselect` / `v128.and` / `reinterpret` / 外部 intrinsic /
未分類の raw SIMD 値を扱う際に必要。

### 4. ターゲット適用範囲

| ターゲット | SIMD 可否 | 挙動 |
|------------|-----------|------|
| `wasm32` | 全 SIMD 命令使用可能 | v128 命令を直接 emit |
| `wasm32-gc` | 全 SIMD 命令使用可能 | v128 命令を直接 emit |
| `native-cpp` / `native-llvm` | **本 ADR のスコープ外** | 後継 native ADR（ADR-045 再評価後）で決定 |

`wasm32` / `wasm32-gc` は Wasm SIMD を直接 emit する。
スカラー展開は SIMD 無効ビルド向けの同値計算パスとして保持する。

### 5. 機能検出

コンパイルターゲットによる出し分けとする。実行時検出は行わない。
ターゲットが決まれば SIMD 可否は確定する。

### 6. SIMD 無効時の挙動

`wasm32` と `wasm32-gc` はともにネイティブ SIMD 命令を使用する（§4 参照）。
SIMD が無効なビルド（例: `-simd128` 未指定、または SIMD 非対応の将来 embedder）では
scalar 展開で同値計算する。エラーではなくエミュレーションである。
これは「fallback」ではなく「scalar による同値計算」であり、ADR-015 (ユーザパス panic 禁止) に
従い panic は発生しない。

### 7. 名前空間

`std::simd` 単層とする。`std::wasm::simd` には寄せない。
ポータブル SIMD API と Wasm-specific intrinsic は責務が異なるため分離する (後述 #11, #16)。

### 8. API 形状

lane 型モジュール分割とする。flat 関数 (`simd_i32x4_add`) ではなく、
`std::simd::i32x4::add` のように lane 型ごとにモジュールを分ける。

### 9. 対応 lane 型

初回から全 lane 型をカバーする:

```
i8x16   u8x16
i16x8   u16x8
i32x4   u32x4
i64x2   u64x2
f32x4
f64x2
v128             // 低レベル raw 型
```

### 10. 操作カテゴリ

| カテゴリ | 対象 | 例 |
|----------|------|-----|
| construct | 全型 | `splat`, literal, `zero` |
| lane access | 全型 | `extract_lane`, `replace_lane` |
| shuffle / swizzle | 主に byte lane | `shuffle`, `swizzle` |
| arithmetic | 整数・浮動小数 | `add`, `sub`, `mul`, float の `div`, `sqrt` |
| sign / abs | 整数・浮動小数 | `neg`, `abs` |
| comparison | 全型 | `eq`, `ne`, `lt`, `le`, `gt`, `ge` |
| mask / select | 全型 | `select`, `bitselect`, `any`, `all`, `bitmask` |
| bitwise | raw / integer 寄り | `and`, `or`, `xor`, `not`, `andnot` |
| shift | 整数型 | `shl`, `shr_s`, `shr_u` |
| saturating / narrow | 小さい整数型中心 | `add_sat`, `sub_sat`, `narrow` |
| widening / pairwise | 整数型 | `extend`, `extmul`, `extadd_pairwise` |
| conversion | int/float 間 | `to_i32x4_sat`, `to_f32x4`, `promote`, `demote` |
| memory | raw / backend 層 | `load`, `store`, `load_splat`, `load_lane` |

#### v128 / raw 層の配置

v128 / raw 層の操作は `std::simd` の通常 API 表には出さず、`std::wasm` 側に寄せる。
ここに置くのは: `v128.load` / `v128.store` / `v128.and` / `v128.or` / `v128.xor` /
`v128.not` / `v128.andnot` / `v128.bitselect` / `v128.any_true` / `reinterpret` 系。

Wasm の bitwise は v128 を 128 個の独立したビットとして扱い、memory 操作も
`v128.load` / `v128.store` として定義されている。これらは Wasm-specific intrinsic であり
portable SIMD API の範囲には入らない。

### 11. メモリアクセス — load/store API の分離

**決定:**

1. `std::simd` は明示的な load/store API を持たない。
2. SIMD 値の読み書きは通常の Arukellt の field/index access に統合する。
3. Wasm の `v128.load` / `v128.store` は `std::wasm` に限定し、`LinearPtr` / `LinearSlice`
   だけを受け付ける。
4. GC Vec から raw pointer を取り出す API は提供しない。
5. GC Vec と linear memory の変換は明示的 marshal API として別途提供する。

**根拠:**

- `v128.load` / `v128.store` は線形メモリ命令であり、GC object access ではない。
- WasmGC では v128 を struct/array field として保持できるため、GC Vec 上の SIMD 値は
  `array.get` / `array.set` で扱える。
- raw load/store を `std::simd` に混ぜると、portable SIMD と Wasm-specific intrinsic の境界が崩れる。

### 12. バックエンド実装 (Wasm emitter)

必要な範囲で SIMD opcode / v128 型 / locals / params の対応を追加する:
- `emit_opcodes.ark` への SIMD opcode 追加
- `sections_types.ark` の v128 型追加
- locals / params の v128 扱い
- MIR (`mir_opcodes.ark`) への SIMD 命令追加

### 13. native backends

native SIMD は本 ADR のスコープ外とする。Wasm SIMD との意味論関係を含め、
ADR-045 の再評価条件を満たした後継 ADR で決める。

### 14. stability label と昇格条件

初期 stability は `experimental` (ADR-014)。

`std::simd` が `stable` になる条件:

1. portable API の型・演算・mask・layout が破壊的変更なしで固定されていること
2. `+simd128` と `-simd128` の両方で同じ意味になる scalar fallback が実装済みであること
3. GC Vec / struct field / array element に置いた SIMD 値の lowering が ADR-002 と矛盾しないこと
4. raw `std::wasm` API との境界が確定していること
5. conformance test と lowering test があること

これらが満たされない限り、`std::simd` は後で ABI と型システムに刺さるため stable にしない。

### 15. Issue #107 との関係

本 ADR は Issue #107 (ループベクトル化ヒント) の代替である。

- #107 の hint-based autovectorization は本 ADR では採用しない。
- 将来再評価する場合も、hint を直接 Wasm SIMD 命令へ落とすのではなく、明示的な
  `std::simd` 呼び出しと同じ `Simd<T, N>` MIR へ正規化できるかを条件とする。
- `#[vectorize]` のような hint を入れると、意味論・診断・最適化責任が一気に compiler 側へ
  寄る。現段階ではまず `std::simd::f32x4` を明示的に呼ばせ、compiler はそれを素直に
  v128 または scalar tuple に lower する。
- autovectorization は将来再評価するが、その場合も内部表現としては `Simd<T, N>` IR に
  lower する方針である。`std::simd` はソース上の明示 API、IR は compiler 内部の
  canonical SIMD 表現、という分離を維持する。

### 16. std::wasm::valtype_v128 との統合

`std::wasm::valtype_v128` を `std::simd` に移動しない。

- `std::simd::f32x4` は言語レベルの portable vector
- `std::wasm::valtype_v128` は Wasm module/type reflection / encoder / decoder 側の定数

責務が異なるため、両者は独立して存在する。

---

## 禁止事項

1. `std::simd` に `v128.load` / `v128.store` を混ぜない (portable と Wasm-specific の境界崩壊)
2. GC Vec から raw pointer を取り出す API を提供しない (GC 安全性の侵害)
3. `#[vectorize]` のような compiler hint を導入しない (#107 の代替として明示 API を優先)
4. native 固有の SIMD 方針は本 ADR で固定しない（ADR-045 / 後継）
5. `wasm32` でスカラー展開をデフォルトとしない（iwasm 2.4.1 が SIMD 対応のためネイティブ SIMD を使用。スカラー展開はフォールバックとして保持）

---

## 結果

### 正の影響

- ユーザが明示的に SIMD を制御でき、予測可能な性能が得られる
- Wasm SIMD 命令を直接 emit するため、wasmtime / V8 / SpiderMonkey 間で一貫した挙動
- GC struct/array フィールドに v128 を保持できるため、GC Vec 上の SIMD 値が自然に扱える
- SIMD 無効ビルドでも scalar 展開により同値計算が可能で、ポータビリティが保たれる
- `std::simd` (portable) と `std::wasm` (Wasm-specific) の責務分離が明確

### 負の影響

- v128 第一級型の導入により typechecker / MIR / emitter の全面改修が必要
- `wasm32` の scalar 展開は性能が出ない (SIMD の恩恵を受けられない)
- load/store API の分離により、線形メモリ上の SIMD データ操作は明示的 marshal が必要

---

## 関連

- ADR-002: Wasm GC 前提 (v128 の GC フィールド保持の根拠)
- ADR-045: 旧 LLVM 方針撤回 — native SIMD はスコープ外
- ADR-006: 公開 ABI 3層 (SIMD 値の ABI 表現)
- ADR-007: ターゲット `wasm32` / `wasm32-gc` / `native` (ターゲット別 SIMD 可否)
- ADR-014: stability labels (experimental → stable 昇格条件)
- `issues/reject/107-runtime-loop-vectorization-hint.md` — 本 ADR の代替元
- `std/wasm/mod.ark` — `valtype_v128` 定数 (本 ADR では移動しない)
- [WebAssembly 3.0 Spec — Types](https://webassembly.github.io/spec/core/syntax/types.html)
- [WebAssembly 3.0 Spec — Instructions](https://webassembly.github.io/spec/core/valid/instructions.html)
