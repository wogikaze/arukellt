---
Status: done
Created: 2026-04-15
Updated: 2026-04-22
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib: 実装品質監査 (hash / parsing / collection algorithm の甘さ) を実施する
**Closed**: 2026-04-22
**Commit**: 873f2da0
**ID**: 514
**Depends on**: none
**Track**: stdlib
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: stdlib modernization backlog requested 2026-04-15

## Summary

stdlib には「とりあえず動く」実装が残っており、hash quality や parser robustness、
data structure invariants の品質面で Rust 標準実装や一般的期待値との差が大きい箇所がある。
この issue は correctness / collision / robustness / invariants の観点から実装品質を監査し、
優先順位つきの follow-up を切り出す。

## Repo evidence

- `std/core/hash.ark` は simple multiplicative hash を使っている
- `std/collections/hash.ark` は simple hash + linear probing 前提で、quality note が弱い
- `std/fs/mod.ark` には best-effort / stub notice が残る

## Hash family (quality audit slice)

`std/core/hash.ark` と `std/collections/hash.ark` を対象に、4 軸でリスクを整理する（行番号は当該コミット時点のワークツリー）。

### Correctness

- **欠損キーと値 `0` の曖昧さ**: `hashmap_get` は未発見時も `result` を `0` のまま返す（初期化と「空スロットで打ち切り」の両方で `0`）。値 `0` を格納する用途では欠損と区別できない。→ `std/collections/hash.ark:45-63`（特に `45-46`, `49-50`, `63`）。`hashmap_get_option` が別途あることの是非も契約上の分岐点。→ `std/collections/hash.ark:141-162`
- **挿入のサイレント失敗**: `hashmap_set` はプローブが `cap` に達しても `done` が `false` のまま終了し、呼び出し側へのエラー・戻り値がない。満杯・高負荷クラスタ時にキーが黙って入らない。→ `std/collections/hash.ark:90-117`（`95-105` とループ出口 `118` 周辺）
- **固定容量・リハッシュなし**: 初期容量は `hashmap_new` で `16` 固定。ファイル内にテーブル拡張・リハッシュ手続きがなく、`hashmap_with_capacity` で与えた `cap` が上限。負荷が載ると上記失敗経路に入りやすい。→ `std/collections/hash.ark:18-31`, `126-137`
- **`hash_i32` の二重定義**: コアはバイト混合の乗算ハッシュ（`std/core/hash.ark:4-21`）、コレクション側は `key * 1000003` と絶対値（`std/collections/hash.ark:34-37`）。同名概念で分布が一致しないため、「どちらを標準とみなすか」がブレるとバケット外の期待と実装がずれる。

### Collision（分布・衝突の質）

- **整数**: コアの `hash_i32` は下位 4 バイトのみを `31` 乗算で混ぜる簡易形（高ビットの取り込み方が限定的）。→ `std/core/hash.ark:4-20`
- **文字列**: `hash_string` は FNV-1a 風だが、中間値が負のとき `abs` で折り返し、エントロピーが落ちる経路がある。→ `std/core/hash.ark:24-36`（`30-34`）
- **コレクション用バケット索引**: `hash_i32(key)` は単一の素数乗算と絶対値のみ。キー空間によってはモジュロ `cap` 前の衝突・クラスタが起きやすい。→ `std/collections/hash.ark:34-37` と `h % cap` の利用箇所（例: `42-43`, `68-69`, `92-93`, `143-144`）

### Performance

- **線形プロービング**: 実装・コメントどおりオープンアドレス + 線形探索。負荷因子が高いと平均・最悪プローブが伸びる。→ `std/collections/hash.ark:3-4`, `47-60`, `71-85`, `96-116`, `147-160`
- **削除の全再挿入**: `hashmap_remove` は生存エントリを集めて clear 後に再 `hashmap_set`（注釈どおり O(n)）。大きいマップでは予算超過になりうる。→ `std/collections/hash.ark:211-233`
- **文字列セットの名実ギャップ**: `hashset_str_*` は `Vec<String>` の線形走査。名前は Hash だが挿入・検索のスケールはベクタ相当。→ `std/collections/hash.ark:336-377`（ループ本体 `348-354`, `366-375`）

### Contract（仕様・API の曖昧さ）

- **ジェネリック Hash ではないことの表明**: `STOP_IF` とモノモーフィック制約がモジュール先頭で明示されている。呼び出し側が「汎用 HashMap/HashSet」と誤解すると期待齟齬。→ `std/collections/hash.ark:6-11`, `124-125`（拡張 API セクション見出し）
- **レイアウト契約**: フラット `Vec<i32>` の `[cap, size, keys, values, flags]` と `flags` の意味はコメントで固定されている。バイナリ互換や unsafe 連携をするときの唯一の仕様源。→ `std/collections/hash.ark:13-16`, `18-20`
- **結合ハッシュの弱さ**: `combine` / `hash_combine` は `h * 31 + h2` の線形結合のみ。多コンポーネントや順序近い値での衝突耐性は限定的。→ `std/core/hash.ark:39-47`
- **コアモジュールの位置づけ**: `std/core/hash.ark:1-1` は「現在のコレクション実装が使う小さなヘルパ」とある一方、コレクション側は自前の `hash_i32` を持つ。どちらが正かの一本化が契約上未解決。

## Acceptance

- [x] hash family, parser family, collection family, host facade family の品質監査リストが作成される
- [x] correctness risk / perf risk / collision risk / contract ambiguity の 4 軸で優先順位が付く
- [x] 少なくとも `std::core::hash`, `std::collections::hash`, `std::json`, `std::toml`, `std::fs` の監査結果が文書化される
- [x] 高優先度の follow-up issue が必要数だけ派生するか、本 issue 内に subtask として整理される

## Primary paths

- `std/core/hash.ark`
- `std/collections/hash.ark`
- `std/json/mod.ark`
- `std/toml/mod.ark`
- `std/fs/mod.ark`

## References

- `issues/done/044-std-collections-hash.md`
- `issues/done/392-stdlib-error-result-conventions.md`
- [`docs/stdlib/modernization/514-parser-host-quality-audit.md`](../../docs/stdlib/modernization/514-parser-host-quality-audit.md)