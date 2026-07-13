# コード規約

エージェントおよび実装者向けの現行コーディング規約。
入口は `AGENTS.md`。詳細の正本はこの文書とする。

## 書式と命名

- `.ark` ファイルの書式は selfhost formatter を正本とする。手作業で独自の整形規則を追加せず、変更したファイルには `arukellt fmt` または `arukellt fmt --check` を適用する。
- インデントは 4 スペースとし、タブ、行末空白、連続する複数の空行を残さない。ファイル末尾は改行 1 つとする。
- 関数、メソッド、変数、フィールド、モジュール、ファイル名は `snake_case`、型、trait、enum、enum variant は `PascalCase` を使う。
- 略語を含む名前も単語として扱う。意味の異なる概念を `id`、`name`、`type` などの曖昧な名前だけで区別しない。
- コメントはコードの動作ではなく、設計理由、不変条件、非自明な制約を説明する。
- `TODO`、一時的な互換処理、skip には追跡 issue と削除条件を付ける。所有者や終了条件のない恒久的な暫定処理を追加しない。

## API設計

- ユーザー可達 API は trait、method、associated function を正規形とする。公開 free function、prelude wrapper、型別のモノモルフィック helper を新設しない。
- 型に属する操作は method または associated function として実装する。名前空間だけを目的とした free function で回避しない。
- コンパイラ内部の module-private helper は許容するが、公開 API の代替経路や恒久的な互換 wrapper として使用しない。
- deprecated API を追加・維持するときは、置換先、追跡 issue、削除条件を明記する。新規コードから deprecated API を呼ばない。

## コンパイラの層

- 修正は問題を最初に所有する層で行う。parser の問題を resolver で補正したり、型情報の欠落を emitter で推測したりしない。
- parser は構文、resolver は記号と参照先、typechecker は意味型、CoreHIR は解決済みの意味情報、MIR は制御・値・ABI 情報、backend は機械的な出力変換を所有する。
- resolver 以降では、型、関数、trait、impl、呼び出し先を名前文字列から再推論しない。`TypeId`、`FunctionId`、registry entry などの canonical identity を引き回す。
- 型名、関数名、mangled name などの文字列は診断、表示、互換入力に限定し、semantic dispatch の正本にしない。
- MIR または backend で必要な型・signature・ABI 情報が欠けている場合、既定値、名前推測、先頭候補への fallback で処理を続けない。欠落を発生させた上流を修正する。
- 複数の解決候補が残る場合、宣言順や collection の iteration 順で最初の候補を選ばない。明示的な ambiguity として報告する。
- ユーザー向け診断に必要な source span と phase 情報を変換途中で失わない。新しい IR ノードや lowering 経路には、必要な source information を伝播させる。

## エラー処理

- 回復可能な失敗は `Result`、ユーザーコードの誤りは構造化診断、コンパイラ不変条件の破壊は ICE として扱い、相互に代用しない。
- ユーザー入力から到達可能な経路で、panic、trap、未処理の unwrap、未実装分岐を発生させない。
- ICE はユーザー入力の検証手段として使わない。壊れた入力で発生する問題は parse、resolve、typecheck、target、backend validation の適切な診断へ変換する。
- エラーを空値、unknown、成功結果へ無条件に変換しない。回復を行う場合は、後続診断のための明示的な error sentinel として扱い、正常な意味情報と混同しない。
- 新しい診断には canonical code、severity、phase origin、primary span を与える。型不一致では可能な限り expected と actual を含める。

## 決定性と状態

- 同じ入力から生成される IR、Wasm、診断、snapshot の順序を決定的にする。
- 出力順を unordered collection の iteration 順に依存させない。source order、insertion order、または明示的な stable sort を使う。
- timestamp、乱数、UUID、環境依存の絶対パスなどを生成物へ暗黙に埋め込まない。
- 共有可変 global state を新設しない。必要な状態は明示的な context、registry、table として所有者を定めて渡す。
- キャッシュは意味上の正本にしない。キャッシュを削除しても結果が変わらない構造にする。

## テスト

- 挙動変更には、その変更がなければ失敗する最小の回帰テストを追加する。
- 純粋な helper や局所変換は in-file test、言語機能・診断・pipeline は fixture、host・component・LSP・emitter の副作用経路は対応する integration または contract test で検証する。
- 各テストには主要な責任カテゴリを 1 つだけ割り当てる。複数の無関係な挙動を 1 つの fixture へ詰め込まない。
- 正常系だけでなく、曖昧性、不正入力、unsupported target、欠落した型情報などの失敗経路を検証する。
- 診断テストでは canonical code、phase、重要な span や context を確認する。意図的な snapshot test 以外では、無関係な文面全体への過度に脆い一致を避ける。
- snapshot や baseline をテスト通過のためだけに更新しない。差分の意味を確認し、挙動変更または性能変更として説明できる場合だけ更新する。

## 互換性と現行名

- 新しいコード、テスト、診断、ドキュメントでは canonical target 名だけを使用する。`wasm32-wasi-p1`、`wasm32-wasi-p2`、T1–T5 などの旧名を新しい内部 identity として追加しない。
- legacy alias は入力境界で canonical identity へ変換し、それ以降の pipeline へ持ち込まない。
- `PROPOSED` または `DRAFT` の ADR・RFC を採択済み仕様として実装しない。実験実装には feature boundary と追跡 issue を設ける。
- `docs/history/`、退役済み経路、古い example を新規コードの模範としてコピーしない。現行 fixture、`docs/current-state.md`、構造化 SSOT、`ACCEPTED` ADR を優先する。
