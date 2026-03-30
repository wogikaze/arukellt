# Extension commands / tasks / status bar surfaces

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 190
**Depends on**: 189
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`arukellt-all-in-one` の command surface・task provider・status bar を最小プレースホルダから実用実装に引き上げる。
command graph / environment diff の本実装、task の build / test group と problem matcher の付与、status bar からの target / emit / active project 切り替え導線を含む。

現状は固定 3 タスクのみの task provider、実体のない command graph / environment diff、status bar もプレースホルダにとどまっている。

## Acceptance

- [ ] command surface が実用実装に上がっている（command graph / environment diff を含む）
- [ ] task provider が build / test group と problem matcher を持つ実用実装になっている
- [ ] status bar から target / emit / active project を切り替える導線がある

## Scope

### Commands

- `Arukellt: Command Graph` — 実体実装
- `Arukellt: Show Environment Diff` — 実体実装
- editor title / editor context / explorer context への command 追加
- quick pick ベースの run / test / script 実行

### Task provider

- workspace folder ごとの task 生成
- build group / test group の付与
- problem matcher の定義と適用
- background task 対応
- target / emit を task 定義に反映
- script task 自動生成
- task 実行前 validation

### Status bar

- active target / emit / project の表示
- クリックで quick pick に遷移する導線
- compiling / testing などの状態表示との連携

## References

- `issues/open/189-vscode-extension-package-and-language-client-bootstrap.md`
- `issues/open/184-vscode-extension-foundation.md`
- `extensions/arukellt-all-in-one/src/`
