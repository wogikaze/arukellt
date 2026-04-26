# Harness Blueprint for the Next Project

この blueprint は、次プロジェクトへ持ち込む標準ハーネスを定義します。

意図的に強い推奨を含めています。目的は、次のセットアップで実装者に判断を残さないことです。

## Core Principles

1. 人間向け入口文書は短く保つ。
2. executable source of truth は説明 prose より優先する。
3. 「完了」は 1 本の root verification command に対応づける。
4. 生成物には update command と commit policy を必ず持たせる。
5. CI は integrity drift と behavioral regression を分離する。
6. agent-specific automation は optional に留め、唯一の作業手段にしない。

## Recommended Layout

次のレイアウトを baseline とする。

```text
AGENTS.md
docs/process/agent-harness.md
docs/adr/ADR-0001-agent-harness-entrypoint.md
issues/index.md
issues/open/
issues/done/
scripts/manager.py
mise.toml            # preferred for polyglot or generated-artifact-heavy repos
justfile             # optional thin wrapper, never the second source of truth
.claude/settings.json
.claude/hooks/
.claude/skills/
docs/generated/      # or another explicit generated-docs directory
```

まだ agent hooks や repo-local skills が不要なら、`.claude/` は丸ごと省いてよいです。core harness はそれでも成立しなければなりません。

## Mandatory Files and Their Jobs

## `AGENTS.md`

これは repo boundary contract です。

定義すべき内容は次です。

- working directory expectation
- layering rule
- extension order
- 完了宣言前に必要な verification
- generated docs や fixture に関する rule

changelog や architecture manual の重複にしてはいけません。

## `docs/process/agent-harness.md`

これは最短の human entrypoint です。

指すべき先は次です。

- `issues/index.md`
- `AGENTS.md`
- harness shape を説明する最初の ADR
- `scripts/manager.py`

workflow detail をすべて inline してはいけません。詳細は queue、tests、runner task、ADR に置きます。

## `issues/index.md`

これは live queue です。

含めるべき内容は次です。

- active work
- blocked work
- recently done
- file link と summary を持つ machine-readable index block

queue は project の operational memory です。散在した free-form note で置き換えてはいけません。

## `docs/adr/ADR-0001-agent-harness-entrypoint.md`

これは、なぜハーネスがその形なのかを残す文書です。

次のように workflow choice が重要なときは ADR を使います。

- なぜ queue が必要か
- なぜ verify command が 1 本なのか
- なぜ generated drift で CI を落とすのか
- なぜ特定の task runner を canonical にするのか

## `scripts/manager.py`

これは minimum deterministic local completion gate を定義します。

性質は次です。

- short
- shell-native
- readable
- docs や issue から参照できる程度に stable

ここには、特別な文脈なしで全 contributor がローカル実行できる、最も厳しい fast check を入れます。

## Task Runner Standard

canonical runner は 1 つだけ選びます。

推奨 default:

- repo が polyglot、generated docs を持つ、pinned external tool が必要、または setup hook の恩恵が大きいなら `mise`

許容する代替:

- tool bootstrap が別で解決済みで、task layer に concise command alias だけ必要なら `just`

hard rule:

- `mise.toml` と `justfile` が共存するなら、一方を明示的に subordinate にする
- 実コマンドの source of truth を contributor に推測させない

## Verification Model

verification は 3 層に分けます。

## Layer 1: focused local loop

例:

- 1 つの doctest file
- 1 つの fixture group
- 1 つの module test
- 1 つの CLI smoke command

この層は `NEPLg2` と `vibe-lang` が強いです。

## Layer 2: root verify gate

`scripts/manager.py` から呼ぶ層です。

典型的な内容:

- formatter drift check
- strict lint gate
- queue consistency または metadata test
- main test suite

これは完了宣言の minimum standard です。

## Layer 3: completion or integrity regeneration

generator を持つ repo では `mise run on-task-done` のような広い task を使います。

典型的な内容:

- format
- repo が意図的に使うなら clippy-fix または equivalent update pass
- generated docs
- golden fixture
- syntax または grammar export
- full tests

CI はこの層を dedicated integrity job で replay し、working tree が dirty になったら失敗させます。

## Generated Artifacts Policy

生成物運用は次の方針をそのまま採用します。

1. 生成ファイルの class ごとに 1 つの named update command を持たせる。
2. update command は CI だけでなく task runner にも載せる。
3. generated output を commit するか ignore するかを repo が明示する。
4. commit するなら CI が generator を再実行して drift を reject する。
5. commit しない場合でも、verify script は generator が動くことを保証する。

参照プロジェクトの良い例:

- `NEPLg2`: source docs と stdlib 入力から HTML docs を生成
- `vibe-lang`: `moon info`、coverage report、contract table、self-host bundle
- `wado`: stdlib docs、golden fixture、grammar file、WIT 由来の bundled library

## Documentation Model

次プロジェクトの docs は 3 tier に分けます。

1. short operational docs
   `AGENTS.md`、`docs/process/agent-harness.md`、queue files
2. decision docs
   ADR または WEP 風の判断記録
3. executable または generated reference docs
   doctest、reference export、cheatsheet、syntax table、fixture-backed guide

例を実行できるのに narrative-only prose へ流してはいけません。

## Agent Extension Layer

repo-local agent augmentation は推奨ですが optional です。

標準形は次です。

- plugin enablement や session hook のための `.claude/settings.json`
- environment bootstrap などの safe startup work を置く `.claude/hooks/`
- 繰り返し価値が高い workflow を包む `.claude/skills/*/SKILL.md`

この層の rule:

- すべての hook は明確な non-agent shell equivalent を持つ
- hook は環境準備まではよいが、hidden destructive change をしてはいけない
- skills は coverage investigation、benchmark refresh、formatter principle、vendor sync のような「覚えるコストが高い workflow」を包む
- agent extra を task 完了の唯一の documented path にしてはいけない

## Adoption Sequence

ハーネス導入は 3 phase で進めます。

## Phase 0: minimum viable harness

- `AGENTS.md`
- `docs/process/agent-harness.md`
- `issues/`
- 1 本の ADR
- 1 本の root verify script
- 1 つの canonical runner

## Phase 1: generated-artifact discipline

- explicit update task
- dirty-tree detection を含む integrity CI
- doc generation task
- golden または snapshot refresh task

## Phase 2: advanced automation

- benchmark task
- coverage task
- self-host または multi-backend gate
- repo-local agent hook と skills

## Reject する Anti-Pattern

- compiler や language の truth を CLI wrapper の中へ隠すこと
- partially-overlapping な verify script を複数持つこと
- CI YAML を唯一の workflow documentation にすること
- regeneration command なしで generated file を commit すること
- plain shell explanation のない agent magic を足すこと
- docs と tests が別の挙動を説明すること
