# ADR-013: T3 (wasm32-wasi-p2) をプライマリターゲットとして選定する

ステータス: **DECIDED** — T3 (wasm32-wasi-p2)をプライマリターゲットとして選定

決定日: 2026-04-08

---

## 文脈

ADR-007 では T1〜T5 の 5 ターゲットを整理したが、「どれが出荷品質を保証する本線か」は決定されていなかった。
2026-04-08 現在、T3（wasm32-wasi-p2）のすべての fixture テスト（575 件）が通過し、T1（wasm32-wasi-p1）と合わせて 2 本のバックエンドが実際に動作している。
今後の言語意味論の基準・CI 品質ゲートの本体・リリース保証の起点を 1 本に絞るため、この ADR を設ける。

---

## 決定

**T3 (wasm32-wasi-p2) をプライマリターゲット (primary) とする。**

T3 を選ぶ根拠:

1. **言語意味論の基準**: ADR-007 § ターゲット優先順位に「言語意味論の基準: T3」とある。Wasm GC ありの実装が Arukellt の型モデルに最も近い。
2. **fixture coverage**: T3 は 2026-04-08 時点で 157 `t3-run` + 161 `t3-compile` の 318 エントリが manifest に登録され、すべてが pass している。
3. **Component Model**: `--emit component` により WIT ベースの interop が可能。T1 にはない。
4. **将来対応**: T5 (wasm32-wasi-p3) は T3 の後継として設計されており、T3 との継続性が高い。
5. **ユーザー価値**: VS Code 拡張・LSP・タスクランナーすべてが T3 を前提としている。

### ターゲット tier 定義

| Tier | 定義 | 保証 |
|------|------|------|
| **primary** | 出荷品質を保証するターゲット。CI の全品質ゲートを通過。 | バグは即時 issue 化 |
| **supported** | 日常利用は可能だが、primary と同等の保証はない。CI は別途 job で実行。 | ベストエフォート |
| **experimental** | 使えるかもしれないが保証なし。CI は opt-in または不在。 | 保証なし |
| **scaffold** | 構造のみ存在し、実装がない。除外 flag が必要。 | 保証なし |
| **not-started** | コードも構造も存在しない。 | — |

### 各ターゲットの tier 割り当て

| Target | Tier | 根拠 |
|--------|------|------|
| T3: wasm32-wasi-p2 | **primary** | 全 fixture pass、Component Model 対応、言語意味論基準 |
| T1: wasm32-wasi-p1 | **supported** | 全 fixture pass、AtCoder 用途、linear memory の制約あり |
| T2: wasm32-freestanding | **not-started** | コードなし、テストなし |
| T4: native (LLVM) | **scaffold** | ark-llvm crate あり、LLVM 18 必須、テストなし |
| T5: wasm32-wasi-p3 | **not-started** | 仕様策定中、コードなし |

---

## CI への影響

- primary (T3) の CI ゲートが main ブランチへの merge 条件になる。
- supported (T1) の CI は別 job として実行し、失敗は merge を止めない（ただし issue 化する）。
- scaffold / not-started は CI から除外する。
- ADR-007 の「実装優先度: T1 → T3 → T2 → T4 → T5」は歴史的な実装順であり、この ADR の tier 割り当てと矛盾しない。

## current-state.md への影響

`docs/current-state.md` の target 表はこの ADR の tier を反映する。
各 tier が CI 結果と一致しているか、`scripts/run/verify-harness.sh` 経由で確認できるようにする。

---

## 禁止事項

- supported / experimental ターゲットの失敗を理由に primary ターゲットの fix を後回しにしない。
- tier を変更する際は、この ADR を amendment するか、新 ADR を作成すること。
- primary tier を複数にしない（1 本に保つことで CI ゲートを明確にする）。

---

## 関連

- ADR-007: コンパイルターゲット整理（5 ターゲットの定義）
- ADR-002: Wasm GC 採用
- ADR-005: LLVM バックエンドの役割制限（T4 scaffold の根拠）
- `docs/target-contract.md`: 各ターゲットの詳細検証面
- `issues/open/241-define-primary-target-and-tier-others.md`
- `issues/open/242-ci-layer-structure.md`
