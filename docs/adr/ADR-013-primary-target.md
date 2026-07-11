# ADR-013: wasm32-wasi-p2 をプライマリターゲットとして選定する

ステータス: **ACCEPTED** — `wasm32-wasi-p2`（旧称 T3）をプライマリターゲットとして選定

決定日: 2026-04-08

---

## 文脈

ADR-007 は当初 T1〜T5 の 5 ターゲット表記で整理していたが、現行の正本命名は
ターゲット名ベース（`wasm32` / `wasm32-gc` / `native` 系統、および具体プロファイル
`wasm32-wasi-p1` / `wasm32-wasi-p2` 等）である（ADR-007 2026-07 改定）。
本 ADR の決定時点では「どれが出荷品質を保証する本線か」が未確定だった。
2026-04-08 現在、`wasm32-wasi-p2`（当時の文書では T3）のすべての fixture テスト（575 件）が通過し、
`wasm32-wasi-p1`（当時 T1）と合わせて 2 本のバックエンドが実際に動作している。
今後の言語意味論の基準・CI 品質ゲートの本体・リリース保証の起点を 1 本に絞るため、この ADR を設ける。

---

## 決定

**`wasm32-wasi-p2` をプライマリターゲット (primary) とする。**

（歴史的別名: T3。新規文書ではターゲット名を使う。）

選定根拠:

1. **言語意味論の基準**: ADR-007 の優先順位では Wasm GC ありの実装が Arukellt の型モデルに最も近い。
2. **fixture coverage**: 2026-04-08 時点で 157 `t3-run` + 161 `t3-compile` の 318 エントリが manifest に登録され、すべてが pass している。
3. **Component Model**: `--emit component` により WIT ベースの interop が可能。`wasm32-wasi-p1` にはない。
4. **将来対応**: `wasm32-wasi-p3`（旧称 T5）は p2 の後継として設計されており、継続性が高い。
5. **ユーザー価値**: VS Code 拡張・LSP・タスクランナーすべてが `wasm32-wasi-p2` を前提としている。

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
| `wasm32-wasi-p2` (旧 T3) | **primary** | 全 fixture pass、Component Model 対応、言語意味論基準 |
| `wasm32-wasi-p1` (旧 T1) | **supported** | 全 fixture pass、AtCoder 用途、linear memory の制約あり |
| `wasm32-freestanding` (旧 T2) | **not-started** | コードなし、テストなし |
| `native` / LLVM (旧 T4) | **scaffold** | ark-llvm crate あり、LLVM 18 必須、テストなし |
| `wasm32-wasi-p3` (旧 T5) | **not-started** | 仕様策定中、コードなし |

---

## CI への影響

- primary (`wasm32-wasi-p2`) の CI ゲートが main ブランチへの merge 条件になる。
- supported (`wasm32-wasi-p1`) の CI は別 job として実行し、失敗は merge を止めない（ただし issue 化する）。
- scaffold / not-started は CI から除外する。
- ADR-007 の旧「実装優先度: T1 → T3 → T2 → T4 → T5」は歴史的な実装順の記録であり、この ADR の tier 割り当てと矛盾しない。

---

## 禁止事項

- supported / experimental ターゲットの失敗を理由に primary ターゲットの fix を後回しにしない。
- tier を変更する際は、この ADR を amendment するか、新 ADR を作成すること。
- primary tier を複数にしない（1 本に保つことで CI ゲートを明確にする）。

---

## 関連

- ADR-007: コンパイルターゲット整理（現行はターゲット名ベース）
- ADR-002: Wasm GC 採用
- ADR-005: LLVM バックエンドの役割制限（native scaffold の根拠）
- [ADR-007: Targets](ADR-007-targets.md): 各ターゲットの詳細検証面
- `issues/done/241-define-primary-target-and-tier-others.md`
- `issues/done/242-ci-layer-structure.md`
