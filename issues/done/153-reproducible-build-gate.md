# 横断検証: bit-exact Wasm 再現ビルドゲートと決定性ルール

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-15
**ID**: 153
**Depends on**: —
**Track**: cross-cutting
**Blocks v1 exit**: no

---

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/153-reproducible-build-gate.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/process/roadmap-cross-cutting.md` §6.5 / §6.6 は、
同一入力から同一 `.wasm` を生成する再現ビルド検証と、HashMap iteration 順序・emitter 出力順の決定性を要求している。
現状は perf baseline や WAT roundtrip はあるが、bit-exact reproducible build gate は verify-harness に入っていない。

## 受け入れ条件

1. 同一入力を複数回 compile して bit-exact に一致することを確認する check が追加される
2. 決定性に影響する要素 (関数順、type 順、HashMap iteration 順など) の運用ルールが文書化される
3. 再現ビルド失敗時に diff を追える導線がある
4. `scripts/run/verify-harness.sh` か専用 script から再現ビルド check を呼べる

## 実装タスク

1. 現在の emitter / benchmark / harness で非決定化しうる点を棚卸しする
2. 同一 fixture を 2 回以上 compile して bytes compare するスクリプトを追加する
3. failure 時に WAT / phase dump / checksum など比較材料を出す
4. 決定性ルールを process / compiler docs に反映する

## 参照

- `docs/process/roadmap-cross-cutting.md` §6.5, §6.6
- `scripts/run/verify-harness.sh`
- `scripts/run/wat-roundtrip.sh`
- `docs/compiler/pipeline.md`

---

## 完了サマリ (2026-04-15)

全ての受け入れ条件を達成した:

1. ✅ `scripts/gate/check-reproducible-build.sh` — 同一 fixture を 2 回コンパイルして sha256 を比較
2. ✅ `docs/process/roadmap-cross-cutting.md` §6.5「再現ビルド検証 — 決定性の運用ルール」を追加 (関数順・型順・HashMap iteration 順等の表)
3. ✅ 差分時に WAT diff + バイナリ diff を出力 (wasm2wat があれば WAT、常に `cmp -l` で差分)
4. ✅ `scripts/run/verify-harness.sh --repro` および `--full` から呼び出し可能
