# ADR-008: Component Model ラッピング戦略

ステータス: **DECIDED** — v2/v3 は wasm-tools に一時委譲、v4/v5 で in-tree 化して依存を除去

決定日: 2026-03-28（2026-07-15 改訂: in-tree 化の方針を明確化）

---

## 文脈

v2 で Component Model 対応を実現するにあたり、core Wasm モジュールを
Component Model バイナリ (.component.wasm) に変換する方法を決定する必要がある。

選択肢:
1. **外部ツール (`wasm-tools component new`)** — Bytecode Alliance 公式実装を subprocess 呼び出し
2. **ツリー内実装** — component binary format を自前で生成

---

## 決定

**最終目標: ツリー内実装により `wasm-tools` への依存を完全に除去する。**

v2/v3 では `wasm-tools component new` を外部 subprocess として使用する（一時的委譲）。
Component Model 仕様が安定した v4/v5 で in-tree 実装に移行し、外部依存をなくす。

理由:
- Component binary format は複雑 (component sections, canonical options, type interning)
- `wasm-tools` は Bytecode Alliance が維持する参照実装であり、仕様追従が保証される
- ツリー内実装は v2 のスコープでは労力対効果が低い
- **ただし Arukellt はセルフホスト言語であり、外部ツールへの恒久依存は設計上望ましくない**
- Component Model 仕様が Wasm 3.0 で安定化した現在、in-tree 化の目処は立っている

### 移行タイムライン

| フェーズ | wasm-tools 依存 | 備考 |
|----------|----------------|------|
| v2/v3（現在） | ✅ subprocess 呼出 | 一時的委譲 |
| v4/v5 | ❌ in-tree 実装 | component binary format を自前生成 |

---

## 代替案

### ツリー内バイナリ生成 (最終目標 — v4/v5 で採用)

- 利点: 外部依存なし、ビルド再現性向上、セルフホスト完結
- 欠点: 仕様追従コスト大 — ただし Wasm 3.0 で仕様安定化済み
- v4/v5 で実装し、`wasm-tools` への依存を完全に除去する

---

## フォールバック動作

`wasm-tools` が PATH にない場合:
- `--emit component` はエラーメッセージを出力: "error: wasm-tools not found. Install with: cargo install wasm-tools"
- `--emit core-wasm` と `--emit wit` は影響を受けない
- `--emit all` は core + WIT を出力し、component のみ warning で skip

---

## Canonical ABI メモリ予算

- Linear memory 1 page (64KB) のうち offset 256–65535 を canonical ABI スクラッチ領域として使用
- Per-call bump allocator (呼び出し毎にリセット)
- 上限: 65280 bytes/call — 大きな文字列・リストはこの制約に引っかかる
- v3 で `memory.grow` による動的拡張を検討

---

## 影響

- `src/compiler/component.ark` — wrapping 実装（旧 Rust プロトタイプは #562 で削除）
- `crates/ark-driver/src/session.rs` — `compile_component()` メソッド追加
- `crates/arukellt/src/commands.rs` — `--emit component` CLI ルーティング
- `src/compiler/driver.ark` — `EmitCapability::Component` 追加
