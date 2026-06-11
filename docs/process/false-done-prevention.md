# False-Done 再発防止

> 監査オーケストレーター契約: `prompts/research.md`  
> 監査ログ: `docs/process/false-done-audit-2026-06-12.md`  
> 更新: 2026-06-12

この文書は **false-done がなぜ起きるか** と **二度と起きないためのゲート** を定義する。  
issue を `done` にする前に reviewer / verifier が必ず照合する。

## 定義

**False-done**: `issues/done/` にあるが、repo 内証拠（現物・fixture・verify）で acceptance が満たされていない状態。

証拠にならないもの（単独では done 不可）:

- issue 本文の `[x]`
- ADR / docs / README の主張
- 外部 URL
- 「将来やる」「blocked」「partial」のメモだけ
- bootstrap / stub / passthrough 実装の存在のみ

## 根本原因カテゴリ（観測済み）

| ID | パターン | 典型例 | 再発防止 |
|----|----------|--------|----------|
| FD-01 | **監査メモだけ移動** | 「Moved to `issues/open/`」と書いたがファイルが `done/` に残る | reopen は **必ず `git mv`** + index 再生成 + 同一 wave でコミット |
| FD-02 | **Status とディレクトリ不一致** | frontmatter `Status: open` または `**Status**: open` が `issues/done/` に存在 | `generate-issue-index.py` 前にパスと Status を突合；verify の issue hygiene を通す |
| FD-03 | **docs-only close** | 実装 track が残っているのに ADR/docs slice だけで `done` | 1 issue = 1 product claim；docs slice は別 issue か acceptance 内の明示サブセット |
| FD-04 | **parse / stub を製品と混同** | `typecheckSource()` が `parseSource()` のラップのみ | user-visible claim には **専用 fixture**（parser と異なる phase/code） |
| FD-05 | **削除済み crate を evidence に引用** | `crates/ark-playground-wasm` 削除後も close evidence が残る | close gate は **現時点のパス** のみ引用；削除 issue とセットで evidence 更新 |
| FD-06 | **parent gate 未達で child/parent を done** | #074 P2 native close gate 未達のまま #510 が done 扱い | 親 issue の close gate checklist を子より先に機械化 |
| FD-07 | **bootstrap stub を本番と同一視** | `BOOTSTRAP_COMPONENT_STUB` passthrough を component done の根拠にする | fixpoint bootstrap と stage-N 本番 compiler を分離；component は **非 bootstrap** fixture で証明 |
| FD-08 | **部分実装を acceptance 全体で close** | `std::random` のみ完了で #051 全体を done | サブサーフェス完了は **split issue** か acceptance の明示的スコープ縮小 + 未達を open 化 |
| FD-09 | **「remains open」後に無 close** | 本文に `remains open` があり、その後の Close note がない | 監査スクリプト: `remains open` より後に `Close note` / `Completion` が無ければ flag |
| FD-10 | **verify をスキップして close** | `wasm-tools validate` skipped と書いてあるのに done | acceptance の required verification は **省略不可**；SKIP は semantic justification 必須 |

## Close 前チェックリスト（reviewer 必須）

1. **Directory**: ファイルが `issues/done/` にあり、frontmatter `Status: done`
2. **Evidence list**: close 時に列挙できる repo 内パスが acceptance ごとに 1 つ以上
3. **Fixture / verify**: `python3 scripts/manager.py verify quick` 通過；該当 scope は fixture または専用 check
4. **User-visible**: entrypoint / route / command / UI が repo で grep 可能
5. **No stale reopen**: 「Reopened by audit」が残る場合、その後の **resolution または re-close evidence** がある
6. **Future work**: `deferred` / `future work` があるなら対応 **open issue** が存在
7. **Bootstrap vs prod**: selfhost bootstrap パスと本番 compiler パスの差を明記

## Reopen 後の done 復帰契約（テスト追加）

false-done と判定して `issues/open/` に戻した issue は、次を満たすまで **再 close 禁止**:

1. **Close-gate fixture** を `tests/fixtures/` に追加（または既存 fixture を acceptance に紐付け）
2. `tests/fixtures/manifest.txt` に登録（`run:` / `compile:` / `component-compile:` / `diag:` 等）
3. issue 本文の **Required verification** に再現コマンドを記載
4. `python3 scripts/manager.py verify quick` が緑

### 既知 reopen 群と必要なテスト種別

| Issue | 必要な close-gate 証拠 |
|-------|------------------------|
| #074 | P2 stdio + `wasi:cli/run` export + wasmtime run fixture（bootstrap 非依存） |
| #510 | `--wasi-version p2 --emit component` + `wasm-tools validate` |
| #472 / #500 | playground typecheck が parse と区別できる fixture / worker 経由の real checker |
| #051 | `stdlib_time/monotonic.ark` compile+run；`__intrinsic_clock_*` emitter ハンドラ |
| #123 | Layer C 実装 acceptance（docs 以外）に対応する module/fixture |

## 運用ルール

- **監査 wave 完了ごと**に orchestration-state のみコミット（`prompts/research.md`）
- 監査レポートは `docs/process/false-done-audit-YYYY-MM-DD.md` に追記
- bulk reopen 禁止: 各 issue は **repo 証拠** 付きで個別判定
- `issues/done/` 全件監査は track 別 subplanner に分割（component / stdlib / playground / LSP / release）

## 機械チェック（今後の CI 候補）

- `issues/done/**/*.md` で `Status: open` を検出
- `Moved to issues/open` があり同名ファイルが `issues/open/` に無い
- `remains open` より後に `Close note` / `Completion` が無い
- done issue が参照するパスが repo に存在しない

機械チェックは verify quick に登録済み:

- `python3 scripts/check/check-false-done-hygiene.py` — FD-01 / FD-02 / duplicate ID
- `python3 scripts/check/check-false-done-close-gates.py` — reopen 済み issue が `issues/done/` に戻ったとき acceptance gate を強制
- `playground/src/tests/typecheck-close-gate.test.ts` — #472 / #500 用（issue が done のとき gate 経由で実行）
