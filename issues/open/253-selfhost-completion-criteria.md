# セルフホスト達成条件を厳密化し、「できたかどうか」を曖昧にしない

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-04-13
**ID**: 253
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Reopened by audit — 2026-04-13

**Reason**: Acceptance criteria remain unchecked in file. Completion not demonstrated.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

selfhost 周辺は `src/compiler/*.ark`、`docs/compiler/bootstrap.md`、`scripts/run/verify-bootstrap.sh`、`docs/migration/v4-to-v5.md`、`issues/done/209` 以降の一連の issue により前進しているが、「selfhost 用の部品が揃ってきた」ことと「selfhost compiler が達成された」ことがまだ厳密には同義ではない。

## Why this matters

* `scripts/run/verify-bootstrap.sh` は Stage 0 で各 selfhost source を個別に compile し、`main.wasm` が生成されない場合は Stage 1/2 を skip する構造であり、fixpoint が継続検証されていない。
* selfhost compiler が end-user fixture 全体を通すことまで検証されていない。
* Rust 実装と selfhost 実装の parity が日次で崩れていないかを示す CI 契約がまだ弱い。
* selfhost が曖昧なまま進むと「言語仕様の canonical source」「実装の canonical compiler」「バグ修正の適用先」が二重化する。

## Acceptance

* [ ] selfhost 完了条件が 1 行で言える形で文書に固定されている
* [ ] `scripts/run/verify-bootstrap.sh` が skip 前提の scaffold ではなく達成判定の本体になっている
* [ ] Stage1 fixture parity・CLI parity・diagnostic parity・determinism が CI で継続検証されている
* [ ] Rust 実装と selfhost 実装の dual period を終わらせる条件が定義されている
* [ ] `docs/current-state.md` の selfhost 記述が「部品がある」ではなく「どこまで verified か」で表示されている

## Scope

### selfhost 完了条件の定義（→ 266）

* Stage0→Stage1→Stage2 fixpoint、Stage1 fixture parity、CLI parity、diagnostic parity、determinism の各条件を明文化

### verify-bootstrap.sh の昇格（→ 267）

* skip 条件を除去し、全 stage を逐次実行・失敗時に詳細ログを出力する構造に改修

### CI での parity 継続検証（→ 268）

* Stage1 fixture parity・CLI parity・diagnostic parity を CI ジョブとして配線

### dual period 終了条件の定義（→ 269）

* Rust 実装削除のトリガー条件と移行手順を `docs/compiler/bootstrap.md` に記載

### current-state.md の selfhost 記述更新（→ 270）

* 「部品がある」表現を排除し、verified 状態ベースの記述に更新

## References

* `src/compiler/*.ark`
* `docs/compiler/bootstrap.md`
* `scripts/run/verify-bootstrap.sh`
* `docs/migration/v4-to-v5.md`
* `issues/done/209-selfhost-cli-driver-connection.md`
* `issues/open/266-selfhost-completion-definition.md`
* `issues/open/267-verify-bootstrap-upgrade.md`
* `issues/open/268-selfhost-parity-ci-verification.md`
* `issues/open/269-dual-period-end-condition.md`
* `issues/open/270-current-state-selfhost-verified-update.md`
