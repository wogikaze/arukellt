# Archived v1 status

この文書は過去の v1 進捗メモです。現在の実装確認には使わないでください。

## Current source of truth

- [../current-state.md](../current-state.md) — current state including V1 exit criteria
- [policy.md](policy.md) — operational policy including V1 completion gate

## V1 Exit Criteria (cross-reference)

V1 completion is defined as T3 core-wasm compile/run correctness with WasmGC-native data model completion and T1 fallback removal. See `docs/current-state.md` § V1 Exit Criteria for the canonical definition. `--emit component` is **not** part of v1 exit.

## なぜ archive 化したか

以前の `v1-status.md` は milestone の達成状況をまとめていましたが、
現在は:

- `current-state.md` が targets / tests / limitations をまとめている
- 一部 milestone 表記が scaffold 段階のままずれていた
- 利用者が「今何が動くか」を判断するには情報粒度が粗い

ため、履歴資料としてのみ残します。

## いま見るべき文書

- 現在の実装: [../current-state.md](../current-state.md)
- v1 構文プレビュー: [../language/syntax-v1-preview.md](../language/syntax-v1-preview.md)
- T1/T3 移行: [../migration/t1-to-t3.md](../migration/t1-to-t3.md)
