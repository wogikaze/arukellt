# AGENTS.md

## プロジェクトの正本

- 現行のユーザー可視挙動は `docs/current-state.md` と `docs/data/*.toml` を正とする。
- 設計判断は `docs/adr/` を参照する。拘束力があるのは `ACCEPTED` のみで、`PROPOSED` は未採択、`SUPERSEDED` は履歴である。
- 詳細仕様は `docs/rfcs/`、実装計画と一時制限は `docs/plans/`、調査は `docs/research/` に置く。ADR に進捗や一時的な実装上限を書かない。
- 生成物は `docs/directory-ownership.md` に従い、生成元を変更して再生成する。生成済み Markdown を直接直さない。
- 検証コマンド名は `docs/data/verification-commands.toml` を正とする。
- 実装の読みやすさ・分割・書式の品質規約は本ファイルの「コード品質規約」を正とする。
- コンパイラ層・API・エラー・決定性など設計寄りの規約は `docs/process/coding-conventions.md` を正とする。

## 現行アーキテクチャの制約

- `src/compiler/` のセルフホスト実装をコンパイラ・LSP の正本として扱う。退役済み Rust-era 経路や `crates/**` を既定の変更先・検証前提にしない。
- 公開ターゲットは `wasm32-gc`（primary）、`wasm32`（supported）、`native-cpp` / `native-llvm`（scaffold）。WASI P1/P2/P3 は host profile でありターゲット名ではない。
- `wasm32-gc` が既定でも、実装状態は partial である。ADR の理想形を現行実装済みと誤記しない。
- 公開 API は trait / method / associated function を正規形とする。ユーザー可達 free function を新設・温存しない。例外は非公開 intrinsic のみ（ADR-044、ADR-046）。
- 安定性変更は ADR-014、ユーザー入力から到達するエラー処理は ADR-015、セルフホスト検証は ADR-029 に従う。

## 必須ワークフロー

- 言語意味論、公開 API、ABI、ターゲット、コンパイラ段階、stdlib 移行方針を変える前に `$implementation-strategy` を使う。
- 長期的な設計判断を新設・置換するときは `$architecture-decision` を使う。
- docs、生成元、例、current-state の主張に影響する変更では `$docs-sync` を使う。
- benchmark、baseline、perf threshold を変更するときは `$benchmark-change` を使う。
- コード、テスト、例、ビルド、検証挙動を変更した後は `$code-change-verification` を使う。
- issue を `issues/open/` から `issues/done/` へ移す前は `$issue-close-review` を使う。

## 実装規律

- 依頼の目的、対象、制約、完了条件を先に確定し、必要最小限の差分にする。旧スキルの `PRIMARY_PATHS` 形式は必須ではないが、issue に指定があれば従う。
- コード変更は「コード品質規約」と `docs/process/coding-conventions.md` に従う。変更した `.ark` には `arukellt fmt` または `arukellt fmt --check` を適用する。
- 既存 ADR と衝突したら、コードで既成事実化せず設計判断を解決する。
- 仕様変更には最小の回帰試験を追加する。テスト不能な完了主張をしない。
- 無関係なリファクタ、生成物の手編集、baseline による回帰隠し、SKIP の無根拠追加をしない。
- コマンドを実行できない、または失敗した場合は、その事実と未確認範囲を明記する。成功扱いにしない。
- 「関数は N 行以下」「ファイルは N 行以下」のような単純な長さ上限を品質指標にしない。Ark はすでに細かく分割されており、その種の規則は wrapper と小ファイルを増やす。

### 設計判断の順序（ADR-048）

1. 現在必要な振る舞いと契約を特定する。
2. 最も直接的で単純な実装を選ぶ（KISS）。
3. 未確定の将来要求を実装していないか確認する（YAGNI）。
4. データと責務の owner が一意か確認する。
5. 重複が同じ知識か、偶然似ているコードかを区別する。
6. 同じ知識にだけ DRY を適用する。
7. 変更理由が異なる責務が混ざる場合だけ局所的に SOLID を適用する。
8. 二つ目の実例がない extension point や interface は原則作らない。
9. コードで表せない制約と判断だけをコメントまたは ADR に残す。

「SOLID 違反」「DRY ではない」だけの抽象的なレビュー指摘をしない。具体的な変更圧力、同期漏れ、責務混在、依存問題を示す。

## コード品質規約

変更後のコードは、動作するだけでなく、次に読む人が局所的に理解・修正できる状態にする。既存コードに読みにくいパターンがあっても、それを新しいコードの前例として扱わない。

### 書式

- `.ark` ファイルはプロジェクトの formatter を通し、formatter 適用後の差分を確認する。
- インデントはスペース 4 個とする。タブ、タブとスペースの混在、桁合わせのための大量の空白を追加しない。
- `verify quick` は `scripts/check/check-ark-code-quality.py` でタブ禁止・極端インデント禁止・200 文字超行の件数 ratchet と、高信頼な未正当化 pure forwarder / wrapper-only file の新規混入を検査する（正本は `docs/data/ark-code-quality-baseline.toml`）。
- pre-commit は staged `.ark` に `fmt --check` のあと lint を走らせる。`src/compiler/` / `std/` は `lint --local`（parse＋AST ローカル規則）、それ以外はフル lint。W0011 は基準版からの件数増加を禁止する。通常の warning は exit 0、ratchet 増加 / `--deny` / エラーのみ失敗。`verify quick` に `scripts/check/check-ark-lint-smoke.py` がある。
- 1 行は原則 120 文字以内にする。長い関数宣言、呼び出し、record literal、条件式は意味のまとまりごとに複数行へ分ける。
- 複数フィールドを持つ record literal は、原則として 1 フィールド 1 行で記述する。
- CSS、HTML、JavaScript、fixture などの長い文字列を minify した 1 行として埋め込まない。意味のある断片へ分けるか、専用の resource または template として管理する。
- 空行は論理的なまとまりの区切りにだけ使う。連続する複数の空行や、`use` 宣言間の不規則な空行を残さない。
- formatter が異常なインデントや構文を生成した場合、その出力を受け入れず、formatter または入力コードを修正する。

### 命名

- 型、struct、enum、trait は `PascalCase`、通常の関数、変数、フィールド、ファイル、モジュールは `snake_case` とする。
- constructor や型の擬似 associated function として既存の `Type_name` 形式が必要な場合を除き、関数名の途中に大文字を混在させない。
- 真偽値を返す関数と変数は、可能な限り `is_`、`has_`、`can_`、`should_`、`needs_` で始める。
- `try_*` が `bool` を返す場合、`true` は「処理した」、`false` は「対象外」を一貫して意味する。
- `emit_*` の `bool` 戻り値に、「処理した」「stack に値がある」「成功した」など複数の意味を持たせない。追加情報が必要なら、意味を表す名前の関数または result record を使う。
- `i`、`j` は短い単純な走査、`ctx`、`mir`、`wasm`、`idx`、`vt` は対応領域内に限って使用できる。それ以外の略語や一文字名を新設しない。
- `data`、`value`、`info`、`item`、`result` のような広すぎる名前は、短い局所変数以外では避ける。
- issue 番号を関数名やファイル名の主要な意味にしない。テストや self-check は検証する振る舞いで命名し、issue 番号は補助情報として残す。

### 関数

- 関数は行数ではなく、1 つの判断または 1 つの処理単位を担当させる。
- 短くすることだけを目的に、1 式しかない private wrapper や、同じ引数をそのまま転送する helper を追加しない。
- wrapper は、公開 API の固定、型変換、既定値の付与、不変条件の検査など、呼び出し元に見える責任がある場合だけ作る。
- 既存関数を別名で公開するだけの facade を複数作らない。互換性のために必要な場合は、正規の入口と削除条件を明示する。
- 関連する数行の処理を別ファイルへ切り出すだけの分割をしない。単独で名前を付ける価値がなく、他から再利用されない helper は利用箇所の近くに置く。
- 5 個を超える引数を持つ関数を追加する場合、引数が独立した入力なのか、同じ処理文脈を構成する値なのかを確認する。同じ呼び出し経路をまとめて移動する値は、意味のある request、options、state record へまとめる。
- context record を、無関係な値をすべて入れる汎用的な袋として拡張し続けない。
- 未使用引数は削除する。interface 上必要なら、名前を `_` または `_name` 形式にして意図的に未使用であることを示す。
- `let _nop = 0` のような dummy statement を置かない。未実装、no-op、passthrough のいずれなのかをコード上で明確にする。
- 単に受け取った値を返すだけの関数や、常に同じ値を返す関数は、明確な抽象化境界を形成していない限り削除する。

### 制御フロー

- 正常経路を左端に保つ。エラー、対象外、特殊ケースは早期 `return` または早期 `continue` で処理する。
- `else` の中にさらに `if` を重ねる形を繰り返さない。特に option 解析や dispatch では、1 段ずつ処理を終了させて平坦にする。
- 3 段を超える入れ子を追加する場合、条件の反転、早期 return、named predicate、局所 helper によって平坦化できないか確認する。
- 同じ変数に対する多数の `if` を並べる場合、複数条件が同時に成立してよいのかを明確にする。排他的なら `else if`、`match`、または明示的な dispatch を使う。
- 数値文字コードを並べて文字列を判定しない。通常の文字列比較または名前付きの parser helper を使う。
- 複雑な条件式は、条件を説明する named predicate へ分ける。長い式を改行しただけで可読性を改善したことにしない。

### 重複と dispatch

- 同じ文字列 alias 集合、型名集合、opcode 集合を複数箇所へ複製しない。
- 4 個以上の `a == x || a == y || ...` を新設する場合、`is_*` predicate、alias 判定 helper、またはデータとして定義できないか検討する。
- dispatch 関数では、各分岐を「条件判定」「処理」「結果の格納」の同じ形に揃える。
- 同じ数個の引数を多数の dispatch helper へ転送し続ける場合、呼び出し情報を表す名前付き record へまとめる。
- helper 化によって元の処理より読む場所が増えるだけなら、重複が少量でも局所性を優先する。
- 数行の重複を消すために、引数が多く分岐だらけの汎用 helper を作らない。

### コメント

- 公開面は、`std/manifest.toml` 登録 API（A）、`src/compiler/*.ark` の安定 subsystem boundary（B）、module 可視性のための内部 `pub`（C）へ分類する。C へ一律に doc comment を要求しない。
- `python3 scripts/check/check-comment-policy.py` は structured TODO/FIXME、issue-only marker、明確な commented-out code、A/B documentation contract、doc comment attachment を検査する。

- コメントは「何をしているか」ではなく、「なぜこの形が必要か」「どの不変条件を守るか」「直感的でない制約は何か」を説明する。
- 関数名を言い換えるだけの `Handler for:` や、ファイル名から分かる `Arukellt Selfhost - ...` を機械的に追加しない。
- `i++`、`Result on stack`、`Save value` など、直後のコードを読むだけで分かる実況コメントを追加しない。
- issue 番号だけを根拠にしたコメントを残さない。コード単体で理解できる理由を書き、必要なら末尾に issue 番号を添える。
- 修正履歴、過去の不具合の症状、デバッグ時の経緯を長期間コードコメントとして蓄積しない。現在も必要な制約だけを残す。
- 一時処理には追跡 issue だけでなく、何が解決したら削除できるかを記載する。

### データと constructor

- 多数の bool フィールドを持つ record は、呼び出し側で位置引数を並べる constructor を作らない。名前付き setter、builder、分類された sub-record、または明示的な field 初期化を使う。
- 10 個以上のフィールドを持つ record を 1 行で初期化しない。
- boolean field が増え続けている場合、互いに排他的な状態を複数 bool で表していないか確認する。
- magic number、文字コード、Wasm opcode、scratch slot 番号には名前を付ける。仕様上の数値であっても、利用箇所に裸の値を反復しない。
- 同じ初期値を多数の constructor で繰り返す場合、明示的な default constructor または初期化 helper へ集約する。

### ファイル分割

- ファイルは「1 関数 1 ファイル」ではなく、同じ概念を変更するときに一緒に読む関数群を単位とする。
- 数行の wrapper だけを置くファイルを新設しない。
- facade、compatibility alias、module re-export は、プロジェクトの外部境界または移行境界に限定する。
- 空ファイルや、将来のためだけに作られた placeholder file を残さない。
- 同じ公開関数を持つ別名ファイルを追加しない。
- 1 つの変更で多数の小ファイルを往復しなければ処理を理解できない場合、関連コードの併置を優先する。

### テストコード

- テスト名は issue 番号ではなく、入力条件と期待する振る舞いを表す。
- 1 つの不具合に対する複数の self-check ファイルを細分化しすぎない。同じ機能領域の準備処理と assertion は可能な限り同じ場所へ置く。
- テスト用 fixture 生成コードも通常コードと同じ書式・命名・重複規則に従う。
- regression test には、何が壊れていたかではなく、今後保証する振る舞いを残す。
- snapshot や期待値を巨大な 1 行文字列として埋め込まない。

### レビュー基準

レビューでは、関数の行数やファイル数を単独の品質指標にしない。次の問いで判断する。

- 関数名と引数だけで責任を予測できるか。
- 処理を理解するために不要な wrapper やファイルを往復しないか。
- 条件式の意味を左から最後まで追わなくても理解できるか。
- 引数の順序を間違えても型検査で検出できない構造になっていないか。
- コメントを読まなくても通常経路を追えるか。
- コメントはコードから分からない理由を説明しているか。
- 新しい抽象化は、重複だけでなく認知負荷も減らしているか。
- 変更によってコード量は減っても、理解に必要な概念や移動回数が増えていないか。

比較的よい基準: `analysis/doc_scan.ark`、`fmt/range.ark` のように短い名前付き処理を順に追えるコード。
明確に避けるパターン: `main/args_parse.ark` のような深い分岐、薄い転送 facade、多数の bool 引数 constructor、壊れた巨大インデントや minify 埋め込み。

## 基本コマンド

- 書式: `python3 scripts/manager.py fmt` / `python3 scripts/manager.py fmt --check`
- lint: `python3 scripts/manager.py lint`
- 品質 quick: `python3 scripts/manager.py quality quick`
- 構造契約: `python3 scripts/manager.py quality structure`
- advisory metrics: `python3 scripts/manager.py quality report`
- 高速ゲート: `python3 scripts/manager.py verify quick`
- fixture: `python3 scripts/manager.py verify fixtures`
- **コンパイラ wasm 更新（emitter 編集後）**: `python3 scripts/manager.py selfhost build-compiler`（stage-2 のみ、**~45–50s が下限**。別名 `build-s2` / `rebuild-s2`）
- **fixpoint ゲート（ADR-029）**: `python3 scripts/manager.py selfhost fixpoint`（s2==s3 確認。日常の s2 再ビルドには使わない）
- docs 再生成: `python3 scripts/manager.py docs regenerate`
- docs 検査: `python3 scripts/manager.py docs check`
- 全体: `python3 scripts/manager.py verify full`

`build-compiler` を 1 行修正ごとに回さない（`45s × N` で律速になる）。編集をバッチして
1 回だけ rebuild → 多数 fixture を検証する。並列レーンは親が 1 回だけ rebuild する。
`selfhost fixpoint --build --no-cache` を emitter 作業の再ビルドに使わない。
コピーは `/bin/cp -f`（対話的 `cp -iv` 禁止）。詳細は `docs/compiler/bootstrap.md`。

変更範囲に応じた追加コマンドは `docs/data/verification-commands.toml` と対象 issue を確認して選ぶ。
