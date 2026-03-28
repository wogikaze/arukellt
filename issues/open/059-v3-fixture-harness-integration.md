# v3 fixture 統合 + verify-harness.sh v3 ゲート

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 059
**Depends on**: 039, 040, 041, 042, 043, 044, 045, 046, 047, 048, 049, 050, 051, 052, 053, 054, 055, 056, 057
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

#039–#057 の各 issue が個別の fixture 要件を持つ。
この issue ではそれらを統合し、`verify-harness.sh` に **v3 ゲート** を追加して
v3 の全受け入れ条件を自動検証できるようにする。

- deprecated API がコードベースで使われていないことを確認
- manifest.txt に全 fixture が登録されていることを確認
- 各モジュールの代表 fixture がすべて pass することを確認

## 受け入れ条件

### fixture 一覧 (追加分)

各 issue の fixture 要件をまとめて追加する。

| fixture ファイル | カバー機能 | issue |
|---|---|---|
| `stdlib_core/error_basic.ark` | Error enum + message() | #041 |
| `stdlib_core/ordering.ark` | cmp::compare + Ordering | #041 |
| `stdlib_core/range_basic.ark` | Range + contains + step | #041 |
| `stdlib_core/hash_basic.ark` | hash関数 基本検証 | #041 |
| `stdlib_text/string_from_utf8.ark` | string::from_utf8 + validate | #042 |
| `stdlib_text/string_lines.ark` | string::lines + split | #042 |
| `stdlib_text/string_builder.ark` | StringBuilder push/finish | #042 |
| `stdlib_text/rope_basic.ark` | rope_from_string + len + slice | #042/#047 |
| `stdlib_text/rope_edit.ark` | rope_insert + rope_delete | #042/#047 |
| `stdlib_bytes/bytes_basic.ark` | Bytes::new + get + len | #043 |
| `stdlib_bytes/buf_basic.ark` | ByteBuf push/pop + len | #043 |
| `stdlib_bytes/cursor_basic.ark` | ByteCursor read + pos | #043 |
| `stdlib_bytes/view_basic.ark` | ByteView zero-copy slice | #043 |
| `stdlib_bytes/leb128_roundtrip.ark` | LEB128 encode/decode | #043 |
| `stdlib_bytes/hex_base64.ark` | hex + base64 encode/decode | #043 |
| `stdlib_bytes/endian_le.ark` | LE read_u32 + write_u32 | #043 |
| `stdlib_collections/hashmap_generic.ark` | HashMap<K,V> insert/get/remove | #044 |
| `stdlib_collections/hashset_basic.ark` | HashSet insert/contains/remove | #044 |
| `stdlib_collections/hashset_ops.ark` | union/intersection/difference | #044 |
| `stdlib_collections/deque_basic.ark` | push_front/push_back/pop_front | #045 |
| `stdlib_collections/priority_queue.ark` | push + pop_min + len | #045 |
| `stdlib_collections/btree_map.ark` | BTreeMap insert + range iter | #046 |
| `stdlib_collections/index_map.ark` | IndexMap insertion-order | #046 |
| `stdlib_collections/bit_set.ark` | BitSet set/test/clear + and/or | #046 |
| `stdlib_collections/arena_basic.ark` | Arena alloc/get + ArenaId | #047 |
| `stdlib_collections/slot_map_basic.ark` | SlotMap insert/get | #047 |
| `stdlib_collections/interner_basic.ark` | intern + resolve 双方向 | #047 |
| `stdlib_seq/seq_basic.ark` | range→map→filter→collect | #048 |
| `stdlib_seq/seq_zip.ark` | zip + enumerate | #048 |
| `stdlib_seq/seq_group_by.ark` | group_by + partition | #048 |
| `stdlib_seq/seq_word_count.ark` | 実用例 (単語頻度) | #048 |
| `stdlib_fs/path_join.ark` | path::join + parent + extension | #049 |
| `stdlib_fs/fs_exists.ark` | fs::exists + is_file + is_dir | #049 |
| `stdlib_io/reader_basic.ark` | Reader::from_bytes + read_line | #050 |
| `stdlib_io/buffered_writer.ark` | BufWriter flush + len | #050 |
| `stdlib_time/instant_basic.ark` | Instant::now + duration_since | #051 |
| `stdlib_random/random_basic.ark` | Rng::seeded + next_u32 + fill | #051 |
| `stdlib_process/args_basic.ark` | process::args() 読み取り | #052 |
| `stdlib_process/env_basic.ark` | env::var + env::vars | #052 |
| `stdlib_test/assert_ok.ark` | assert_ok + assert_err | #056 |
| `stdlib_test/snapshot_basic.ark` | snapshot 一致検証 | #056 |

### verify-harness.sh v3 ゲート追加

- [ ] Check 18: `stdlib_core/`, `stdlib_text/`, `stdlib_bytes/` の全 fixture が `manifest.txt` に登録されているか確認
- [ ] Check 19: deprecated API 使用禁止ゲート (`grep -r "Vec_new_i32\|filter_i32\|map_i32" tests/fixtures/ → 0件`)
- [ ] Check 20: v3 stdlib fixture 一括実行 (`cargo test -p arukellt --test harness -- stdlib_`)

### manifest.txt 更新

上記すべての fixture を `tests/fixtures/manifest.txt` に追加する。

## 実装タスク

1. `tests/fixtures/stdlib_{core,text,bytes,collections,seq,fs,io,time,random,process,test}/` ディレクトリを作成
2. 上記 fixture ファイルを作成 (各 issue の受け入れ条件を直接テストする)
3. `tests/fixtures/manifest.txt` に全エントリを追加 (`compile:tests/fixtures/stdlib_*/...`)
4. `scripts/verify-harness.sh` に Check 18–20 を追加

## 完了条件

- fixture 40 件以上が `cargo test -p arukellt --test harness` で pass
- `scripts/verify-harness.sh` が exit 0 (Check 20 まで全通過)
- deprecated API 使用が 0 件 (Check 19 pass)
- manifest.txt の整合性 Check 18 pass

## 注意点

1. ファイルシステム系 fixture (fs_exists, fs_read_bytes など) は WASI filesystem preopened dir が必要なため、`skip:` フラグを使うか wasmtime `--dir` オプションが必要
2. fixture の期待出力が `#[expected]` コメントで明示されていること
3. 各 issue が完了してから対応 fixture を追加する (コンパイルエラーを避ける)

## ドキュメント

- `docs/stdlib/testing-guide.md`: v3 stdlib fixture 追加方法のガイド
