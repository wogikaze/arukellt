# ADR-013: `wasm32-gc` をプライマリターゲットとする

ステータス: **ACCEPTED** — `wasm32-gc` を唯一の primary とし、既定 host は WASI P2

決定日: 2026-04-08  
改訂日: 2026-07-11 — ADR-007 の canonical 名（`wasm32-gc`）に追従

---

## 文脈

ADR-007 は canonical ターゲットを `wasm32` / `wasm32-gc` / `native-*` に再構成した。
出荷品質・CI ゲート・言語意味論の基準を 1 本に絞る必要がある。

決定時点の実装・検証の中心は WASI P2 付き GC 経路（当時の文書名 `wasm32-wasi-p2`）だった。
これは canonical 名では `wasm32-gc`（既定 host profile = WASI P2）に相当する。

---

## 決定

1. **`wasm32-gc` を唯一の primary target とする。**
2. **既定の host profile は WASI P2 とする。** WASI P3 は同一言語ターゲット上の
   host profile であり、別の primary target とはしない。
3. **`wasm32`（linear / AtCoder 向け）は supported** とし、primary と同等の CI ゲートは課さない。
4. **native 系統は scaffold** とし、ABI・意味論は ADR-045 再開まで固定しない。

### tier 語彙（定義のみ）

| Tier | 定義 |
|------|------|
| **primary** | 出荷品質を保証。CI の全品質ゲート対象。 |
| **supported** | 日常利用可。CI は別 job、失敗は merge を止めない。 |
| **scaffold** | 構造・実験のみ。広域保証対象外。 |
| **not-started** | 未着手。 |

各ターゲットの**現行** tier・fixture 件数・CI 状態は `docs/current-state.md` を正本とする。
本 ADR は tier 語彙と primary 選定のみを固定する。

---

## 帰結

- primary（`wasm32-gc`）の失敗を、supported / scaffold の都合で後回しにしない。
- primary tier を複数にしない。
- 旧名 `wasm32-wasi-p2` は `wasm32-gc` の歴史的別名として文書に残してよい。

## 関連

- [ADR-007](ADR-007-targets.md)
- [ADR-002](ADR-002-memory-model.md)
- [ADR-045](ADR-045-llvm-scope-withdrawn.md)
- `issues/done/241-define-primary-target-and-tier-others.md`
