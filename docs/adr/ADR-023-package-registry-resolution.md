# ADR-023: パッケージレジストリ解決の設計

ステータス: **ACCEPTED** — Registry lookupモデル（local > workspace > registry）を採用
日付: 2026-04-14
決定者: Module-system track (issue #487)
決定日: 2026-04-14

## 背景

Arukellt の依存関係解決（`docs/module-resolution.md` §5 に文書化）は、現時点でローカルパス依存（`{ path = "..." }`）のみをサポートする。マニフェスト形式はすでにレジストリ依存用のバージョン文字列構文（`some-pkg = "1.2.3"`）を予約しているが、その解決ロジックは存在しない。

Issue #487 がこのギャップを追跡する。本 ADR は lookup モデル、失敗時診断、明示的な非目標を定義する。

ADR-009 はソースレベルの `use` と Component Model の `import` を分離する。レジストリ解決はソースレベル（`use`）レイヤー内で動作する。

## 決定

### 1. レジストリ lookup モデル

依存エントリが素のバージョン文字列のとき、レジストリ解決が有効になる:

```toml
[dependencies]
foo = "1.2.3"          # registry dependency
bar = { path = "../bar" }  # local — unchanged
```

解決は次の順序で進む（文書化された優先度は変更なし）:

1. **Local path** — `path` キーがあれば、パッケージルート相対で解決する。
2. **Workspace member** — `workspace = true` なら、ワークスペース内で解決する。
3. **Registry** — 値がバージョン文字列なら、レジストリに問い合わせる。

#### レジストリ問い合わせ契約

resolver は `(package-name, version-constraint)` の lookup を、設定された単一のレジストリエンドポイントに発行する。エンドポイント URL はプロジェクトレベルまたはユーザーレベル設定から読む:

```toml
# ark.toml (project) or ~/.config/arukellt/config.toml (user)
[registry]
url = "https://registry.arukellt.dev/v1"
```

`[registry]` セクションがなければ、resolver はコンパイル時デフォルト URL を使う。

問い合わせは JSON マニフェストを返す HTTP GET である。内容には次を含む:

- `name` — パッケージ名（問い合わせと一致必須）
- `version` — 解決されたバージョン
- `checksum` — 整合性ハッシュ（SHA-256）
- `archive_url` — パッケージ tarball のダウンロード URL

resolver はアーカイブをダウンロードし、ユーザーごとのキャッシュディレクトリ（`~/.cache/arukellt/registry/<name>/<version>/`）に展開する。以降のビルドは checksum が変わらない限りキャッシュを再利用する。

#### バージョン制約構文

初期実装は完全一致バージョン文字列のみ（`"1.2.3"`）をサポートする。Semver 範囲と pre-release 処理はフォローアップに延期。

### 2. 失敗時診断

| シナリオ | エラーコード | メッセージパターン |
|----------|-----------|-----------------|
| レジストリ到達不能（ネットワーク / タイムアウト） | E0120 | `registry unreachable: {url} ({reason})` |
| レジストリにパッケージなし | E0121 | `package '{name}' not found in registry` |
| バージョンなし | E0122 | `version '{version}' of '{name}' not found in registry` |
| ダウンロード後の checksum 不一致 | E0123 | `integrity check failed for '{name}@{version}'` |
| レジストリ未設定かつデフォルトなし | E0124 | `no registry configured; add [registry] to ark.toml` |

すべてのエラーはコンパイル時診断（resolver は codegen 前に実行）。既存の解決エラー用 E01xx 番号ブロックに従う。

レジストリに到達できないが、制約に一致するキャッシュ版がある場合、resolver は警告を出しキャッシュを使う（オフライン優先フォールバック）。

### 3. 非目標

以下は本設計および初期実装で**明示的にスコープ外**である:

- **レジストリサービスのホスティング** — 本 ADR で API 契約は定義するが、サービス立ち上げは別のインフラ課題。
- **認証とプライベートレジストリ** — フォローアップ。エンドポイント契約は拡張可能だが当面は公開・非認証アクセスを仮定。
- **Semver 範囲解決** — 当面は完全一致のみ。
- **ロックファイル形式** — 依存のピン留めとロックファイルによる再現可能ビルドは別途追跡。
- **公開ワークフロー** — `arukellt publish` とパッケージアップロードは本 ADR の対象外。
- **ミラーリングとフォールバックレジストリ** — 当面は単一エンドポイントのみ。

## 根拠

1. **最小表面**: 完全一致のみにより初期実装の semver ソルバ複雑さを避けつつ、レジストリ経路を end-to-end で実証できる。
2. **オフライン優先**: checksum 付きキャッシュはネットワーク不可時も決定的ビルドを与え、Cargo/npm のユーザー期待に合う。
3. **明確なエラーコード**: 各失敗モードに専用 E01xx コードで、曖昧さなく actionable な診断になる。
4. **設定のレイヤリング**: プロジェクトレベル `ark.toml` がユーザーレベル設定を上書きし、他の Arukellt 設定と同じ優先度。
5. **ADR-009 整合**: レジストリパッケージは `use` で import し、Layer S（Source）/ Layer C（Component）分離と一致。

## 検討した代替案

### A. 各依存エントリにレジストリ URL を埋め込む

```toml
[dependencies]
foo = { version = "1.2.3", registry = "https://custom.example.com" }
```

当面は却下 — 依存ごとの複雑さが増す。マルチレジストリは後で再検討可能。

### B. Git ベース解決（レジストリなし）

`{ git = "https://...", tag = "v1.2.3" }` のみをリモートソースとする。

唯一の仕組みとしては却下 — git 依存は名前空間ガバナンスや整合性保証を提供しない。レジストリと併用の補完ソースとしては追加可能。

### C. オフラインは vendoring のみ

`arukellt vendor` でソースをプロジェクトツリーにコピーすることを必須とする。

主要手段としては却下 — checksum 検証キャッシュが同等の再現性を、ソースツリー汚染を少なく提供する。

## 実装メモ

- エラーコード E0120–E0124 をエラーカタログに追加する必要がある。
- resolver 配線時に `docs/module-resolution.md` §5 と §9 を更新する。
- キャッシュディレクトリは Linux/macOS で XDG 規約に従う。
