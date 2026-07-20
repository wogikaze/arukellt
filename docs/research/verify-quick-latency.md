# verify quick latency — wall-time breakdown (2026-07-21)

ステータス: 調査メモ（決定記録ではない）  
計測環境: WSL2 worktree `wave/gate-speedup` @ `92e11273`  
関連: `docs/research/agent-tooling-latency.md`、`docs/research/selfhost-compile-latency-root-cause.md`、
`verify lane`（同ブランチ）、`#829`（selfhost compile latency）

## 結論（先に）

1. **編集ループの主敵は「verify quick を毎ターン回すこと」自体**である。ゲートを速くしても
   kitchen-sink（~48 bg checks + close-gate + 後段 harness）のままではエージェント並列と相性が悪い。
2. **構造ボトルネックは heavy プールが `workers=1` で直列**なこと。static は×8 まで並列化済みでも、
   壁時間の下限はほぼ `sum(heavy)` になる。
3. **実測ウォーム壁時間 ≈ 37s**（この worktree、cache 有効、`PYTHONUNBUFFERED=1`）。
   冷たい直列合計は ≈ **80s**（bg only）+ close-gate。
4. **「数分ハングしてログが空」**は、パイプ先へのフルバッファ + フォアグラウンド close-gate /
   flock 待ちが主因になりやすい（死んでいない）。
5. いまの `verify quick` は **緑でもない**（本計測 10 fail）。失敗チェックもフル時間を消費する。

## 計測方法

1. `check-false-done-close-gates.py` 単体を `/usr/bin/time`。
2. `verify quick` の `bg_checks` 48 本を **cache 無効・直列**で個別計測。
3. 現行プール（static×8 / heavy×1、check cache 有効）で `verify quick` の壁時間を計測。

Artifact: `.build/selfhost/arukellt-s2-runtime.wasm` を `ARUKELLT_SELFHOST_WASM` に固定。

## 結果

### Foreground

| Step | Wall | Notes |
|---|---:|---|
| false-done close-gate | **6.2 s** | cache hit 多め。cold / lock 競合時は更に長い |
| manifest completeness | ≪1 s | |

close-gate は **bg 並列の前に直列実行**される。ここが伸びると「何も出ない待ち」に見える
（stdout がファイルリダイレクトでブロックバッファされるとき特に）。

### bg_checks 個別（cold, cache off, 直列）

| 集計 | 値 |
|---|---:|
| checks | 48 |
| serial static sum | **34.1 s**（28 本想定） |
| serial heavy sum | **46.1 s**（20 本想定） |
| serial total | **80.2 s** |
| ideal static wall (@8) | ~4.3 s |
| ideal wall (static∥ / heavy 直列) | **~46 s** |

#### TOP（個別）

| Wall | Pool | Check | rc |
|---:|:---:|---|---:|
| 19.7 s | H | quality quick | 1 |
| 6.0 s | H | T3 fixture WASM validation (#686) | 0 |
| 5.2 s | S | ark code quality ratchet | 1 |
| 4.9 s | S | internal link integrity | 1 |
| 4.7 s | H | selfhost LSP lifecycle (#569) | 0 |
| 4.5 s | S | asset naming convention | 0 |
| 3.1 s | H | opt-equivalence (O0 == O1) | 0 |
| ≤1.6 s | * | その他多数 | * |

`quality quick` が heavy 鎖の支配項。中で `quality structure` 等が走り、
`fixture_manifest_count` drift（2702 vs 2703）などで **失敗しても ~20s 消費**する。

### 実 `verify quick` 壁（warm, cache on, 現行プール）

| Metric | Value |
|---|---:|
| Wall | **36.9 s** |
| pools | static=28×8, heavy=20×1 |
| Passed / Failed | 159 / **10** |

失敗の内訳（本計測）:

- docs freshness / docs consistency（manifest count 2702→2703 drift、generated docs stale）
- #798 callee-string / legacy dispatch / core-op validator
- internal link integrity
- ark code quality ratchet
- quality quick（上記 structure drift を含む）
- runtime Wasm debug smoke (#638)
- GC array smoke

## なぜ遅いか（構造）

```text
verify quick
  ├─ close-gate          (serial, selfhost/wasm を含み得る)
  ├─ bg static ×8        (docs/registry — 並列で効く)
  ├─ bg heavy  ×1        (quality quick, T3, LSP, opt-equiv, … — 直列が下限)
  └─ 後段 harness 集計
```

1. **Kitchen sink**: merge/CI 用の広い契約を「quick」と呼んでいる。
2. **Heavy 直列**: flock 回避のため `heavy_workers=1`。ロック不要な heavy
   （opt-equivalence の一部、既に cache された T3 等）まで直列化している。
3. **quality quick の入れ子**: VQ の中で PR quality フル相当が走り、lane 用の
   `quality changed` より重い。
4. **失敗してもフルコスト**: drift / ratchet 失敗が短絡しない。
5. **観測性**: リダイレクト時に進捗がフラッシュされず、「ハング」に見える。
6. **履歴**: 修正前は `ThreadPoolExecutor(max_workers=1)` で **全 bg が直列**
   （≈80s+）。並列化パッチ後も heavy 鎖が残る。

## エージェント体験との関係

| ループ | あるべきゲート | 実測感 |
|---|---|---|
| 編集中 | `verify lane`（~数–十秒） | 意図どおり速い |
| レーン完了 | `verify lane --gate …` | ドメイン次第 |
| merge / CI | `verify quick` | **~37s でも長い**。失敗 10 件で繰り返し再実行すると分単位 |

並列エージェントが **同じ tree で VQ** を回すと `.build/selfhost-runtime.lock` で更に直列化する
（`ARUKELLT_BUILD_DIR` / worktree 局所 `.build` で緩和可能）。

## 改善の優先順位（効果 ÷ 実装コスト）

### P0 — 運用・契約（実装済 2026-07-21）

1. **エージェント既定を `verify lane` に固定**（AGENTS / skill / `verify lane`）。
2. **VQ を merge bot / orchestrator 専用**と明記。
3. **進捗を line-buffer / `PYTHONUNBUFFERED=1` 既定化**（`_configure_verify_quick_stdio` + `[bg] done`）。

### P1 — VQ 自体の短縮（実装済 2026-07-21）

1. **heavy を flock 要/不要に分割**（`heavy_serial`×1 / `heavy_parallel`×2–3）。
2. **`quality quick` を外し `quality changed` に置換**。
3. **fail-fast** `ARUKELLT_VERIFY_QUICK_FAIL_FAST=1`。

### P2 — かつら剥がし（実装済 2026-07-21）

1. **`verify quick` = core**、**`verify quick --extended`** で LSP/component/opt 等を追加。
2. fixture accounting drift（2703 / remainder 1115）と generated docs を同期。

### やらない方がよいこと

- cold stage-3 / fixpoint を VQ に足す（#829 の領域。更に悪化する）。
- 「VQ を速くする」ために acceptance を黙って SKIP 増やす。

## 目標値と達成（2026-07-21 P0–P2 後）

| ゲート | 目標 | 実測（同 worktree, warm） |
|---|---|---|
| `verify lane` | ≤ 15 s | ~9 s（`.ark` 差分なし） |
| `verify quick` コア | ≤ 25 s | **6.5 s, 147/147 pass** |
| `verify quick --extended` | ≤ 40 s | （merge 時に使用） |
| エージェントが VQ を触る頻度 | merge 時のみ | AGENTS / skill で固定 |

## 再現コマンド

```bash
export ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2-runtime.wasm
export PYTHONUNBUFFERED=1
/usr/bin/time -f 'vq_wall=%e rc=%x' python3 scripts/manager.py verify quick
```

個別プロファイルは本調査時に `/tmp/vq-profile.json` へ書き出した
（label / seconds / rc / kind）。
