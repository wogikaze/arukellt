# ADR-034: Component 合成を wac plug に委譲

ステータス: **ACCEPTED** — Phase 3 wac 委譲 landed (#443, 2026-06-15)

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
| import/export WIT 型解決 | ✅ Phase 2（`.wit` sidecar マッチング） |
| ネイティブバイナリ合成 | ✅ Phase 3 (`wac plug` 委譲 — `arukellt compose` + selfhost wrapper) |
| `ark.toml` 依存グラフ連動 | ❌ #124 Phase 3 |

バイナリ合成は `wac plug` に委譲する（#476 smoke と同じツールチェーン）。

### Phase 2 — WIT インターフェース検証（2026-06-15 部分完了）

- `.wit` sidecar から world import/export を抽出（`provider.wasm` → `provider.wit`）
- provider export と socket import の名前/func シグネチャマッチング
- 不一致時: `error: compose conflict: ...`
- 未実装: component バイナリからの in-tree WIT 抽出（`wasm-tools component wit` 相当）

### Phase 3 — `wac plug` 恒久委譲（2026-06-15 完了）

`arukellt compose`（`--validate` なし）は検証後に `wac plug --plug <provider> <socket> -o <output>` を実行する。
`wac` が PATH に無い場合は delegate hint を出力して非ゼロ exit。

- 実装: `src/compiler/main/compose_cmd.ark`（マーカー + コマンド構築）
- 実行: `scripts/run/arukellt-selfhost.sh`（`exec wac plug`）
- ゲート: `scripts/check/gate-443-component-composition-phase3.py`

in-tree linker は long-term で非スコープ（option 2）。

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
- Issue #443 — acceptance 完了（Phase 3 wac 委譲）

---

## 関連

- [ADR-008-component-wrapping.md](ADR-008-component-wrapping.md)
- [ADR-007: Targets](../adr/ADR-007-targets.md)
- Issue #443, #476, #124
