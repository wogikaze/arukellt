# Harness Tooling Catalog

この catalog は、参照リポジトリの運用を次プロジェクト向けの tooling choice に落とし込むものです。

判断ラベルは次の 3 つです。

- 今すぐ採用
- 条件付き採用
- デフォルトでは採用しない

## Tooling Decisions Table

| Category | Reference patterns | Recommendation |
| --- | --- | --- |
| Environment bootstrap | `wado` は `mise` task と Claude startup hook を持ち、`vibe-lang` と `NEPLg2` は手動 shell setup 寄り。 | 今すぐ採用: 外部 tool や generated artifact が複数ある repo では `mise` を使う。 |
| Task runner | `vibe-lang` は `just`、`wado` は `mise`、`NEPLg2` は raw command 中心。 | 今すぐ採用: canonical runner を 1 つに決める。polyglot なら `mise`、軽量なら `just`。 |
| Root verify command | Arukellt は `scripts/run/verify-harness.sh` を持ち、参照 repo は CI、`just`、`mise` で completion surface を定義。 | 今すぐ採用: 短い root verify script を常に持つ。 |
| Formatter and lint | `vibe-lang` は `moon fmt` と `moon check --deny-warn`、`wado` は `cargo fmt`、`cargo clippy -D warnings`、`dprint fmt`。 | 今すぐ採用: production code は warning で fail させ、docs が重要なら Markdown も整形する。 |
| Executable docs | `NEPLg2` は docs と stdlib comment の doctest、`wado` は generated stdlib docs。 | 今すぐ採用: 例は executable docs、網羅参照は generated docs にする。 |
| Golden and generated artifacts | `wado` は named regeneration task と dirty-tree CI、`vibe-lang` も explicit script が多い。 | generated output を commit するなら今すぐ採用。 |
| Coverage and benchmark tasks | `vibe-lang` と `wado` で厚く、`NEPLg2` では最小。 | 条件付き採用: baseline verify loop が安定してから足す。 |
| Semantic code navigation | `vibe-lang` は `moon ide` と `moon doc` を必須化し、`wado` は rust-analyzer を有効化。 | 言語エコシステムが対応しているなら今すぐ採用。 |
| Agent hooks and skills | `wado` だけが成熟、`NEPLg2` は plugin enablement が中心。 | 条件付き採用: extension layer に留め、core harness にしない。 |

## Bootstrap and Task Surface

## `mise`

重要な理由:

- pinned tool を install できる
- named task を持てる
- polyglot repo と相性がよい
- `on-task-started` や `on-task-done` のような lifecycle command を表現できる

今すぐ採用すべき条件:

- Rust、Node、Wasmtime、formatter、generator が混在する
- 新規 contributor に install 手順を暗記させたくない
- CI と local で tool version を揃えたい

参照価値:

- `wado/mise.toml` は environment と task orchestration を 1 か所で持つ最も分かりやすい例

## `just`

重要な理由:

- concise な command catalog を作れる
- focused engineering task の discoverability が高い
- 導入コストが低い

今すぐ採用すべき条件:

- tool bootstrap が別で解決済み
- repo が shell script の上に clean な command surface だけ必要としている

参照価値:

- `vibe-lang/justfile` は、多数の focused check を YAML の奥へ埋めずに公開する実例

デフォルト採用を避ける条件:

- pinned external tool install や lifecycle hook まで必要

## Raw shell commands

それでも重要な理由:

- どの task runner も最終的には shell に落ちる
- docs には trust と debuggability のための生コマンドも必要

参照価値:

- `NEPLg2` は specialized doctest と doc-site workflow では raw command が依然有効だと示している

rule:

- raw shell command は docs か scripts に見える形で残しつつ、唯一の harness surface にはしない

## Verification Tooling

## Focused test commands

今すぐ採用。

次プロジェクトは次の narrow command を持つべきです。

- 1 つの fixture group
- 1 つの doctest source
- 1 つの module または crate
- 1 つの CLI smoke path

参照パターン:

- `NEPLg2`: `node nodesrc/tests.js ...`、`node nodesrc/run_doctest.js ...`
- `vibe-lang`: fixture isolation、typecheck fixture、warning fixture、component test、self-host gate などの focused `just` task
- `wado`: `cargo test` の slice、`mise run test-wado`、最適化別 E2E

## Root completion gate

今すぐ採用。

次プロジェクトは、次に相当する root command を必ず持つべきです。

```bash
./scripts/run/verify-harness.sh
```

これは trust できる短さと、completion を定義できる広さの両方を持たせます。

## Integrity regeneration

generated output を commit する repo なら採用。

参照パターン:

- `wado` は generator を再実行し、docs を format し、golden を更新し、それで tree が dirty になったら CI を落とす

これは次の生成物に向くモデルです。

- generated docs
- syntax file
- fixture golden
- bundle output

## Formatter and Linter Stack

## Rust

Rust-heavy repo なら今すぐ採用。

- `cargo fmt --check`
- `cargo clippy ... -- -D warnings`

参照パターン:

- Arukellt `scripts/run/verify-harness.sh`
- `wado` integrity task

## Markdown

Markdown が product-facing または generated なら今すぐ採用。

参照パターン:

- `wado` は `mise run format` の中で `dprint fmt` を走らせる

重要な理由:

- generated docs による review churn を減らせる
- 複数文書のスタイルを揃えられる

## Language-native type/lint checks

使えるなら今すぐ採用。

参照パターン:

- `vibe-lang`: `moon check --deny-warn --warn-list ...`
- `NEPLg2`: `web/package.json` の TypeScript build と、言語 docs を意味的に検証する doctest tooling

rule:

- 言語ネイティブな check は editor setup ではなく harness に入れる

## Docs and Generated Reference Material

## Executable docs

言語例や user-facing snippet には今すぐ採用。

参照パターン:

- `NEPLg2` は `.n.md` docs と stdlib comment に挙動例を置き、doctest runner で実行する

向いている用途:

- language tutorial snippet
- stdlib example
- raw fixture より docs として置いた方が分かりやすい bug reproduction

## Generated reference docs

言語や CLI に structured API surface があるなら今すぐ採用。

参照パターン:

- `wado`: `doc-stdlib`、cheatsheet 生成、syntax/grammar file 生成
- `vibe-lang`: `moon info` 再生成と generated contract table

向いている用途:

- API index
- stdlib reference
- syntax または grammar export
- editor support file

rule:

- すべての generated doc は named source と named regeneration command を持つ

## CI Patterns

## Integrity job

今すぐ採用。

integrity job が持つべき責務:

- formatter check
- lint
- generator replay
- dirty-tree detection

参照パターン:

- `wado/.github/workflows/ci.yml`

## Behavior matrix jobs

runtime、target、最適化レベルが複数ある段階で採用。

参照パターン:

- `NEPLg2` の compile-test、rust-test、doctest、NM compile 分離
- `wado` の core tests、E2E O0/O1/O3/Os、Wado module tests、wasm32 check 分離
- `vibe-lang` の smoke、bundle size、wasm quick gate 分離

rule:

- integrity concern と broad runtime/performance coverage を分離する

## Exploratory non-blocking jobs

不安定領域を探っている段階で採用。

参照パターン:

- `vibe-lang` は exploratory job と regression sensing job の一部で `continue-on-error` を使う

向いている用途:

- unstable backend
- performance probe
- parity experiment

project の core correctness gate には使わない。

## Agent Layer

## Plugins and LSP

stable な言語 support を標準化する価値が出た段階で採用。

参照パターン:

- `NEPLg2`: `context7`、`rust-analyzer`、`typescript-lsp`
- `wado`: `rust-analyzer`

rule:

- 想定 plugin は明記するが、primary workflow は shell command に置く

## Session hooks

environment drift が recurring issue になったら採用。

参照パターン:

- `wado/.claude/hooks/mise-setup.sh` は remote session で `mise` を install し activate する

良い用途:

- safe な environment setup
- PATH activation
- project tool config の trust

悪い用途:

- hidden file mutation
- 現タスクと無関係な高コスト background work

## Repo-local skills

高複雑度 workflow が繰り返し現れ始めた段階で採用。

参照パターン:

- benchmark refresh
- coverage investigation
- debugger usage
- formatter principle
- vendor submodule sync

rule:

- skills は expert workflow を包み、repository の基本事実までは抱え込まない

## Starter Stack for the Next Project

次プロジェクトを今日始めるなら、推奨 stack は次です。

- canonical tool / task manager としての `mise`
- root completion gate としての `scripts/run/verify-harness.sh`
- operational queue としての `issues/`
- workflow decision を残す `docs/adr/`
- 重要な例を支える executable docs
- 網羅参照を支える generated reference docs
- generated drift で fail する 1 本の integrity CI job
- 上が安定してから足す optional な `.claude/` hooks と skills

この stack は、立ち上げやすさと scale しやすさの両方を満たします。
