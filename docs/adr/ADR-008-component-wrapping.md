# ADR-008: Component Model ラッピング戦略

ステータス: **DECIDED** — v2ではwasm-tools component newを外部subprocessとして使用

決定日: 2026-03-28

---

## 文脈

v2 で Component Model 対応を実現するにあたり、core Wasm モジュールを
Component Model バイナリ (.component.wasm) に変換する方法を決定する必要がある。

選択肢:
1. **外部ツール (`wasm-tools component new`)** — Bytecode Alliance 公式実装を subprocess 呼び出し
2. **ツリー内実装** — component binary format を自前で生成

---

## 決定

**v2 では `wasm-tools component new` を外部 subprocess として使用する。**

理由:
- Component binary format は複雑 (component sections, canonical options, type interning)
- `wasm-tools` は Bytecode Alliance が維持する参照実装であり、仕様追従が保証される
- ツリー内実装は v2 のスコープでは労力対効果が低い
- Component Model 仕様が安定した v4/v5 で in-tree 移行を検討する

---

## 代替案

### ツリー内バイナリ生成 (不採用)

- 利点: 外部依存なし、ビルド再現性向上
- 欠点: 仕様追従コスト大、Component Model 仕様がまだ進化中
- 将来: v4 最適化フェーズで再評価。バイナリサイズ最適化の文脈で有利になる可能性

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

- `crates/ark-wasm/src/component/wrap.rs` — wrapping 実装
- `crates/ark-driver/src/session.rs` — `compile_component()` メソッド追加
- `crates/arukellt/src/commands.rs` — `--emit component` CLI ルーティング
- `crates/ark-target/src/plan.rs` — `EmitCapability::Component` 追加
