# wasm32 linear-memory lowering（tracing fallback）

ステータス: 実装計画（決定記録ではない。collector 方針は ADR-075 が正本）

関連 ADR: ADR-002、ADR-075
日付: 2026-03-25（計画抽出: 2026-07-11）

---

`wasm32`（AtCoder / 非 GC ランタイム向け）への lowering 実装方針。
言語仕様の正本は ADR-002（Wasm GC）。本ファイルは backend 実装計画のみ。

---

### wasm32 プロファイル: メモリ管理モデル

`wasm32` は ADR-075 の非移動 mark-sweep tracing runtime を使う。
arena、RC、cycle reject を target-specific な source semantics として導入しない。

#### runtime の段階導入

1. **recorded bump allocation**: 現行の linear allocation を allocation record として登録する。
2. **明示 root frame**: managed local の入口登録、local update、出口解放を compiler が出力する。
3. **mark-sweep**: root から payload の pointer candidate を追跡し、cycle を含む unreachable graph を free list へ戻す。
4. **managed-value lowering**: #874 で struct/enum/Option/Result/Vec/String/HashMap の descriptor と allocator reuse を接続する。

root update が全 managed local に接続されるまで、collector の自動起動は有効化しない。
この制限は live value の誤回収を防ぐための移行境界であり、#873/#874 の完了時に削除する。

2026-09-20 時点では `aed3af031` により、この計画の compiler 接続の第一段階を実装した。
linear 関数だけに frame cursor local を追加し、i32 表現の local と linear ref local を入口で
`root_push`、`LOCAL_SET` 後に `root_update`、return/fallthrough で `root_frame_leave` へ接続する。
これは保守的な bootstrap 表現に対する接続であり、descriptor-aware な managed write の全経路、
closure environment、HashMap、そして自動 `collect` の有効化を完了したことを意味しない。

同日 `196da76fa` では、length-prefix String ABI を使う fallback の `slice`、`trim`、
`trim_start`、`trim_end`、`to_upper`、`to_lower` の allocation を
`LINEAR_GC_ALLOC` へ接続した。allocator の戻り値を record base とし、既存の
payload pointer は `base + 4` として保持する。Wasm GC 側の array lowering と公開 API は
変更していない。format/split/join/replace、HashMap descriptor、closure environment、
descriptor-aware root write、automatic `collect` は引き続き未完了である。

続く `bf94dbe98` では、fallback の `join`、`repeat`、`pad_left`、`pad_right` の
allocation も `LINEAR_GC_ALLOC` へ接続した。`join` は count pass 後の総長、`repeat` は
積算長、`pad` は目標幅に header 分を加えたサイズを allocator へ渡す。no-fill `pad` の
既定 ASCII space も fallback で明示し、GC 側の array representation と同じ出力を保つ。
zero-length の `join`/`repeat` を含む両 target の fixture で検証済みである。

続く `d00da2f1c` では、fallback の `replace` が計算した上限サイズに header 分を
加え、`LINEAR_GC_ALLOC` から得た record base と `base + 4` の payload pointer を
使うようにした。empty pattern は source をそのまま返すことで、非進行の走査と
out-of-bounds を防ぐ。GC 側の lowering と公開 API は変更していない。
`replace` の両 target fixture で compile・validate・run を検証済みである。
format/split、HashMap descriptor、closure environment、descriptor-aware root write、
automatic `collect` は引き続き未完了である。

続く `26513a797` では、fallback の非空 separator `split` が使う Vec header と
backing storage（76 bytes）、および各分割 String record を `LINEAR_GC_ALLOC` へ
接続した。String record は `4 + segment_len` を 4-byte 境界へ丸め、戻り値を
record base として既存の `base + 4` payload pointer を維持する。empty separator は
既存の `chars` 経路へ委譲するため、この slice では allocator 接続の対象外とし、
両 target の回帰 fixture だけを確認した。format/char、HashMap descriptor、closure
environment、descriptor-aware root write、automatic `collect` は引き続き未完了である。

続く `b31fe4557` では、fallback `chars` の Vec allocation と scalar ごとの
length-prefix String allocation を `LINEAR_GC_ALLOC` へ接続した。empty separator の
`split` が委譲する経路も同じ record base / `base + 4` payload ABI を使う。
format、HashMap descriptor、closure environment、descriptor-aware root write、
automatic `collect` は引き続き未完了である。

続く `5393f3dc7` では、fallback の numeric/floating-point `to_string`、
`char_to_string`、`push_char`、`String_new` の length-prefix String allocation を
`LINEAR_GC_ALLOC` へ接続した。i32 の既存 4-byte 丸めと `base + 4` payload pointer
を維持し、Wasm GC の array lowering と公開 API は変更していない。debug／structured
format、HashMap descriptor、closure environment、descriptor-aware root write、
automatic `collect` は引き続き未完了である。

### wasm32 プロファイルが課す言語設計制約

以下は GC 依存かつ lowering 不可能なため、言語仕様から除外する:

| 除外機能 | 理由 |
|---------|------|
| moving compaction / finalizer / weak reference | ADR-075 の初期 runtime 契約に含めない |

続く `9e81a367e` では、linear fallback の Vec backing growth を
`LINEAR_GC_ALLOC` へ接続した。concrete `vec_push` と sealed `raw_array_grow` の
両経路が、既存の capacity doubling、element stride、Vec header、old backing の
copy を維持したまま、`grown_capacity * element_stride` を allocator へ渡す。
`std::collections::hash` の互換 HashMap はこの raw Vec growth を使うため、構築・再ハッシュ
も recorded allocation を通る。Wasm GC の typed array lowering と公開 API は変更していない。
HashMap descriptor、正確な pointer-field metadata、closure environment、automatic `collect`、
shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `0578b95a2` では、fallback の `Vec::get`／`Vec::pop` が返す
`Option<i32>` の 8-byte record（tag offset 0、payload offset 4）を
`LINEAR_GC_ALLOC` へ接続した。Some/None の layout と Vec の length semantics は維持し、
Wasm GC は typed Option variant の lowering を継続する。generic Option/Result constructor、
parse I/O buffer、HashMap descriptor、closure environment、automatic `collect`、
shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `f88ce4917` では、fallback の整数 parse Result allocation を
`LINEAR_GC_ALLOC` へ接続した。`Ok<i32>` の 8-byte record、`Ok<i64>` の 12-byte record、
invalid-integer の length-prefix String（32 bytes）と `Err` record（8 bytes）の既存 layout を
維持し、tag、payload、`base + 4` の String ABI を変更していない。f64 の Result／invalid-float
経路、parse I/O scratch buffer、generic constructor、HashMap descriptor、closure environment、
automatic `collect`、shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `c91051952` では、fallback の `Result<f64, String>` 成功 record と invalid-float の
String／`Err` record を `LINEAR_GC_ALLOC` へ接続した。成功 record は 12 bytes、エラー String は
30 bytes を要求して既存 allocator の 32-byte 境界へ丸め、`Err` record は 8 bytes とする。
tag、f64 payload、length-prefix、`base + 4` の String ABI は維持し、数値走査用の固定 scratch
address は一時状態として残している。Wasm GC の typed Result lowering、parse I/O scratch buffer、
generic constructor、HashMap descriptor、closure environment、automatic `collect`、
shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `8e720d28f` では、fallback の `core::range_new` が返す 8-byte Range record
（start offset 0、end offset 4）を `LINEAR_GC_ALLOC` へ接続した。Wasm GC は typed
Range struct の lowering を継続する。`RangeInclusive` は aggregate slice で接続済みの
generic struct constructor を使う。seq take/skip/unique の Vec allocation、generic
Option/Result constructor、HashMap descriptor、closure environment、automatic `collect`、
shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `15e0bfe75` では、fallback の `seq::take_i32`／`seq::skip_i32`／`seq::unique`
が作る Vec の header（12 bytes）と backing storage（result_len または src_len × 4 bytes）を
`LINEAR_GC_ALLOC` へ接続した。既存の `[len, cap, data_ptr]` layout、len=0 初期化、
memory.copy による payload 複製を維持し、raw bump 専用だった `emit_seq_slice_reserve_payload`
は allocator への一本化で削除した。Wasm GC の typed array lowering は変更していない。
generic Option/Result constructor、HashMap descriptor、closure environment、
automatic `collect`、shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `d752e6a4a` では、fallback の env/argv ABI 確保を `LINEAR_GC_ALLOC` へ接続した。
`env::var` が返す 8-byte `Option<String>` record（tag offset 0、payload offset 4）、
argv pointer table（argc × 4）と byte buffer、`env::args` の Vec header（12 bytes）と
data slot（P1 は 32 bytes、P2 は align4((argc-1) × 4)）、cstr→String コピー（clen + 4）を
managed allocator 経由に変更した。raw bump 専用の `emit_env_var_option_alloc`／
`emit_env_var_option_ptr_on_stack`／未使用の `emit_env_missing_var_option` は削除し、
bump pointer の alignment だけを担う `emit_env_var_align_heap` は raw heap_get/heap_set の
まま残した。Wasm GC の `*_gc` variant と P2 component path は変更していない。
host Result marshalling（result_helpers、fs/stdio buffer）、generic Option/Result
constructor、HashMap descriptor、closure environment、descriptor-aware root write、
automatic `collect`、shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `26ff0f517` では、fallback の fs host ABI 確保を `LINEAR_GC_ALLOC` へ接続した。
`emit_fs_read_result_pair`／`emit_fs_write_success_unit_result`／
`emit_fs_write_error_result` が作る 8-byte {tag, payload} Result record、
`emit_fs_read_open_error_message`／`emit_fs_write_error_message` の 19/20-byte
error String record、`emit_fs_write_bytes_buffer` の align4(bcount) staging buffer を
managed allocator 経由にした。error message builder は emit 時に target で分岐し、
wasm32 では managed allocator、wasm32-gc では GC string copy-out 用の raw scratch
staging を維持する。fs_read/stdin の streaming chunk loop は従来どおり chunk ごとの
bump commit で、単一 record 化は後続課題とする。GC 専用の P2 canonical result area は
raw scratch のままである。streaming loop の単一 record 化、`result_helpers.ark` の
dead emit 関数群、generic Option/Result constructor、HashMap descriptor、
closure environment、descriptor-aware root write、automatic `collect`、
shared/cyclic source graph、move/borrow parity は引き続き未完了である。

続く `0c4696759` では、streaming read buffer を単一 managed record として commit
する形に変えた。`helpers_core_heap::emit_heap_advance_uncommitted`（global.set +
ensure-grown、commit-top hook なし）を追加し、fs_read P1/P2 loop と stdin read loop の
length-prefix reserve・chunk ごとの claim を uncommitted advance に変更、finalize
（`emit_fs_read_success_len_header`／`emit_finish_stdin_string_buffer`／
`emit_finalize_stdin_string_buffer`）で一度だけ `emit_heap_set` を発行して
`[BUFSTART, heap)` 全体を 1 record として記録する。従来は chunk ごとに record が
分かれ、live String の尾部 chunk が collect で回収されうる状態だった。wasm32-gc では
commit index が未設定で heap_set は元々 raw bump のため、emit 形状の実効は変わらない。
`result_helpers.ark` の dead emit 関数群、generic Option/Result constructor、
HashMap descriptor、closure environment、descriptor-aware root write、
automatic `collect`、shared/cyclic source graph、move/borrow parity は引き続き
未完了である。

続く `f3c56bbef` では、managed local への全書き込みが root slot へ反映される
ようにした。従来は `MIR_LOCAL_SET` だけが `root_update` を発行し、
`inst_store::emit_store_action`（SET/TEE）経由の intrinsic 結果、linear
`struct_new`/`array_new` constructor、`array_set` の element staging、
非 GC `REF_TEST`/`REF_CAST` fallback、`future_new`/`await` の copy が
root slot を更新しないまま MIR local を書き換えていた（collect 有効化時の
false negative 要因）。`emit_frame_enter_and_push` が関数ごとの
slot-offset 表と frame cursor local を `WasmWriter` へ格納し、
`emit_update_after_local_set` は ctx なしで参照する形に変えたため、
generic store 経路のシグネチャ変更は不要だった。scratch local と SIMD lane
への書き込みは managed pointer を持たないため対象外（表は -1 を返す）。
operand stack / scratch local だけが保持する pointer の collect 安全性、
`LINEAR_GC_COLLECT` の呼び出し、HashMap descriptor、closure environment、
automatic `collect`、shared/cyclic source graph、move/borrow parity は
引き続き未完了である。

続く `2aaadcbf4` では、linear fallback の自動回収を有効化した。新しい mutable
global `GLOBAL_LINEAR_GC_ALLOC_WATERMARK`（index 7）が
`LINEAR_GC_ALLOC` ごとの aligned payload バイト数を累積し、
`LINEAR_GC_COLLECT` が先頭で 0 にリセットする。
`emit_frame_enter_and_push` は root push 群の直後に
`global.get watermark; i32.const 64MiB; i32.ge_u; if; call collect; end`
を発行し、前回の sweep から 64 MiB 割り付けた時点で関数 entry が
collector を起動する。entry 配置が健全性の核心で、operand stack が空かつ
全 live frame の managed local が root slot へ反映済みのため mark が
false negative を起こせない（caller の中断 stack に残るのは call 引数だけで、
それは callee の rooted param になる）。alloc 起点の collect は、allocation
site の operand stack 上に size より下へ managed pointer が残りうるため
採用しなかった。64 MiB threshold は linear 経路で 1 回の compile に数百 MiB
を割り付ける selfhost compiler が full-heap mark-sweep を償却できる値で、
小さなプログラムは collect せず memory.grow が backing する（回収は
footprint 最適化であり正しさの条件ではない）。e2e は
`test_function_entry_collects_when_watermark_exceeds_threshold` を追加し、
unrooted garbage 確保 → watermark 強制超過 → compiled `$main` 呼び出しで
watermark リセットと free list 非空を確認した。closure environment、
HashMap descriptor / 正確な pointer-field metadata（scan は ADR-075 が
許す conservative のまま）、shared/cyclic source graph、move/borrow parity
は引き続き未完了である。

続く `b2144f1c1` では、自動回収をソースレベルのプログラムから駆動する
初めての fixture `tests/fixtures/target_capability/linear_gc_entry_collect.ark`
を追加した。`churn` が呼び出しごとに約 1 MiB の `Vec<i32>` を捨て、
`main` が 160 回呼ぶ（約 160 MiB）ため、64 MiB watermark gate が
`churn` の entry で少なくとも 2 回発火する。その間 5 個の live 文字列は
`main` の root slot と `keep` backing store 経由で到達可能であり、
ソース内の `test::assert_*` が panic せず exit 0 となることで、mark が
Vec-of-String の live graph を推移的にたどり sweep が garbage だけを
回収したことを証明する。linear artifact の WAT では churn entry が
frame_enter + 7 root push + watermark gate、Vec header と backing が
`LINEAR_GC_ALLOC` 呼び出しであることを確認済み。fixture は `run:` と
`t3-run:` に登録し、e2e には
`test_source_level_live_data_survives_entry_collection` を追加して
wasm32/wasi-p1/memory32 での実実行を CI に残した。ソースで真の pointer
cycle は書けない（struct は managed ref ではなく i32 index を持つ。
実 cycle は runtime probe がカバー）、closure environment、HashMap
descriptor、move/borrow parity は引き続き未完了である。

---
