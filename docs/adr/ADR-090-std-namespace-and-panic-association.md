# ADR-090: `Std` 名前空間型と `Std::panic` への制御 API 移動

ステータス: **ACCEPTED** — 無レシーバの制御操作を prelude の `Std` 名前空間型へ移し、free `panic` を退役させる

決定日: 2026-10-29

関連: [ADR-000](ADR-000-process.md)、[ADR-014](ADR-014-stability-labels.md)、[ADR-044](ADR-044-trait-method-syntax-adopted.md)、[ADR-046](ADR-046-free-function-eradication.md)、[ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md)、[ADR-087](ADR-087-0x-deprecation-exception.md)、[#917](../../issues/open/917-stdlib-eradication-execution.md)、[#697](../../issues/open/697-vec-operation-extension.md)

## 文脈

[ADR-046](ADR-046-free-function-eradication.md) D1 は、レシーバを持たない公開操作も
free function のまま残さず、型またはモジュール名前空間の associated function へ寄せる
ことを要求している。例外は非公開 intrinsic のみである（D2）。

`panic` は長らく `std/prelude.ark` の free function として公開され、
`std/manifest.toml` では `kind = "prelude_wrapper"`、`intrinsic = "__intrinsic_panic"`、
`core_op_id = "panic"` を持つ。呼び出しは CoreOp `panic` へ lower され、本体は
typechecking 用の shell である。`docs/stdlib/eradication.md` の scorecard は
`panic` を `associated-or-method`（manifest 1 件）として残し、置換案を
「core namespace 型の associated（例: `Std::panic`）」または「明示的な ADR 例外」と
していた。同 scorecard の `collections sort free algorithms`（source 4 件:
`sort` / `sort_unstable` / `is_sorted` / `select_nth`）は、`Vec<T: Ord>` の
no-argument method が未整備だったため残っていた。

[ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md) §1 は v2 stable Prelude の
型を `Option` / `Result` / `String` / `Vec`、関数を `panic` / `assert` / `String_from` /
`String_new` / `char_to_string` に固定している。したがって free `panic` の置換には、
prelude の安定集合そのものの改訂が必要である。

`panic` は stable 公開エントリであるため、[ADR-014](ADR-014-stability-labels.md) と
[ADR-087](ADR-087-0x-deprecation-exception.md) D2 は通常、W0009 による deprecation
window、migration guide、changelog の 3 条件を削除前に要求する。

## 決定

### 1. `Std` を prelude の安定名前空間型にする

`std/prelude.ark` に `pub struct Std` を置き、stable Prelude の型集合へ加える
（ADR-072 §1 の改訂）。`Std` はインスタンスを作らない名前空間型であり、private field
だけを持つ。空 record は wasm32-gc component の type section で不正になるため、
フィールドは空にしない。

`Std` の最初の associated function を `Std::panic(message: String)` とする。
`panic` の core-op binding と `__intrinsic_panic` は non-public bridge として維持する
（ADR-046 D2）。公開面に残るのは `Std::panic` だけである。

### 2. free `panic` を削除する

`std/prelude.ark` の `pub fn panic` と `std/manifest.toml` の `[[functions]]` entry を
削除する。旧 bare `panic(...)` は E0100 と `resolver::removed_names` の
「`Std::panic(message)` を使う」hint で失敗させる。呼び出し側は `Std::panic` へ移行する。
`Std` は prelude の安定型として auto-import されるため、利用側の追加 import は不要である。

### 3. ADR-014/087 lifecycle の明示例外

free `panic` の削除は、W0009 の deprecation window を置かずに同じ wave で行う。
[ADR-087](ADR-087-0x-deprecation-exception.md) D2 の 0.x 例外のうち、次を明示的に満たす
ものとして扱う。

- **Migration diagnostic**: 旧名の利用は W0009（warning）ではなく E0100（compile error +
  replacement hint）で止まる。削除後の移行診断として W0009 より強い。
- **Migration guide**: `docs/stdlib/migration-guidance.md` に before/after と
  replacement を記録する。
- **Changelog**: `docs/stdlib/changelog.md` に日付、ADR-087 / 本 ADR、対象シンボル、
  削除理由、replacement を記録する。

この例外は「stable 全体を一括削除してよい」という許可ではない。対象は `panic` 1 件に
限定し、制御プリミティブという性質（レシーバなし、CoreOp 直結、同じ leaf 名を保つ）を
根拠とする。

### 4. sort の no-argument 面は `Vec<T: Ord>` に置く

`sort` / `sort_unstable` / `is_sorted` / `select_nth` は
`impl<T: Ord> Vec<T>` の method とする。comparator を受ける
`sort_by` 系は `impl<T> Vec<T>` に残し、`f64` のように `Ord` でない要素型でも
total order を渡せる境界を維持する。`std::collections::sort` モジュールは削除する。
これは ADR-046 / ADR-086 の method 正規形へ実装を合わせるものであり、新しい API 判断では
ない。

## 帰結

- `Std` が prelude の安定型に加わり、`Std::panic` が唯一の panic 綴りになる。
- `std::collections::sort` の 4 free algorithm と `panic` の manifest entry が消え、
  scorecard の `associated-or-method` から 5 件が外れる。
- 旧 `panic(...)` と `sort(v)` 系の call site は E0100 の hint で機械的に移行できる。
- `panic` の CoreOp 契約と `__intrinsic_panic` bridge は変わらない。
- ADR-072 の stable 集合は本 ADR の範囲で改訂される。ADR-072 本体の keyword policy と
  target 非依存の原則は維持する。

## 却下した代替案

### free `panic` を ADR 例外として温存する

関連付けが実現可能であることを両 target で確認できるため、採らない。温存は ADR-046 D1 の
「無レシーバ系も associated へ寄せる」に反し、scorecard の根絶を止める。

### `Std` を追加せず `String` などの既存型の associated にする

`panic` はレシーバ型に属する操作ではなく、`Result::panic` / `String::panic` は型の責務を
混ぜる。専用の名前空間型の方が契約が一意になる。

### W0009 の deprecation window を先に置き、削除を次 wave に送る

0.x の削除実行をさらに 1 wave 遅らせるだけで、移行診断は E0100 の方が強い。本 ADR の
限定例外で同じ wave に置く。

### `panic` を非公開 intrinsic だけにする

`__intrinsic_panic` は non-public bridge として既に存在する。公開操作を消すだけで、
ユーザーは `Std::panic` を使える。intrinsic を公開面に戻す必要はない。

## 再検討条件

- `Std` に panic 以外の無関係な操作を追加したくなったとき（名前空間型の責務を再検討する）。
- 1.0.0 を切り、ADR-087 の 0.x 例外が失効したとき（本 ADR の削除根拠を再確認する）。
- E0100 の移行診断が不十分で、W0009 の deprecation window が必要と判断されたとき。

## 参照

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- [ADR-046](ADR-046-free-function-eradication.md)
- [ADR-072](ADR-072-language-v2-prelude-and-keyword-policy.md)
- [ADR-086](ADR-086-linear-collection-adts.md)
- [ADR-087](ADR-087-0x-deprecation-exception.md)
- [`std/prelude.ark`](../../std/prelude.ark)
- [`std/collections/vec.ark`](../../std/collections/vec.ark)
- [`std/manifest.toml`](../../std/manifest.toml)
- [migration-guidance.md](../stdlib/migration-guidance.md)
- [changelog.md](../stdlib/changelog.md)
