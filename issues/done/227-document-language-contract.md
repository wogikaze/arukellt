# 構文・型システム・import・visibility・error model の契約を文書化する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 227
**Depends on**: 226
**Track**: main
**Blocks v1 exit**: yes

## Summary

構文、型システム、import 方式、visibility ルール、error model の各仕様が十分に文書化されていない。
利用者が大規模コードを組もうとした際に「どう書けばよいか」が仕様から判断できない。
この issue では、これらの核心部分を利用者向けの契約文書として整備する。

## Acceptance

- [x] 構文の全要素（式・文・パターン・item）が仕様書に網羅されている
- [x] 型システムの規則（推論・強制・型エラーの条件）が文書化されている
- [x] import 方式と解決規則が一意に定まる仕様として記述されている
- [x] visibility（pub / priv / module 境界）の規則が明文化されている
- [x] error model（panic / Result / host error の扱い）が文書化されている

## Scope

### 構文契約

- 全構文要素の BNF / 準形式的記述
- 曖昧な構文の解消規則（優先順位・結合性）
- 既知の制約・制限の明記

### 型システム契約

- 型推論の範囲と限界
- 型エラーが発生する条件
- 型変換の暗黙/明示の境界

### import・module・visibility 契約

- `import` の解決順序と探索パス
- `pub` / `priv` の意味と module 境界の定義
- circular import の扱い

### error model 契約

- `panic` が起きる条件と保証
- `Result` 型の利用規則
- host error の伝播方法

## References

- `docs/language/spec.md`
- `issues/open/226-language-spec-stability-labels.md`
- `issues/open/233-module-package-workspace-resolution-spec.md`

## Completion Note

Closed 2026-04-09. docs/language/spec.md (1330 lines) covers: (1) full syntax in BNF/sections 1-7 with operator precedence table; (2) type system §2 with inference, coercion, generics; (3) imports §7 with resolution rules; (4) visibility §7.3 (pub/priv/module boundary); (5) error model in §10 error codes + Result/panic in §9. All stability-labeled per ADR-014.
