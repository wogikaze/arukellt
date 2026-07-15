# RFC-005: Prelude compilation restoration

ステータス: ACCEPTED  
関連 ADR: [ADR-042](../adr/ADR-042-intrinsic-layer-separation.md)  
関連 issue: [#816](../../issues/open/816-prelude-compilation-restoration.md)  
関連 plan: [intrinsic-layer-separation.md](../plans/intrinsic-layer-separation.md)  
日付: 2026-07-16

本 RFC は ADR-042「要別 ADR / RFC」のうち **prelude のコンパイル対象復帰**
の詳細仕様と移行手順の正本である。採択判断の枠は ADR-042。実装 owner は #816。

---

## 要約

`combine_loaded_and_main_decls_skip_prelude` を廃止し、`std/prelude.ark` を
backend の decl 結合に含める。公開面（`std/manifest.toml` auto-import）は維持する。

意味の正本は `std::*` と CoreOpRegistry に置き、prelude は薄い forwarding /
compatibility binding に限定する。新規の正規入口は method / trait /
associated function（ADR-044 / ADR-046 / #718）とし、`__intrinsic_*` を
新しい正規入口に固定しない。

---

## 動機 / 問題

現行パイプラインは prelude を型検査用にロードしつつ、backend 結合では
`name == "prelude"` を除外している。そのため:

1. prelude 本体が「コンパイルされる Ark」ではなく型チェック用スタブになっている。
2. ADR-042 の 5 層分離（特に semantic stdlib の Ark fallback）が始められない。
3. #820（stdlib-only inliner）と #821/#822（Ark body 移行）が prelude 復帰に依存する。

一括で全 prelude 記号の意味論を Ark へ移すと、emitter CoreOp と二重化し、
現状の selfhost メモリ上限にも抵触する。段階復帰が必要である。

---

## 提案

### D1: skip 関数を廃止する

`combine_loaded_and_main_decls_skip_prelude` を削除し、backend は
`combine_loaded_and_main_decls` のみを使う。prelude モジュールの decls は
他の loaded module と同様に結合される。

### D2: 正規形と compatibility

| 層 | 役割 |
|----|------|
| method / trait / associated | 新規・移行先の正規入口（#718 と整合） |
| prelude free function | 削除条件付き compatibility binding |
| `__intrinsic_*` | 移行期の emitter / CoreOp 別名。新規正規入口にしない |

prelude から `std::*` への薄い委譲は許可する。委譲先を free function のまま
固定しない。

### D3: CoreOp 束縛関数の body 扱いは段階的

CoreOpRegistry / binding で `core_op_id` が付く prelude 記号について:

1. **#816 時点**: call site は既存どおり CoreOp / `legacy_emitter` で lower する。
   MIR body の lowering は省略してよい（selfhost メモリと未配線 `__intrinsic_*`
   呼び出しを避ける暫定ゲート）。decls と SignatureEntry は結合対象に含める。
2. **#821 / #822**: 各操作に解決可能な Ark `implementation_symbol` を与え、
   body を本番 fallback として lower する。#820 inliner がその前提になる。

CoreOp を持たない純粋 Ark（`assert`、`clamp_*`、手書きループの `contains_*` 等）
は #816 で必ず body を lower する。

### D4: スタブ除去の定義

「fake stub」とは次を指す:

- 到達不能な `panic(...); return ...` だけの本体
- 意味を持たない自己再帰
- 型を通すためだけの空本体

`__intrinsic_*` 直呼びは #816 では **移行期の明示的 thin shim** として残してよいが、
D3 により CoreOp 束縛分の shim は body lower しない。shim の削除と Ark 本体への
置換は #821/#822 の完了条件である。

### D5: manifest 互換

`std/manifest.toml` の prelude 公開記号・stability・auto-import は壊さない。
`core_op_id` 欠落で intrinsic 名が binding に載らないエントリは、既存 CoreOp へ
`core_op_id` を埋めてよい（`legacy_bindings` の無秩序な拡張はしない）。

---

## 移行手順（#816）

1. 本 RFC を ACCEPTED にする。
2. skip 関数と呼び出しを削除する。
3. CoreOp 束縛 top-level の MIR body 省略ゲートを入れる（D3）。
4. 非 CoreOp の純粋 Ark body が lower されることを fixture で確認する。
5. `docs/current-state.md` を更新し `verify quick` を通す。

---

## 代替案と却下理由

| 案 | 却下理由 |
|----|----------|
| skip を残したまま std だけ移行 | ADR-042 の prelude 復帰要件を満たさない |
| 全 prelude body を即座に lower | selfhost OOM、未配線 intrinsic で失敗 |
| `__intrinsic_*` を正規 API として固定 | ADR-044/046 と #718 に反する |
| `legacy_bindings` を大量追加してつなぐ | #798 スコープの再拡張。manifest `core_op_id` を優先 |

---

## 未決事項（本 RFC 外）

- 個別操作の Ark 本体と differential（#821 / #822）
- stdlib-only inliner の予算（#820）
- `status = "production"`（#818）

---

## 関連

- ADR-042 D 層分離、要別 RFC 節
- [#816](../../issues/open/816-prelude-compilation-restoration.md)
- [#718](../../issues/open/718-stdlib-free-function-method-migration.md)
- [#820](../../issues/open/820-stdlib-only-inliner.md), [#821](../../issues/open/821-pure-semantic-stdlib-migration.md)
