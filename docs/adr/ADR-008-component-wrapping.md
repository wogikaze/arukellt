# ADR-008: Component Model ラッピング戦略

ステータス: **ACCEPTED** — in-tree 実装により `wasm-tools` への依存を除去（前倒し完了）

決定日: 2026-03-28（2026-07-10 改訂: in-tree 化前倒し完了を反映）

---

## 文脈

v2 で Component Model 対応を実現するにあたり、core Wasm モジュールを
Component Model バイナリ (.component.wasm) に変換する方法を決定する必要がある。

選択肢:
1. **外部ツール (`wasm-tools component new`)** — Bytecode Alliance 公式実装を subprocess 呼び出し
2. **ツリー内実装** — component binary format を自前で生成

---

## 決定

**ツリー内実装により `wasm-tools` への依存を除去する。**

当初の計画では v2/v3 は `wasm-tools component new` を外部 subprocess として使用し（一時的委譲）、
v4/v5 で in-tree 実装に移行する予定だった。しかし、Component Model 仕様が Wasm 3.0 で
安定化したことと、セルフホスト完結の優先度が高まったことから、**in-tree 化を前倒しして完了**した。

理由:
- Component binary format は複雑 (component sections, canonical options, type interning)
- **Arukellt はセルフホスト言語であり、外部ツールへの恒久依存は設計上望ましくない**
- Component Model 仕様が Wasm 3.0 で安定化したため、in-tree 化のリスクが低下した
- in-tree 実装により、ビルド再現性とセルフホスト完結が同時に達成された

### 移行タイムライン（実績）

| フェーズ | wasm-tools 依存 | 備考 |
|----------|----------------|------|
| v2/v3（計画） | ✅ subprocess 呼出 | 当初の計画（一時的委譲） |
| v3（実績） | ❌ in-tree 実装 | **前倒し完了** — `src/compiler/component/` 配下に ComponentWriter を実装 |

### 現在の wasm-tools 使用箇所

in-tree 化により、component binary 生成に `wasm-tools` は不要。残存する使用箇所:

- `scripts/selfhost/p2_component_wrap.py` — WAT → Wasm コンパイルのフォールバック（bridge adapter 生成時のみ）
- `src/compiler/main/component_cmd.ark` — `arukellt component wit` / `arukellt component validate` サブコマンドが `wasm-tools` を呼出（ユーティリティ機能、component 生成には非必須）

---

## 代替案

### ツリー内バイナリ生成 (採用・完了)

- 利点: 外部依存なし、ビルド再現性向上、セルフホスト完結
- 欠点: 仕様追従コスト大 — ただし Wasm 3.0 で仕様安定化済み
- **v3 で前倒し実装し、`wasm-tools` への依存を component binary 生成から除去**

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

- `src/compiler/component/` — in-tree wrapping 実装（ComponentWriter、adapter 生成、型変換）
  - `writer_core.ark`, `component_base.ark`, `emit_color.ark`, `emit_string_general.ark`,
    `emit_list.ark`, `emit_record_ops.ark`, `adapter_body.ark`, `adapter_memory.ark` 等
- `src/compiler/main/component_cmd.ark` — `arukellt component` サブコマンド（wit/validate は wasm-tools 経由）
- `src/compiler/driver.ark` — `EmitCapability::Component` 追加
- `scripts/selfhost/p2_component_wrap.py` — WAT bridge adapter コンパイル用（wasm-tools 使用）
