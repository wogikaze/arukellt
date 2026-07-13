# AGENTS.md

## プロジェクトの正本

- 現行のユーザー可視挙動は `docs/current-state.md` と `docs/data/*.toml` を正とする。
- 設計判断は `docs/adr/` を参照する。拘束力があるのは `ACCEPTED` のみで、`PROPOSED` は未採択、`SUPERSEDED` は履歴である。
- 詳細仕様は `docs/rfcs/`、実装計画と一時制限は `docs/plans/`、調査は `docs/research/` に置く。ADR に進捗や一時的な実装上限を書かない。
- 生成物は `docs/directory-ownership.md` に従い、生成元を変更して再生成する。生成済み Markdown を直接直さない。
- 検証コマンド名は `docs/data/verification-commands.toml` を正とする。

## 現行アーキテクチャの制約

- `src/compiler/` のセルフホスト実装をコンパイラ・LSP の正本として扱う。退役済み Rust-era 経路や `crates/**` を既定の変更先・検証前提にしない。
- 公開ターゲットは `wasm32-gc`（primary）、`wasm32`（supported）、`native-cpp` / `native-llvm`（scaffold）。WASI P1/P2/P3 は host profile でありターゲット名ではない。
- `wasm32-gc` が既定でも、実装状態は partial である。ADR の理想形を現行実装済みと誤記しない。
- 公開 API は trait / method / associated function を正規形とする。ユーザー可達 free function を新設・温存しない。例外は非公開 intrinsic のみ（ADR-044、ADR-046）。
- 安定性変更は ADR-014、ユーザー入力から到達するエラー処理は ADR-015、セルフホスト検証は ADR-029 に従う。

## 必須ワークフロー

- 言語意味論、公開 API、ABI、ターゲット、コンパイラ段階、stdlib 移行方針を変える前に `$implementation-strategy` を使う。
- 長期的な設計判断を新設・置換するときは `$architecture-decision` を使う。
- docs、生成元、例、current-state の主張に影響する変更では `$docs-sync` を使う。
- benchmark、baseline、perf threshold を変更するときは `$benchmark-change` を使う。
- コード、テスト、例、ビルド、検証挙動を変更した後は `$code-change-verification` を使う。
- issue を `issues/open/` から `issues/done/` へ移す前は `$issue-close-review` を使う。

## 実装規律

- 依頼の目的、対象、制約、完了条件を先に確定し、必要最小限の差分にする。旧スキルの `PRIMARY_PATHS` 形式は必須ではないが、issue に指定があれば従う。
- 既存 ADR と衝突したら、コードで既成事実化せず設計判断を解決する。
- 仕様変更には最小の回帰試験を追加する。テスト不能な完了主張をしない。
- 無関係なリファクタ、生成物の手編集、baseline による回帰隠し、SKIP の無根拠追加をしない。
- コマンドを実行できない、または失敗した場合は、その事実と未確認範囲を明記する。成功扱いにしない。

## 基本コマンド

- 高速ゲート: `python3 scripts/manager.py verify quick`
- fixture: `python3 scripts/manager.py verify fixtures`
- docs 再生成: `python3 scripts/manager.py docs regenerate`
- docs 検査: `python3 scripts/manager.py docs check`
- 全体: `python3 scripts/manager.py verify full`

変更範囲に応じた追加コマンドは `docs/data/verification-commands.toml` と対象 issue を確認して選ぶ。
