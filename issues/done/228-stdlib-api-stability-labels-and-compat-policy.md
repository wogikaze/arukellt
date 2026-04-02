# stdlib 全公開 API に安定性ラベルと互換ポリシーを付与する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 228
**Depends on**: 226
**Track**: main
**Blocks v1 exit**: yes

## Summary

stdlib の全公開 API に安定性ラベル・互換ポリシー・非推奨導線が存在しない。
利用者が stdlib API を使っても「いつまで使えるか」が判断できず、ecosystem の基盤になれない。
この issue では `std/manifest.toml` を基に全 API にラベルとポリシーを付与する。

## Acceptance

- [x] `std/manifest.toml` の全公開 API に stability フィールドが付与されている
- [x] 各 API の互換ポリシー（stable / provisional / experimental）が明記されている
- [x] deprecated API には非推奨導線（代替 API・移行方法）が記述されている
- [x] stdlib リファレンスドキュメントにラベルが反映されている

## Scope

### manifest 更新

- `std/manifest.toml` の `[[functions]]` エントリへの `stability` フィールド追加
- 各関数の安定性判断（現行実装・テスト状況・使用頻度を基準）

### ドキュメント生成

- stability ラベルを stdlib リファレンスに反映するジェネレータ更新
- `python3 scripts/gen/generate-docs.py` で生成される docs への反映

### 互換ポリシー文書

- stdlib 互換ポリシーを `docs/stdlib-compatibility.md` として作成
- バージョン間での API 変更ルール（廃止予告期間など）の明記

## References

- `std/manifest.toml`
- `scripts/gen/generate-docs.py`
- `docs/language/spec.md`
- `issues/open/226-language-spec-stability-labels.md`
