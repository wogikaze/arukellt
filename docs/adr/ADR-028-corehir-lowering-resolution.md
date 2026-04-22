# ADR-028: CoreHIR lowering circular blocker — re-route via #529 selfhost retirement

ステータス: **DECIDED** (2026-04-22) — Rust 側 `lower_hir_to_mir` は実装せず、Rust legacy
lowering の撤去は issue #529 (100% selfhost transition) と同期して行う。
これにより issue \#285 ⇄ \#508 の循環ブロッカーを解消する。

**Created**: 2026-04-22
**Scope**: corehir lowering, legacy path retirement, issue dependency graph

## Context

`issues/open/285-legacy-path-deprecation.md` と
`issues/open/508-legacy-path-removal-unblocked-by.md` は循環的にブロックし合っており、
どちらか一方だけでは前進できない状態にある。

- `issues/open/285-legacy-path-deprecation.md:8` — `Blocks: 508`
- `issues/open/285-legacy-path-deprecation.md:31-43` — 残るアクセプタンス
  項目 (フォールバック除去・legacy なしでの fixture) はすべて
  `**blocked by #508**` と明記。
- `issues/open/508-legacy-path-removal-unblocked-by.md:7-11` —
  `Depends on: 285`, `Orchestration upstream: #285`。
- `issues/open/508-legacy-path-removal-unblocked-by.md:53-57` — 解除条件は
  「`lower_hir_to_mir` が実 MIR を産出するようになること」。

実装側の根因は `crates/ark-mir/src/lower/facade.rs:44-116` の
`lower_hir_to_mir` がスタブで、空の `MirModule` を返してしまうため
`crates/ark-mir/src/lower/facade.rs:251-268` の
`lower_corehir_with_fallback` が常に legacy アーム
(`crates/ark-mir/src/lower/func.rs:87` の `lower_to_mir`) に落ちる点にある。
したがって両 issue のクローズには、原則として Rust 側 CoreHIR lowerer の
本実装が必要に見える。

しかし両 issue は次の重要なメタ情報を持つ:

> Implementation target: Use Ark (src/compiler/*.ark) instead of Rust crates
> (crates/*) per #529 100% selfhost transition plan.
> — `issues/open/285-legacy-path-deprecation.md:15`,
> `issues/open/508-legacy-path-removal-unblocked-by.md:14`

そして実際に canonical な HIR→MIR lowering は selfhost 側に既に存在する:

- `src/compiler/mir.ark:1127` — `// ── HIR→MIR Lowering ─` セクション
- `src/compiler/mir.ark:2187` — `fn lower_expr(ctx: LowerCtx, node: AstNode) -> i32`

つまり「Rust 側 `lower_hir_to_mir` を本実装する」という素朴な解決策は
ADR-027 (v3 完了時点で selfhost 完了, v4 スキップ) と #529 の方針と矛盾する。
撤去予定のコードに対する重い実装投資になってしまう。

## Decision

**Rust 側 `lower_hir_to_mir` は実装しない。Rust legacy lowering
(`lower_to_mir` を含む) の撤去は #529 100% selfhost transition の
クレート退役ステップに統合する。** これにより:

1. #285 の "deprecation marker" 部分は既に完了しているのでクローズする。
   未完のアクセプタンス項目 (fallback 除去 / fixture が legacy なしで pass) は
   #529 配下の Rust クレート退役サブイシューに移管する。
2. #508 の `Depends on` を `285` から `529` に張り替える。これで循環は解消する。
3. `crates/ark-mir/src/lower/facade.rs` の `lower_corehir_with_fallback`
   は現状維持 — 空 MIR → legacy フォールバックの動作を凍結し、ADR-028 への
   pointer コメントを残す (実装スライスは別 issue)。

### Why option (c) "delete with re-scoping", not (a) "implement Rust lowerer"

評価した3案:

- **(a) Rust 側 `lower_hir_to_mir` を本実装する**
  → 拒否。退役予定コード (`crates/ark-mir/`) に lowering 全機能を再実装する
    のは ADR-027 / #529 と矛盾。selfhost 側 (`src/compiler/mir.ark`) に
    既に canonical 実装が存在するため二重投資になる。
- **(b) HIR→MIR を別経路で迂回する (例: legacy のままにし CoreHIR を skip)**
  → 拒否。CoreHIR は型解析・解決の正本であり、迂回するなら #285 が掲げた
    「二重メンテ終了」目的を達成できない。実態は (a) と同じく Rust 側で
    再実装が必要。
- **(c) Rust legacy 撤去を #529 のクレート退役と同期させ、両 issue を
    re-scope する** ← 採用。
  → 退役予定コードへの新規投資ゼロ。selfhost 側の lowering を canonical と
    位置づける ADR-024 / ADR-027 と整合。循環ブロッカーを設計判断のみで
    解消できる。

## Sequencing

1. **ADR-028 が ACCEPTED** (本コミット)。
2. **#285 をクローズする**: 残アクセプタンス項目を「#529 配下に移管」と
   注記して done に移す。"deprecation marker" 部分は実体として完了済み。
3. **#508 の `Depends on:` を `285` から `529` に書き換え**、
   `Orchestration upstream: #529` に更新。これで循環は数学的に消える
   (#508 は #529 にしか依存しない)。
4. **#529 配下に新サブイシューを起票** (このスライスでは作成しない、
   下記 "Follow-up sub-issues" を参照)。
5. **Rust クレート退役 (#529 配下のサブイシュー) が完了した時点で**、
   `lower_to_mir` / `lower_hir_to_mir` / `lower_corehir_with_fallback`
   を含む `crates/ark-mir/src/lower/` 全体を削除し、#508 をクローズする。

## Contract for `lower_hir_to_mir` (frozen until retirement)

退役までの期間、Rust 側 `lower_hir_to_mir` は以下の凍結契約を持つ。
新規機能実装は禁止 (ADR-028 違反となる):

- **Input**: `(core_hir: &ark_hir::Program, checker: &TypeChecker, sink: &mut DiagnosticSink)`
- **Output**: `Result<MirModule, String>` — 常に `Ok(MirModule::new())` 相当を
  返し、`MirProvenance::CoreHir` をセットし `corehir-snapshot ...`
  の optimization trace のみを残す (現状そのまま)。
- **Error handling**: CoreHIR `Program` に module が無い場合のみ `Err`。
  それ以外のエラーパスは追加しない。
- **Completeness criteria**: 「常に空 MIR を返す」ことが完全性。
  `lower_corehir_with_fallback` が `mir.functions.is_empty()` を見て
  legacy アームへ抜ける invariant を維持する。
- **Stability**: シグネチャ・戻り値の意味は #529 退役完了まで変更禁止。
  変更が必要になった場合は本 ADR を supersede する後続 ADR を起こすこと。

実装側に追加すべきコメント (別スライスの作業):

```rust
// Frozen per ADR-028: this function is intentionally a no-op until the
// Rust ark-mir crate is retired by #529. Do not add lowering logic here;
// canonical HIR→MIR lowering lives in src/compiler/mir.ark.
```

## Follow-up sub-issues (open under #529, NOT in this slice)

以下のサブイシューは本スライスでは起票しない (design-only slice の境界)。
issue \#529 を担当する agent / メンテナが起票すること。

1. **`selfhost: retire crates/ark-mir lowering surface (lower_to_mir, lower_hir_to_mir, facade)`**
   - **Acceptance sketch**:
     - `crates/ark-mir/src/lower/` 配下のすべての `pub` lowering 関数が
       削除されている、または `cfg(feature = "legacy-lower")` で隔離されている。
     - `src/compiler/mir.ark` 経由の selfhost ビルドのみで全 fixture が pass。
     - `cargo test -p ark-mir` が legacy lowering 抜きで通る。
   - **Depends on**: #529 (selfhost retirement milestone)
   - **Closes (when merged)**: #508, #285 の残項目

2. **`selfhost: docs sweep — retire "legacy lowering removal" framing in compiler docs`**
   - **Acceptance sketch**:
     - `docs/compiler/legacy-path-status.md` と
       `docs/compiler/legacy-path-migration.md` を「Rust クレート退役と
       一体で行う」フレーミングに書き換え、ADR-028 へリンク。
     - `python3 scripts/check/check-docs-consistency.py` が clean。
   - **Depends on**: ADR-028 (this ADR)

3. **`ci: guard — flag any new code added to crates/ark-mir/src/lower/`**
   - **Acceptance sketch**:
     - 新規 PR が `crates/ark-mir/src/lower/{func,facade}.rs` に
       実装行を追加した場合に CI が ADR-028 違反として警告/失敗する
       lint または GitHub Action を追加。
     - false-positive を避けるため、コメント・空行・テストは除外。
   - **Depends on**: ADR-028 (this ADR), #529 への追跡。

## Done criteria (machine-checkable for downstream impl work)

ADR-028 由来の実装作業 (上記サブイシュー群) が完了したと宣言できる条件:

- `rg -n "fn lower_to_mir\b|fn lower_hir_to_mir\b" crates/ark-mir/src/`
  が 0 ヒットを返す。
- `cargo build --workspace` が成功し、`crates/ark-mir/src/lower/`
  ディレクトリが存在しない (または空)。
- `python scripts/manager.py verify quick` が PASS。
- `python3 scripts/check/check-docs-consistency.py` が exit 0。
- `issues/open/508-legacy-path-removal-unblocked-by.md` が `issues/done/`
  に移動している。
- `issues/open/285-legacy-path-deprecation.md` が `issues/done/` に
  移動している (もしくは既に移動済み)。
- selfhost canonical path (`src/compiler/mir.ark`) を経由した全 T1+T3
  fixture が legacy fallback なしで pass。

## Open questions

ADR 採択時点で未確定だが、サブイシュー実装時に解決が必要な点:

1. **selfhost 経由の lowering が Rust テストハーネスから直接呼べるか?**
   現在 Rust テストは `lower_check_output_to_mir` 経由で MIR を組み立てる。
   退役時には Rust テストを selfhost CLI 経由のゴールデン比較に切り替える
   必要がある可能性が高い。**仮定**: #529 の harness 整備で解決済みとして
   進める。未解決ならサブイシュー1の `Depends on` に追加すること。
2. **`MirProvenance::CoreHir` / `LegacyAst` の区別が下流 (validate, opt) で
   依然必要か?** 退役後は `MirProvenance` enum 自体が不要になる可能性。
   **仮定**: provenance は退役と同時に削除する (サブイシュー1に含める)。
3. **`lower_corehir_with_fallback` を呼ぶ external crate / tool は無いか?**
   `compare_check_output_to_legacy` (facade.rs:292) など比較 API が
   存在するため、退役時に利用箇所を一掃する必要がある。**仮定**:
   サブイシュー1のスコープに含める。

これらの仮定は ADR-028 の判断を変更するほどの影響はないため、
本 ADR は以上の前提で ACCEPTED とする。

## References

- `issues/open/285-legacy-path-deprecation.md` — 上流 issue。
- `issues/open/508-legacy-path-removal-unblocked-by.md` — 下流 issue。
- `issues/open/529-100-percent-selfhost-transition-plan.md` —
  Rust クレート退役の親 issue。
- `crates/ark-mir/src/lower/facade.rs:44-116` — `lower_hir_to_mir` 凍結対象。
- `crates/ark-mir/src/lower/facade.rs:251-268` —
  `lower_corehir_with_fallback`、フォールバック分岐。
- `crates/ark-mir/src/lower/func.rs:87` — `lower_to_mir` (legacy 本体)。
- `src/compiler/mir.ark:1127` — selfhost canonical な HIR→MIR lowering の
  起点。
- [ADR-024](ADR-024-selfhost-mir-explicit-cfg-before-ssa.md) — selfhost MIR
  を canonical 表現とする決定。
- [ADR-027](ADR-027-v3-selfhost-completion-skip-v4.md) — v3 完了時点での
  selfhost 完了、Rust 実装は参照実装として保持。
