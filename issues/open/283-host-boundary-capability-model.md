# ホスト境界と capability model の固定

**Status**: open
**Created**: 2026-03-31
**ID**: 283
**Depends on**: —
**Track**: main
**Blocks v1 exit**: no
**Priority**: 3

## Summary

標準ライブラリは広がっているが、「どの環境で何が使えて、何が拒否されるか」の契約がまだ穴だらけ。`--deny-clock` `--deny-random` は hard-error placeholder 扱い、`--dir` は存在しない、`std/host/env.ark` `std/host/http.ark` `std/host/sockets.ark` には stub が残っている。host API を増やすことではなく、capability surface を縮めて固定することが先。

## Current state

- `--deny-clock`, `--deny-random`: placeholder（hard error にするだけで compile-time 検証なし）
- `--dir`: 未実装（filesystem access policy がない）
- `std/host/env.ark`: stub 関数あり
- `std/host/http.ark`: stub 関数あり
- `std/host/sockets.ark`: stub 関数あり
- capability の deny が runtime stub で、compile-time / target-time に落ちていない
- CI で host API の run-time 検証が不完全

## Acceptance

- [ ] capability surface の公式リストが文書化される（何が使えて何が使えないか）
- [ ] `--deny-clock` が compile-time に clock 関連 import を拒否する（runtime ではなく）
- [ ] `--deny-random` が compile-time に random 関連 import を拒否する
- [ ] `--dir <path>` フラグが WASI の filesystem preopens を制御する
- [ ] stub 状態の host module (`http`, `sockets`) が明示的に「未実装」とマークされ、使用時に compile error になる
- [ ] `env` module の stub 関数が実装されるか、未実装なら compile error になる
- [ ] 使用可能な host API に対して CI で run-time テストがある
- [ ] `std/manifest.toml` の capability 情報が正確に反映される
- [ ] `docs/current-state.md` §Known Limitations が更新される

## Approach

1. host module の stub を棚卸し: 実装済み / stub / 未実装を分類
2. capability deny を compile-time check に引き上げ（MIR lowering 時に import を検査）
3. `--dir` フラグを CLI に追加し、WASI preopens に渡す
4. stub module の使用を compile error にする gate を追加
5. 使用可能な host API の run-time テストを `tests/fixtures/` に追加
6. `std/manifest.toml` の target / capability 情報を整合
7. docs 更新

## References

- `docs/current-state.md` §Known Limitations
- `std/host/env.ark`, `std/host/http.ark`, `std/host/sockets.ark`
- `std/manifest.toml`
- `crates/arukellt/src/main.rs` (CLI flags)
- `crates/ark-wasm/src/emit/t3/mod.rs` (WASI import generation)
