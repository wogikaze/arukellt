# RFC-001: Playground Share URL Format（wire format）

ステータス: 仕様草案（ADR-021 が採択した中核判断の詳細）  
関連 ADR: [ADR-021](../adr/ADR-021-playground-share-url-format.md)  
日付: 2026-05-15（ADR 採択） / 抽出: 2026-07-11

本 RFC は fragment ベース share URL の **byte layout・符号化・版・上限・エラー処理** の正本である。
採択判断（なぜ fragment か等）は ADR-021。

---

## 決定

### 1. URL 構造

共有 URL は playground URL の fragment に、版付きパス構造を使う:

```
<base-url>/playground#share/<format-version>/<payload>
```

**例:**

```
https://arukellt.dev/playground#share/1/eNpLSS0u0c1IzcnJVyjPL8pJUQQALLwF5Q
```

構成要素:

| 要素 | 説明 |
|------|------|
| `<base-url>/playground` | playground ページ URL（ホスト依存） |
| `#share/` | 共有リンクを示す fragment 接頭辞 |
| `<format-version>` | 整数スキーマ版（現行 `1`） |
| `<payload>` | 圧縮・符号化された状態（§2–§4） |

`#share/` 接頭辞は、他の fragment 用途（例: `#example/hello` でキュレート例を ID 読み込み、
将来の fragment ナビ）と区別する。playground ルータは接頭辞を見て動作を決める。

### 2. ペイロード符号化パイプライン

playground 状態を URL 安全な文字列へ変換する:

```
   PlaygroundState (object)
        │
        ▼
   JSON.stringify()          →  UTF-8 JSON string
        │
        ▼
   deflate (raw, no header)  →  compressed bytes
        │
        ▼
   base64url encode          →  URL-safe ASCII string
        │
        ▼
   Append to fragment        →  #share/1/<payload>
```

復号は厳密な逆順:

```
   Fragment payload string
        │
        ▼
   base64url decode          →  compressed bytes
        │
        ▼
   inflate (raw)             →  UTF-8 JSON string
        │
        ▼
   JSON.parse()              →  PlaygroundState (object)
        │
        ▼
   Validate against schema   →  Validated state or error
```

#### 2.1 JSON 直列化

playground 状態は JSON オブジェクトとして直列化する。キーは**正規順**（アルファベット順）で
出し、決定的出力を保証する — 同じ論理状態は常に同じ URL になる。実装は直列化前に
キーをソートしなければならない（MUST）。

#### 2.2 圧縮: Raw DEFLATE（RFC 1951）

圧縮は **raw DEFLATE**（RFC 1951）— ラッパなしの DEFLATE（zlib/gzip ヘッダなし）。
最もコンパクトで、ラッパ形式の 2–6 バイトオーバーヘッドを避ける。

**DEFLATE を選ぶ根拠:**

| 選択肢 | 利点 | 欠点 | 判定 |
|--------|------|------|------|
| Raw DEFLATE + base64url | 標準、ブラウザ支援良好（pako, fflate）、テキスト圧縮良好 | base64url で約 33% 膨張 | **✅ 採用** |
| LZ-String | URL 向け、URL 安全出力を直接生成 | 非標準、単一メンテナ JS、ネイティブ API なし、構造化テキストでは DEFLATE より劣る | ❌ 却下 |
| Brotli | 最高圧縮率 | 2026 時点で全対象ブラウザに `CompressionStream` がない、展開器が大きい | ❌ v1 では却下 |
| 無圧縮 | 最も単純 | 非自明なプログラム（>30 行）で URL が実用不能 | ❌ 却下 |

**実装注:** ブラウザの `CompressionStream` / `DecompressionStream` は `"deflate-raw"` を
サポートする。無い場合は `pako` または `fflate` を使う。実装は raw DEFLATE
（`deflate-raw` / `pako.deflateRaw` / `fflate.deflateSync`）を MUST とし、
zlib ラップの `deflate` や `gzip` を使ってはならない。

#### 2.3 Base64url 符号化（RFC 4648 §5）

圧縮バイトは **base64url**（RFC 4648 §5）で符号化する:

- アルファベット: `A-Z a-z 0-9 - _`（標準 base64 の `+` `/` を置換）
- **パディングなし**（`=` を省略）
- percent-encoding なしで URL 安全

JWT や WebAuthn など、URL 埋め込みバイナリと同じ符号化である。

### 3. ペイロードスキーマ（版 1）

形式版 `1` の JSON ペイロード:

```json
{
  "src": "<string>",
  "ver": "<string>",
  "ex":  "<string>",
  "f":   { "<key>": <value>, ... }
}
```

| フィールド | 型 | 必須 | 説明 |
|------------|-----|------|------|
| `src` | string | **yes** | ソース本文（UTF-8）。空文字は可だが必須。 |
| `ver` | string | no | 共有リンクを作ったコンパイラ/フロント版（semver、例 `"0.1.0"`）。 |
| `ex` | string | no | Example ID。ある場合、キュレート例から読み込んだことを示す。slug と一致（例 `"hello-world"`）。 |
| `f` | object | no | Feature flags。キーは kebab-case、値は bool または string。未知フラグは復号時に保持し playground は無視。 |

**短いフィールド名の理由:** `src`/`ver`/`ex`/`f` は圧縮前 JSON を小さくする。
URL は機械生成・機械解析なので、生 JSON の可読性は優先しない。

#### 3.1 最小ペイロード例

最小の有効共有ペイロード（空プログラム、任意フィールドなし）:

```json
{"src":""}
```

圧縮・符号化後の fragment はおおよそ 20 文字。

#### 3.2 典型ペイロード例

```json
{"ex":"hello-world","f":{"diag-verbose":true},"src":"fn main() {\n  println(\"Hello, world!\")\n}","ver":"0.1.0"}
```

注: キーはアルファベット順（正規形）。

#### 3.3 未知フィールド

デコーダは再符号化時に未知のトップレベルフィールドを保持しなければならない（MUST）。
将来版がフィールドを足しても、古い playground が損失なく運べる。未知フィールドを
理由にペイロードを拒否してはならない（MUST NOT）。

### 4. 版ピン留め

`ver` は共有リンクを生成した Arukellt フロントエンド（parser / typechecker / formatter）の版を記録する。目的は二つ:

1. **診断コンテキスト** — 共有リンク経由のバグ報告で、どのコンパイラが出した診断か分かる。
2. **将来互換** — 言語意味が版間で変わったとき、playground は
   「この snippet は版 X、実行中は版 Y」と案内できる。

#### 4.1 版文字列形式

`ver` は **semver**（`MAJOR.MINOR.PATCH`、例 `"0.1.0"`）。
`src/compiler/parser.ark` / playground Wasm バンドルの版に合わせる。
プレリリース接尾辞（例 `"0.2.0-dev"`）は許容。

#### 4.2 版不一致時の挙動

実行中の版と異なる `ver` を復号したとき:

- ソースは変換せずそのまま読み込む。
- 情報バナーを出してよい（MAY）: _"この snippet は版 X.Y.Z から共有されました。表示中は版 A.B.C です。挙動が異なる場合があります。"_
- 読み込みを拒否してはならない（MUST NOT）。
- 再共有すると `ver` は現行版に更新される。

#### 4.3 版なし

`ver` が無い場合は版未指定として扱い、バナーは出さない。手動構築や v1 初期 URL で想定される。

### 5. URL 長上限とフォールバック

#### 5.1 目標長バジェット

| 要素 | バジェット |
|------|------------|
| Base URL + path | ~40 文字 |
| Fragment 接頭辞（`#share/1/`） | 9 文字 |
| Payload | 残り |
| **合計 URL 目標** | **≤ 8,192 文字** |

base64url ペイロードに約 **8,143 文字**、圧縮データ約 **6,107 バイト**。
典型ソースで DEFLATE が ~40–60% なら、ソース約 **10,000–15,000 文字**を支えられる。

#### 5.2 ブラウザ / プラットフォーム上限

| プラットフォーム | 実用上限 | 状態 |
|------------------|----------|------|
| Chrome / Edge | アドレスバー ~2 MB | ✅ 余裕 |
| Firefox | ~65,536 文字 | ✅ 余裕 |
| Safari | ~80,000 文字 | ✅ 余裕 |
| Twitter / X | 短縮されるが展開で fragment 保持 | ✅ |
| GitHub Issues / Markdown | `href` に実用上限なし | ✅ |
| Slack | 表示は ~1,000 で切るがリンクは完全 URL | ⚠️ 表示短縮だが機能する |
| メールクライアント | まちまち。一部は 2,083 で折り返し（旧 IE） | ⚠️ 一部で壊れる |

8,192 文字目標はほぼすべての共有文脈と両立する。

#### 5.3 上限超過時のフォールバック

符号化 URL が 8,192 を超えるとき（ソースが非常に大きい）:

1. **警告** — _"この snippet は URL 共有には大きすぎます（N 文字）。コードを短くしてください。"_
2. **それでも URL を生成** — アドレスバーに置く。多くのブラウザでは動くが、一部共有文脈では失敗しうる。
3. **ダウンロードを提供** — 「.ark としてダウンロード」を代替共有にする。本文は生ソース、メタデータはコメントヘッダ。
4. **黙って切り詰めてはならない** — ソースを URL バジェットに合わせて切らない。
   §6 のラウンドトリップが成り立たないなら URL を生成しない。

#### 5.4 硬上限

**65,536 文字**（Firefox 上限）を超える URL は生成してはならない（MUST NOT）。
エラーを出し、ファイルダウンロードのみを提示する。

### 6. ラウンドトリップ契約

共有形式の基本不変条件:

```
∀ state ∈ ValidPlaygroundState:
    decode(encode(state)) = state
```

形式的には:

1. **Encode** は `PlaygroundState` を URL fragment 文字列へ変換する。
2. **Decode** は fragment 文字列を `PlaygroundState` へ戻す。
3. 任意の有効状態について、符号化して復号した結果は元と**意味的に同一**でなければならない（MUST）。

#### 6.1 意味的同一性

次をすべて満たすときのみ意味的に同一:

- `src` がバイト同一（UTF-8）
- `ver` が同一文字列、または両方欠如
- `ex` が同一文字列、または両方欠如
- `f` が同じキー値対（順不同）、または両方欠如/空

#### 6.2 正規符号化

JSON キーをアルファベット順に出すため（§2.1）、符号化は**決定的**である:

```
∀ state: encode(state₁) = encode(state₂)  ⟺  state₁ = state₂
```

URL 比較を状態比較の代理にできる。

#### 6.3 復号エラー処理

いずれかの段階で失敗したら（不正 base64url、展開失敗、壊れた JSON、必須 `src` 欠如）:

- 既定状態（空エディタまたは既定例）を読み込む
- エラーバナー: _"共有 snippet を読み込めませんでした。リンクが壊れているか、非互換の版の可能性があります。"_
- クラッシュや白紙ページにしてはならない

#### 6.4 テスト契約

共有機能実装時、次のラウンドトリップ試験が MUST で通ること:

| ケース | 入力 `src` | 検証内容 |
|--------|------------|----------|
| 空文字 | `""` | 最小ペイロード |
| ASCII のみ | `"fn main() {}"` | 基本ラウンドトリップ |
| Unicode | `"// こんにちは\nfn main() {}"` | UTF-8 保持 |
| 大きなプログラム | 有効 Arukellt 10,000 文字 | URL 上限内の圧縮 |
| 任意フィールド全部 | `src` + `ver` + `ex` + `f` | フルスキーマ |
| 未知フィールド | 余分な `"x": 42` | 前方互換の保持 |
| 特殊 JSON 文字 | `"`, `\`, `\n`, `\t`, `\u0000` | JSON エスケープ |

ここは契約の指定であり、実装は別作業である。

### 7. 前方互換

#### 7.1 形式版の進行

URL の `<format-version>`（`#share/<version>/...`）は、符号化パイプラインが
後方非互換に変わるときだけ上げる:

| 変更種別 | 版上げ? | 例 |
|----------|---------|-----|
| 新しい任意 JSON フィールド | **No**（§3.3） | `"theme": "dark"` 追加 |
| 新しい必須 JSON フィールド | **Yes** | `"ver"` を必須化 |
| 別圧縮アルゴリズム | **Yes** | DEFLATE → Brotli |
| 別 base 符号化 | **Yes** | base64url → base45 |
| フィールド削除 | **No**（任意欠如を許容） | `"ex"` 削除 |

#### 7.2 複数版の復号

playground は**過去の全形式版**の復号を MUST でサポートする。版を上げても旧 decode 経路を残す。
URL 内の整数でディスパッチする:

```
switch (version) {
  case 1: return decodeV1(payload);
  case 2: return decodeV2(payload);
  default: return { error: "Unsupported share format version" };
}
```

#### 7.3 符号化は常に最新版

エンコーダは常に**最新形式版**の URL を出す。旧形式を出す手段はない。
旧形式を復号して再共有すると新形式 URL になる。

### 8. Fragment 名前空間

playground URL fragment は接頭辞で分割する:

| 接頭辞 | 用途 | 例 |
|--------|------|-----|
| `#share/<v>/` | 共有/permalink（本 ADR） | `#share/1/eNpLSS0u...` |
| `#example/<id>` | キュレート例を ID で読み込み | `#example/hello-world` |
| _(fragment なし)_ | 既定状態（空エディタ） | `/playground` |

将来接頭辞（`#tutorial/`、`#diff/` など）を足してよい。未知接頭辞はエラーではなく
no-op（既定状態読み込み）として扱う（MUST）。

---


## 検討した代替案

### A. サーバー保存 Snippet（Gist / DB）

**方式:** ソースを POST し短い ID を受け取り、共有 URL は ID のみ。

**v1 では却下:**
- ADR-017 は v1 でバックエンドなしを要求。
- サーバー基盤・レート制限・悪用対策・ストレージ費用が必要。
- リンク解決がサーバー可用性に依存する。
- URL 符号化形式と並ぶ **v2 の任意強化**として追加できる
  （URL 長超過の大きなプログラム向け短縮 URL など）。

### B. LZ-String 符号化

**方式:** `lz-string` の `compressToEncodedURIComponent()` で URL 安全出力を直接得る。

**却下:**
- 単一メンテナ・非標準圧縮。
- ネイティブブラウザ API がなく常に JS 依存。
- 構造化テキストでは DEFLATE より圧縮率が劣る。
- DEFLATE は長年の標準化、複数実装、`CompressionStream` によるネイティブ支援がある。

### C. Brotli 圧縮

**方式:** Brotli（RFC 7932）で DEFLATE より良い圧縮率。

**v1 では却下:**
- 2026-04 時点で `CompressionStream("br")` が全対象ブラウザにない。
- Brotli 展開器の同梱（~30 KB）が DEFLATE より大きい。
- 圧縮率の優位（DEFLATE より ~10–15%）は v1 の互換リスクに見合わない。
- ブラウザ支援が普遍化したとき形式版 2 として導入できる。

### D. Fragment ではなく Query String

**方式:** `?share=...` に共有データを置く。

**却下:**
- Query は HTTP でサーバーへ送られ、アクセスログ・CDN・分析にソースが漏れる。
- 一部 CDN/プロキシは query 部分の URL 長上限がより厳しい。
- SPA のクライアント専用状態の標準位置は fragment（`#`）。

### E. 無圧縮 Base64url

**方式:** 圧縮せず JSON を直接 base64url。

**却下:**
- 100 行（~2,000 文字）で無圧縮 ~2,700 文字 vs DEFLATE ~1,100 文字。
  約 60% の削減は URL 長バジェット維持に不可欠。

---


