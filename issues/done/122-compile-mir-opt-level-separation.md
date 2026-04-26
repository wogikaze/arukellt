# MIR 最適化パスの --opt-level 分離と passes/ ディレクトリ構造確立

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-04-14
**ID**: 122
**Depends on**: 101
**Track**: mir-opt
**Blocks v4 exit**: yes

---

## Closed — 2026-04-14

All acceptance criteria verified by repo evidence.

**Evidence**:
- `crates/ark-mir/src/passes/const_fold.rs` and `dead_block_elim.rs` — each pass in its own file with unified `fn run(module: &mut MirModule, level: OptLevel) -> PassStats` signature
- `crates/ark-mir/src/passes/mod.rs` — `PassStats` type and `run_all` orchestrator
- `crates/ark-mir/src/opt_level.rs` — `OptLevel` enum (`None`, `O1`, `O2`, `O3`)
- `crates/ark-mir/src/passes/README.md` — full pass catalogue, opt-level table, `--no-pass` docs
- `crates/arukellt/src/main.rs` — `--no-pass=<NAME>` and `--opt-level` flags declared on `Compile` subcommand
- `crates/arukellt/src/commands.rs` — `session.disabled_passes = no_pass` wired in `cmd_compile`
- `crates/ark-driver/src/session.rs` — `disabled_passes` honoured in the pass pipeline filter

**Verification**: `bash scripts/run/verify-harness.sh --quick` → 19/19 PASS; `cargo test -p ark-mir` → 49/49 PASS

## Summary

roadmap-v4.md §6 item 1 で要求されている
`crates/ark-mir/src/passes/` ディレクトリ構造と `OptimizationPass` トレイトを確立する。
現在 `opt/pipeline.rs` に集約している全パスを独立ファイルに分割し、
`--opt-level` による有効/無効の制御を統一インタフェースで実装する。

## 受け入れ条件

- [x] `crates/ark-mir/src/passes/` ディレクトリを新設し、各パスを独立ファイルに移動
- [x] `fn run(module: &mut MirModule, level: OptLevel) -> PassStats` シグネチャの統一
- [x] `--no-pass=<name>` フラグで個別パスを無効化できる
- [x] `passes/README.md` に各パスの説明・適用条件・依存関係を記載

## 参照

- roadmap-v4.md §5.1 および §6 item 1
