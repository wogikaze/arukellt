# コンパイルパイプライン

## パイプライン全体像（v0）

```
ソースファイル (.ark)
    │
    ▼
[1] Lexer / Tokenizer
    トークン列
    │
    ▼
[2] Parser
    AST (Abstract Syntax Tree)
    │
    ▼
[3] Name Resolution
    スコープ解決・インポート解決
    シンボルテーブル付き AST
    │
    ▼
[4] Type Checker
    双方向型推論・exhaustive match チェック
    型注釈付き AST（TAST）
    │
    ▼
[5] MIR Lowering
    中間表現（MIR: Mid-level IR）
    制御フローグラフ（CFG）
    │
    ▼
[6a] Wasm Emitter          [6b] LLVM IR Emitter
    .wasm / .wat               .ll
    │
    ▼
[7] LTO / ICF（オプション）
    最適化・デッドコード除去・関数マージ
    │
    ▼
最終出力（.wasm / .cwasm / native binary）
```

---

## 各フェーズの概要

### [1] Lexer

- ソースを UTF-8 として読む
- キーワード、識別子、リテラル、演算子をトークンに分割
- コメントを除去
- 位置情報（行・列）を保持する

### [2] Parser

- トークン列を AST に変換
- 再帰下降パーサ
- エラーリカバリ: 1つのエラーで止まらず、複数のエラーを報告する
- AST ノードに span（ソース上の位置）を持たせる

### [3] Name Resolution

- モジュール境界を越えたシンボルの解決
- import のグラフを構築（循環 import を検出）
- 変数のスコープを確定
- ジェネリクスの型パラメータのスコープ解決

### [4] Type Checker

- 双方向型推論（synthesis + checking）
- ローカル変数の型推論
- enum の exhaustive match チェック
- 型エラーのレポート

**LLM フレンドリなエラーメッセージの原則:**
- エラー位置はなるべく宣言に近い場所を指す
- 「何が期待されていたか」と「何が実際に来たか」を両方示す
- 型変数を生で表示しない（`T` ではなく `推論結果の型` を表示）
- エラーの文面は1エラー1メッセージ。「かもしれない」の連射はしない

### [5] MIR Lowering

- AST → 制御フローグラフ（基本ブロック + 分岐）
- match を条件分岐の連鎖に変換
- クロージャを明示的なクロージャオブジェクト + 環境構造体に展開（ADR-002 依存）
- generic function の monomorphization（ADR-003 依存）

MIR の形式:
- SSA 形式（Static Single Assignment）を採用するか検討中
- Wasm の構造化制御フロー（block/loop/if）への変換を考慮した設計にする

### [6a] Wasm Emitter（主）

- MIR → Wasm バイナリ
- 型セクション・関数セクション・コードセクション・データセクションの生成
- WASI import の追加
- マルチバリュー（multi-value）を tuple / Result の lowering に使う

### [6b] LLVM IR Emitter（補助）

- MIR → LLVM IR テキスト形式（.ll）
- WASM 意味論と同じ動作を保証する
- 未最適化でよい（ADR-005）

### [7] LTO / ICF

- monomorphization による重複コードを統合
- デッドコード除去
- wasmtime の最適化パスとの連携

---

## エラーリカバリ方針

パーサ・型チェッカーともに「エラーを1つ見つけたら止まる」にしない。

方針:
- パースエラー: パニックサイトを挿入して続行
- 型エラー: エラー型（`ErrorType`）を注入して続行
- 出力: ユーザーに複数のエラーをまとめて報告する

LLM が生成するコードのデバッグでは「1エラー修正 → 再コンパイル → 次のエラー」のサイクルを速くすることが重要。複数エラーの一括報告はこれに貢献する。

---

## インクリメンタルコンパイル（将来）

v0 では全量コンパイル。将来の粒度候補:
- モジュール単位（ファイル単位）の再コンパイル
- 型チェック結果のキャッシュ

---

## モジュールシステム（詳細）

```
// ファイル構造
src/
  main.ark      // エントリポイント
  math.ark      // モジュール math
  util/
    string.ark  // モジュール util.string

// インポート
import math
import util.string as ustr

// 公開
pub fn my_function() -> i32 { ... }   // pub をつけないと外から見えない
```

- 1ファイル = 1モジュール
- モジュール名はファイルパスから自動決定
- 循環インポートはコンパイルエラー
- `pub` がないシンボルはモジュール内部のみ

---

## shebang サポート

```
#!/usr/bin/env arukellt run
fn main() { ... }
```

`arukellt run hello.ark` での直接実行をサポートする。内部で wasmtime を呼ぶ。
