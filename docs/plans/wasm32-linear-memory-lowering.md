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

### wasm32 プロファイルが課す言語設計制約

以下は GC 依存かつ lowering 不可能なため、言語仕様から除外する:

| 除外機能 | 理由 |
|---------|------|
| moving compaction / finalizer / weak reference | ADR-075 の初期 runtime 契約に含めない |

---
