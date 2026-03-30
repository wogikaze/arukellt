# Quickstart を「今後も基準になる書き方」に更新する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 229
**Depends on**: 226, 227
**Track**: main
**Blocks v1 exit**: yes

## Summary

現行の Quickstart は「今だけ動く書き方」になっている可能性がある。
stable でない構文・非推奨の API・暫定的な import スタイルが混入していると、
初学者が書いたコードがすぐ壊れるという最悪の初回体験が生まれる。
この issue では Quickstart を stable な機能のみで構成し直す。

## Acceptance

- [ ] Quickstart に登場する全構文・API が stable または provisional ラベル付きである
- [ ] Quickstart のコードサンプルが現行バージョンで実際に動作する
- [ ] experimental / deprecated な書き方が Quickstart から除去されている
- [ ] 「この書き方は将来も壊れない」という根拠が各サンプルに添えられている

## Scope

### Quickstart コンテンツ監査

- 全サンプルコードの動作確認（現行バージョンで実行）
- 使用構文・API の stability ラベル確認
- experimental / provisional な要素の洗い出し

### Quickstart 書き直し

- stable な書き方のみを使ったサンプルへの更新
- 各ステップに「この構文は stable です」等の注釈追加
- よくある間違いパターンの注意書き追加

### サンプルコードのテスト化

- Quickstart のサンプルを CI でテストする仕組みの検討
- `tests/quickstart/` などでのサンプル保護

## References

- `docs/quickstart.md` (存在する場合)
- `README.md`
- `issues/open/226-language-spec-stability-labels.md`
- `issues/open/227-document-language-contract.md`
