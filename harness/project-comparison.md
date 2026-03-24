# Harness Project Comparison

この文書は、`NEPLg2`、`vibe-lang`、`wado` のハーネス要素を比較するものです。

比較では、次の 2 つを分離して扱います。

- 参照元リポジトリから観測できる事実
- 次プロジェクトへ持ち込むべき標準化判断

## Source Map

比較の根拠にした主なファイルは次です。

| Project | Main sources used for harness extraction |
| --- | --- |
| `NEPLg2` | `AGENTS.md`, `doc/testing.md`, `.github/workflows/ci.yml`, `.claude/settings.json`, `web/package.json` |
| `vibe-lang` | `AGENTS.md`, `justfile`, `.github/workflows/ci.yml`, `docs/adr/0009-scratch-workflow.md` |
| `wado` | `AGENTS.md`, `docs/AGENTS.md`, `mise.toml`, `.github/workflows/ci.yml`, `.claude/settings.json`, `.claude/hooks/mise-setup.sh`, `.claude/skills/*/SKILL.md`, `dprint.json` |

## Comparison Matrix

| Axis | `NEPLg2` | `vibe-lang` | `wado` | Portability judgment |
| --- | --- | --- | --- | --- |
| Primary entrypoint | `AGENTS.md` が `plan.md`、`note.n.md`、`todo.md`、`doc/` 更新まで含めて workflow を駆動する。 | `AGENTS.md` は command-centric で、`just` と MoonBit の semantic tool を明示する。 | `AGENTS.md` が主契約で、`docs/AGENTS.md`、`mise.toml`、CI がそれを補強する。 | 入口文書は短く保ち、queue、ADR、verify、task runner へ案内する。 |
| Planning and queueing | 人手管理の `plan.md`、`note.n.md`、`todo.md` が必須。 | 読んだ範囲では明示的 queue はなく、ADR と scripts に計画圧力が分散している。 | 明示的 issue queue は見えないが、task lifecycle の規律は強い。 | 再利用用ハーネスでは Arukellt 型の `issues/` を採用しつつ、NEPLg2 の「計画差分を可視化する」習慣を残す。 |
| Local bootstrap | 手動寄り。CI には bootstrap build があるが、ローカル導線は集中していない。 | CI の shell install と開発者知識に寄る。 | `mise` が一級。`on-task-started` が tool install を担い、Claude session hook でも自動化できる。 | bootstrap は標準化する。次プロジェクトで手順暗記を前提にしない。 |
| Canonical task runner | Node と shell の生コマンドが中心で、単一 root task file は見えない。 | `justfile` が command catalog の中核。 | `mise.toml` が tool manager と task runner を兼ねる。 | 正式 task surface は 1 つに固定する。複数 runner に truth を分散しない。 |
| Focused verification loop | `nodesrc/tests.js`、`run_doctest.js` を軸にした doctest loop が強い。web artifact が絡むと `trunk build` も入る。 | `justfile` に focused script が多く、heavy gate は通常 `test` と分離されている。 | `mise run test`、`test-wado`、各種 update task が明示され、`on-task-done` で束ねられる。 | 速い focused loop と広い completion gate を両方持つ。片方だけでは足りない。 |
| Completion gate | CI が truth を持つ。compile tests、Rust tests、doctests、multi-emit CLI、doc-site build が分かれている。 | `just release-check` があり、CI では `continue-on-error` の exploratory gate も走る。 | `mise run on-task-done` が最も明示的で、CI integrity job が再生成と dirty-tree reject を行う。 | root verify コマンドに加え、生成物込みの長い completion command を持つ。 |
| Lint and format policy | 集中度は弱い。Rust と TypeScript はあるが、単一 lint 契約は読み取りにくい。 | `moon fmt`、`moon check --deny-warn`、`moon test` が明示的。 | `cargo fmt`、`cargo clippy --all --all-features -- -D warnings`、`dprint fmt` が一級。 | failing lint gate を明示する。Markdown が重要ならそれも整形対象にする。 |
| Generated docs and artifacts | CI が `nodesrc/cli.js` で tutorial HTML と doc HTML を生成し、docs と stdlib 内の doctest が実行可能になっている。 | `moon info` 再生成が通常 command surface に含まれ、coverage や benchmark artifact を出す script も多い。 | `doc-stdlib`、grammar 生成、golden fixture、bundled file、VS Code grammar 再生成まで task 化されている。 | 生成 docs は「生成コマンド名」と「commit policy」を必ず持つ。 |
| Executable docs | `.n.md` と stdlib doc comment が doctest として実行される。 | ADR と guide は強いが、見た範囲では code/test tooling の比重が高い。 | cheatsheet と stdlib docs は compiler の doc surface から生成される。 | 例が重要な場所は executable docs、網羅的 reference は generated docs にする。 |
| CI structure | bootstrap artifact を共有する multi-job CI。build、compile-only、Rust tests、doctests、NM compile が分かれている。 | 広く探索的な CI。blocking job と `continue-on-error` job が混在し、bundle-size report も出す。 | integrity、core tests、最適化別 E2E、Wado module tests、wasm32 check に分離されている。 | integrity と挙動 matrix を分離し、生成物 clean check は 1 job に集約する。 |
| Semantic tooling | `.claude/settings.json` で `context7`、`rust-analyzer`、`typescript-lsp` を有効化。 | `moon ide` と `moon doc` を grep より優先することが明文化されている。 | Claude では rust-analyzer が有効。 | 言語ネイティブな semantic tool があるなら、ハーネスで公式化する。 |
| Agent extensions | plugin 有効化はあるが、repo-local skill library は見えない。 | 読んだ範囲では repo-local hook や skills は見えない。 | hook と skills が成熟している。benchmark、coverage investigation、debugger、formatter principles、vendor sync など。 | agent support は extension layer に留め、shell-equivalent command を必ず併記する。 |
| Documentation governance | 挙動変更時の `doc/` 更新が必須で、testing doc も実務的。 | workflow や設計判断を ADR で積極的に残す。 | `docs/AGENTS.md` が MECE や doc edit 後の format まで定義する。 | docs も governed artifact として扱う。文書量が増えたら doc-specific rule を追加する。 |

## 各プロジェクトから強く持ち帰るべき点

## `NEPLg2`

- docs、doctest、実装を一体で動かす点が強い。
- executable documentation を optional ceremony にしない点が強い。
- 一目で分かる local bootstrap 導線は弱い。
- lint と formatter の契約は 1 ファイルからは読み取りにくい。

## `vibe-lang`

- 多数の focused engineering check を 1 つの task surface から呼べる点が強い。
- heavy workflow を CI YAML の奥に隠さない点が強い。
- 言語設計だけでなく workflow 判断にも ADR を使う点が強い。
- 環境再現性と generated-file cleanliness では `wado` に劣る。

## `wado`

- 環境 bootstrap、生成物運用、end-of-task discipline の総合モデルとして最も強い。
- generator replay と dirty-tree reject を行う integrity CI の実例として最も強い。
- shell command を隠さずに repo-local agent augmentation を持つ点が強い。
- 反面、新規プロジェクトが初手から全部入れるには重すぎるリスクがある。

## 推奨する合成方針

次プロジェクトでは、どれか 1 つを丸ごと移植するのではなく、次の合成方針を採る。

1. Arukellt の短い pointer-doc 入口と `issues/` queue を維持する。
2. `wado` 型の bootstrap と generated-artifact task を単一 runner で管理する。
3. プロジェクトが育ったら、`vibe-lang` 型の coverage、benchmark、self-host、heavy probe command を足す。
4. ユーザー向け言語挙動を説明する箇所では、`NEPLg2` 型の executable docs と sample-driven regression を残す。

## そのままは持ち込まず、後期導入に回す要素

次の要素は有用ですが、baseline harness が固まる前提で後期導入に回すべきです。

- `vibe-lang` の大規模 self-host / component-model script matrix
- `wado` の最適化レベル別 test matrix
- `wado` の repo-local skill library
- `NEPLg2` の専用 WASIX 実行ループ

これらは scale-up feature であり、minimum viable harness の要件ではありません。
