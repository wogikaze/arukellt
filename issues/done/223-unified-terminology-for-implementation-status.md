# 全文書の「完成/実験/未着手」表記を統一する用語基準を策定する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 223
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

「completed」「implemented」「scaffold」「open issue 0件」「experimental」など、実装状態を表す用語が文書ごとに異なる。
同じ状態を指す言葉が違うため、利用者・開発者とも現在地の判断ができない。
この issue では、全リポジトリで使う実装状態用語の基準を策定し、既存文書に適用する。

## Acceptance

- [x] 実装状態を表す用語セット（stable / provisional / experimental / unimplemented / blocked / removed）が ADR として文書化されている
- [x] 各用語の定義・使い分け基準・表記方法が明確になっている
- [x] 主要文書（README・current-state.md・issues・ADR・仕様書）への適用が完了している
- [x] 新しい文書を書く際の用語選択ガイドが存在する

## Scope

### 用語基準の設計

- 実装状態カテゴリの定義（stable / provisional / experimental / unimplemented / blocked / removed）
- 各カテゴリの判断基準（「何をもって stable とするか」など）
- 文書内での表記方法（バッジ・ラベル・括弧表記など）

### ADR 作成

- 用語基準を ADR として `docs/adr/` に追加
- 既存 ADR の用語を新基準に揃える

### 既存文書への適用

- docs/current-state.md、README、issues/ の用語を統一
- 不整合箇所のリストアップと修正

## References

- `docs/adr/`
- `docs/current-state.md`
- `issues/open/221-rebuild-current-state-as-single-source.md`
