# API 安定性ラベルとドキュメント体系

**Status**: done
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 058
**Depends on**: 057
**Track**: stdlib
**Blocks v3 exit**: yes

## Summary

全 stdlib API に Stable / Experimental / Internal の三段階ラベルを付与し、
reference / cookbook / migration の三分割ドキュメント体系を確立する。
全 public API に executable example を付与し、harness で検証可能にする。

## 背景

std.md §12 は API 安定性ラベルとドキュメント運用ルールを明記。
v3 の exit criteria として「全 Stable API に example-first docs がある」ことを要求。

## 受け入れ条件

### 安定性ラベル

`std/manifest.toml` に `stability` フィールドを追加:

```toml
[[functions]]
name = "vec_new"
module = "std::collections::vec"
stability = "stable"  # stable | experimental | internal

[[functions]]
name = "rope_insert"
module = "std::text::rope"
stability = "experimental"
```

### ラベル基準

| ラベル | 意味 | 対象 |
|---|---|---|
| Stable | 後方互換保証。破壊的変更は major version のみ | Vec, String, HashMap, HashSet, path, fs, io, time, test の基礎面 |
| Experimental | 設計継続中。minor version で API 変更あり | Rope, SlotMap, Arena, Interner, wasm, wit, component, json, toml, csv |
| Internal | compiler/runtime 専用。公開保証なし | intrinsic wrapper, backend helper |

### ドキュメント体系

1. **Reference** (`docs/stdlib/*-reference.md`): 全 public API を網羅する型・関数一覧
2. **Cookbook** (`docs/cookbook/`): タスク別レシピ (JSON 処理、CLI、Wasm binary emit 等)
3. **Migration** (`docs/migration/v2-to-v3.md`): 旧 API → 新 API の写像表

### Example 要件

全 Stable API に最低 1 つの executable example を付与。
example は `tests/fixtures/examples/` に配置し、harness で compile/run 検証。

## 実装タスク

1. `std/manifest.toml`: 全関数に `stability` フィールドを追加
2. `scripts/check/check-stdlib-manifest.sh`: stability フィールドの必須チェックを追加
3. `docs/stdlib/` 配下に各モジュールの reference doc を作成
4. `docs/cookbook/`: 5 つ以上のレシピを作成 (JSON 処理、CLI ツール、ファイル処理、Wasm binary、テスト)
5. `tests/fixtures/examples/`: 各 Stable API の example fixture を作成
6. verify-harness.sh に API stability consistency check を追加

## 検証方法

- `scripts/check/check-stdlib-manifest.sh` が stability 未設定の public 関数を検出
- 全 example fixture が pass
- reference doc にリンク切れがないこと

## 完了条件

- manifest.toml の全 public 関数に stability ラベルが設定されている
- Stable API の 100% に executable example がある
- reference doc が全モジュール分存在する
- cookbook が 5 レシピ以上
- migration guide が存在する

## 注意点

1. ラベル付与は保守的に: 迷ったら Experimental にする
2. reference doc は自動生成を目指すが、v3 では手書きで可
3. example は prelude + explicit import の両方のパターンを示す

## ドキュメント

- `docs/stdlib/stability-policy.md`: Stable/Experimental/Internal の定義と運用ルール
- `docs/stdlib/README.md`: stdlib 全体の導線 (どのドキュメントを見ればよいか)

## 未解決論点

1. API stability の promise を semver に連動させるか (v3.x.y で Stable は x が上がるまで保持)
2. `@stable` / `@experimental` アノテーション構文を言語に入れるか
3. 自動生成 reference doc のフォーマット (markdown vs HTML)
