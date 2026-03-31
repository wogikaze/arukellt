# デバッグ体験を「DAP の箱がある」状態から、実際に使える end-to-end workflow にする

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 255
**Depends on**: none
**Track**: main
**Blocks v1 exit**: no

## Summary

`crates/ark-dap` と `arukellt debug-adapter` は存在するが、現状の DAP 実装は `initialize / launch / disconnect` に近い最小応答に留まっている。VS Code 拡張側にも debug contribution や debugger 配線が見当たらず、実際のデバッグ体験はまだ成立していない。

## Why this matters

* `crates/ark-dap/src/lib.rs` は DAP transport を受ける最小 scaffold だが、breakpoint・threads・stackTrace・scopes・variables・continue・step が未実装。
* `extensions/arukellt-all-in-one/package.json` に debugger contribution や launch configuration surface がない。
* compiler / LSP / runtime のどれが悪いかを切り分ける導線としても、デバッグ機能の欠如は致命的。
* DAP が「ある」と docs に書いても、実際にブレークして変数を見られないなら利用者視点では未実装に近い。

## Acceptance

* [ ] `arukellt debug-adapter` が `launch / setBreakpoints / configurationDone / threads / stackTrace / scopes / variables / continue / next / disconnect` を持つ
* [ ] VS Code 拡張から launch でき、`.ark` ソースにブレークポイントを置いて止まる
* [ ] T1/T3 のどこまでを debug 対象とするかが明記され、canonical path で end-to-end が動く
* [ ] DAP smoke test と E2E test が配線されている

## Scope

### DAP 基本動詞の実装（→ 276）

* `threads / stackTrace / scopes / variables / launch` レスポンスの実装

### ブレークポイント・ステップ実行の実装（→ 277）

* `setBreakpoints / configurationDone / continue / next / disconnect` の実装

### VS Code debug contribution の追加（→ 278）

* `package.json` に `debuggers` contribution と launch configuration template を追加

### T1/T3 debug 対象範囲の定義と canonical path E2E（→ 279）

* debug 対象 target の明文化と、`.ark` ファイルにブレークして止まる end-to-end 確認

### DAP テストの配線（→ 280）

* DAP smoke test と E2E test の実装・CI 配線

## References

* `crates/ark-dap/src/lib.rs`
* `extensions/arukellt-all-in-one/package.json`
* `issues/open/276-dap-core-verbs-implementation.md`
* `issues/open/277-dap-breakpoint-step-implementation.md`
* `issues/open/278-vscode-debug-contribution.md`
* `issues/open/279-dap-target-scope-e2e.md`
* `issues/open/280-dap-test-wiring.md`
