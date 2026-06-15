# ADR-034: Component Composition Linking Model

ステータス: **PROPOSED** — Phase 1 scaffold landed (#443); native linker deferred

決定日: 2026-06-15

---

## 文脈

Issue #443 は複数の Wasm Component を 1 つの実行単位に合成する linking モデルを求める。
Upstream #442（WIT interop）と #476（`wac plug` / wasm-tools compose smoke）は完了済み。

Selfhost コンパイラは `std::host::process` に `exit` / `abort` のみを持ち、外部プロセス起動 API がない。
そのため v1 scaffold では **検証 + dependency graph 出力 + `wac plug` 委譲ヒント** を `arukellt compose` に実装する。

---

## 決定

### Phase 1（#443 scaffold — 現状）

`arukellt compose` は `wac plug` 互換の CLI 面を提供する:

```bash
arukellt compose --plug <provider.component.wasm> <socket.component.wasm> -o <output.component.wasm>
arukellt compose --validate --plug <provider> <socket> -o <output>   # 検証のみ
```

実装範囲:

| 機能 | 状態 |
|------|------|
| CLI 面（`--plug`, `-o`, `--validate`） | ✅ scaffold |
| 入力パス存在チェック | ✅ |
| パス衝突検出（provider/socket/output の同一パス） | ✅ |
| dependency graph テキスト出力 | ✅ scaffold |
| import/export WIT 型解決 | ❌ Phase 2 |
| ネイティブバイナリ合成 | ❌ Phase 3 |
| `ark.toml` 依存グラフ連動 | ❌ #124 Phase 3 |

バイナリ合成は `wac plug` に委譲する（#476 smoke と同じツールチェーン）。

### Phase 2 — WIT インターフェース検証

- `wasm-tools component wit` 相当の in-tree WIT 抽出（または subprocess ラッパー）
- provider の export と socket の import の名前/型マッチング
- 不一致時の compile-time 相当エラー（`error: compose conflict: ...`）

### Phase 3 — ネイティブ合成（または恒久委譲）

選択肢:

1. **恒久 `wac` 委譲** — ADR-008 と同様、参照実装に依存（推奨 short-term）
2. **in-tree linker** — `component/` emitter と対称の composition writer（long-term）

---

## Dependency Graph モデル

```
[provider.component.wasm] --plug--> [socket.component.wasm] ==> [output.component.wasm]
```

将来の `ark.toml` 連動:

```toml
[compose]
output = "dist/app.component.wasm"

[[compose.plugs]]
provider = "deps/math-lib.component.wasm"
socket = "target/runner.component.wasm"
```

`arukellt compose`（引数なし）はプロジェクトルートの `ark.toml` `[compose]` を読み、
上記グラフをトポロジカル順に検証・実行する（#124 Phase 3）。

---

## 検証

- CLI parity: `scripts/selfhost/checks.py` case 18–19
- Fixture: `tests/component-interop/compose/validate.sh`
- End-to-end plug: `tests/component-interop/compose/run.sh` (#476, `ARUKELLT_TEST_COMPOSE=1`)

---

## 影響

- `src/compiler/main/compose_cmd.ark` — Phase 1 実装
- `docs/cli-reference.md` — compose セクション
- Issue #443 — acceptance 5 項目中 1 項（CLI）のみ scaffold 完了; issue は open のまま

---

## 関連

- [ADR-008-component-wrapping.md](ADR-008-component-wrapping.md)
- [../platform/wasm-features.md](../platform/wasm-features.md)
- Issue #443, #476, #124
