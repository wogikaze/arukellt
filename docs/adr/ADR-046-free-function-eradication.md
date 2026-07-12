# ADR-046: 公開 free function の根絶

ステータス: **ACCEPTED** — ユーザー可達の free function を根絶し、例外は非公開 intrinsic のみに封じる

決定日: 2026-07-12

---

## 文脈

stdlib と prelude は長年 `func(recv, …)` 形の free function とモノモルフィック
helper（`*_i32` 等）を正面 API としてきた。ADR-044 / issue #709 は trait-first /
メソッド構文を正規としたが、[ADR-036](ADR-036-trait-stdlib-redesign.md) D5 は
prelude free function を **trait impl への恒久 thin wrapper** として残す方針だった。

その結果、「橋渡し・内部用として残す」が実質的な公開面の温存になり、
LLM 生成コードとドキュメントが free function を選び続け、`var.method(…)` へ
収束しない。

根絶対象は「優先度の問題」ではなく、**公開・ユーザー可達記号からの排除**である。

---

## 決定

### D1: ユーザー可達 free function の禁止

次を **禁止**する（最終形として残してはならない）:

- 公開・prelude・ユーザーが `use` / 無修飾で呼べる free function
  （例: `push(v, x)`, `eq(a, b)`, `i32_to_string(n)`, `map_i32_i32(...)`）
- `std` モジュール内の private free function も原則禁止
  （実装は `impl` 内の private method、または非公開 intrinsic へ）
- 公開 `kind = "prelude_wrapper"` を恒久 API とすること
- 「`std::host` は薄い FFI だから free でよい」という例外

次を **許可**する:

- メソッド呼び出し `v.push(x)`, `a.eq(b)`, `n.to_string()`
- associated function `Vec::new()`, `String::from(...)`, `i32::parse(s)`
- 無レシーバ系も free のまま残さず、型またはモジュール名前空間の associated へ寄せる
  （例: `Env::args()`, `Stdout::write(...)`, `Process::exit(c)`）

### D2: 例外は非公開 intrinsic のみ

例外として残してよいのは、コンパイラ / ランタイムが要求する **intrinsic /
emit ブリッジのみ**とする。

- 公開名前空間に出さない（`__intrinsic_*`、manifest `kind = "intrinsic"`）
- ユーザー向け記号は常に method / associated / trait 経由
- それ以外の例外は **個別 ADR または issue で明示許可**した場合のみ

ADR-042（intrinsic 層分離）の方向と整合する。ADR-042 自体の採否は本 ADR の範囲外。

### D3: ADR-036 D5 の撤回

[ADR-036](ADR-036-trait-stdlib-redesign.md) の **D5（prelude の thin wrapper 化）は撤回**する。
prelude free を「推奨メソッドの恒久エイリアス」として残す方針は採らない。

移行期間中の deprecated wrapper は許容するが、最終形ではない
（削除目標・owner issue を必須とする）。

### D4: 削除は ADR-014 に従う

`stable` の公開 free function を消すときは [ADR-014](ADR-014-stability-labels.md) に従う:

| stability | 扱い |
|-----------|------|
| `experimental` | 直接削除可（migration note 推奨） |
| `provisional` | 個別判断。原則 migration note |
| `stable` | 少なくとも 1 リリースの deprecation（W0009）+ migration guide + 削除時期の明示 |
| 既に `deprecated_by` | 定めた削除時期に削除 |

「stdlib 全体が provisional だから一括 bold cutover」は認めない
（ADR-036 D2 と同趣旨。#703 の削除実行も本節に揃える）。

### D5: 実行の所有者

- 方針・分類・scorecard: issue #709
- free → method / associated 棚卸しと tier 実行: issue #718
- モノモルフィック API 削除: issue #703
- 言語機能としての trait / メソッド: [ADR-044](ADR-044-trait-method-syntax-adopted.md)

---

## 帰結

- AGENTS.md / CLAUDE.md の「Free functions remain valid for internal helpers…」は本 ADR に合わせて削除・置換する。
- #718 の「Keep as free functions」分類は廃止し、associated / method 再分類へ書き換える。
- 本 ADR は方針のみを固定する。stdlib 一括削除は上流 issue（#691 等）完了後に実行する。

## 関連

- [ADR-014](ADR-014-stability-labels.md)
- [ADR-036](ADR-036-trait-stdlib-redesign.md)（D5 撤回）
- [ADR-042](ADR-042-intrinsic-layer-separation.md)
- [ADR-044](ADR-044-trait-method-syntax-adopted.md)
- `issues/open/709-stdlib-trait-first-api-policy.md`
- `issues/open/718-stdlib-free-function-method-migration.md`
- `issues/open/703-monomorphic-api-bold-cutover.md`
