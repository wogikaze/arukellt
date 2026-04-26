# docs 内コード例の自動検証 CI 追加

**Status**: done
**Created**: 2026-04-02
**Updated**: 2026-04-03
**ID**: 461
**Depends on**: none
**Track**: docs, ci
**Blocks v1 exit**: no
**Priority**: 2

---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: check-doc-examples.py integrated in verify-harness.sh at lines 218-219

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).

## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/461-doc-example-ci.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

## Summary

`docs/stdlib/reference.md` や module page に書いた `.ark` fenced code block がドキュメント更新と共に腐っていくのを防ぐ。Markdown から `ark` 言語指定のコードブロックを抽出し、`arukellt check` にかける CI スクリプトを追加する。失敗時はどのファイルの何番目のブロックが壊れたかを明示する。

---

## 現状の問題

- `docs/stdlib/reference.md` や `docs/language/guide.md` にサンプルコードが増えている。
- `scripts/check/check-docs-consistency.py` はシンボル/fixture のドリフト検査はするが、コードブロック自体の実行可否は検査しない。
- Issue 12 のコードコメント（`<!-- fixture: path/to/fixture -->` によるリンク）は guide.md 限定であり、stdlib docs のインラインスニペットはカバーしていない。

---

## 詳細実装内容

### Step 1: スニペット抽出スクリプトを作る (`scripts/check/check-doc-examples.sh`)

```bash
#!/usr/bin/env bash
# Usage: check-doc-examples.sh [--run] [docs-dir]
set -euo pipefail
DOCS_DIR="${2:-docs}"
MODE="${1:---check}"  # --check or --run
TMP=$(mktemp -d)
PASS=0; FAIL=0; SKIP=0

for md in $(find "$DOCS_DIR" -name '*.md' | sort); do
    block_num=0
    python3 - "$md" "$TMP" <<'PYEOF'
import sys, re, pathlib, hashlib

md_path = pathlib.Path(sys.argv[1])
out_dir = pathlib.Path(sys.argv[2])
text = md_path.read_text(encoding='utf-8')

# Extract ```ark ... ``` blocks
pattern = re.compile(r'^```ark\n(.*?)^```', re.MULTILINE | re.DOTALL)
for i, m in enumerate(pattern.finditer(text)):
    code = m.group(1)
    slug = hashlib.md5(f"{md_path}:{i}".encode()).hexdigest()[:8]
    out = out_dir / f"{md_path.stem}_{i}_{slug}.ark"
    out.write_text(code, encoding='utf-8')
    print(f"{out}|{md_path}|{i}")
PYEOF
done

for line in $(find "$TMP" -name '*.ark' -print | sort); do
    meta_line=$(grep -r "^$line|" /dev/stdin 2>/dev/null || true)
    # fallback: parse filename for metadata
    snippet="$line"
    if arukellt check "$snippet" --target wasm32-wasi-p1 2>/dev/null; then
        PASS=$((PASS+1))
    else
        # show which doc file and block number
        base=$(basename "$snippet" .ark)
        echo "FAIL: $base"
        arukellt check "$snippet" --target wasm32-wasi-p1
        FAIL=$((FAIL+1))
    fi
done

rm -rf "$TMP"
echo "Doc example check: $PASS pass, $FAIL fail, $SKIP skip"
[ "$FAIL" -eq 0 ]
```

### Step 2: Python スクリプトとして書き直す（信頼性重視）

シェルスクリプトよりも Python スクリプトの方が可搬性が高い。`scripts/check/check-doc-examples.py` として実装する。

```python
#!/usr/bin/env python3
"""Extract and check all ```ark code blocks in docs/."""
from __future__ import annotations
import argparse, re, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

def extract_blocks(md_path: Path) -> list[tuple[int, str]]:
    """Return list of (block_index, code) for each ```ark block."""
    text = md_path.read_text(encoding='utf-8')
    pattern = re.compile(r'```ark\n(.*?)```', re.DOTALL)
    return [(i, m.group(1)) for i, m in enumerate(pattern.finditer(text))]

def check_block(code: str, md_path: Path, block_idx: int, target: str) -> bool:
    with tempfile.NamedTemporaryFile(suffix='.ark', mode='w', delete=False) as f:
        f.write(code)
        tmp = Path(f.name)
    try:
        result = subprocess.run(
            ['arukellt', 'check', str(tmp), '--target', target],
            capture_output=True, text=True
        )
        if result.returncode != 0:
            print(f"FAIL: {md_path}  block #{block_idx}")
            print(result.stderr or result.stdout)
            return False
        return True
    finally:
        tmp.unlink(missing_ok=True)

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--docs', default=str(ROOT / 'docs'))
    parser.add_argument('--target', default='wasm32-wasi-p1')
    parser.add_argument('--allow-skip', action='store_true',
                        help='Skip blocks with a `# doc-skip` comment')
    args = parser.parse_args()

    docs_dir = Path(args.docs)
    pass_count = fail_count = skip_count = 0

    for md in sorted(docs_dir.rglob('*.md')):
        for idx, code in extract_blocks(md):
            if args.allow_skip and '# doc-skip' in code:
                skip_count += 1
                continue
            if check_block(code, md, idx, args.target):
                pass_count += 1
            else:
                fail_count += 1

    print(f"\nDoc example check: {pass_count} pass, {fail_count} fail, {skip_count} skip")
    return 0 if fail_count == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
```

### Step 3: `# doc-skip` コメントによる除外機能

コードブロック内の最初の行に `# doc-skip` があれば検査対象から外す。使用例:

```ark
# doc-skip  (このスニペットは説明用で実行不可)
// error 例: これはコンパイルエラーになる意図のサンプル
let x: i32 = "hello"
```

`# doc-error` でエラーが出ることを期待するブロックとして扱う（将来拡張。本 issue では skip のみ実装）。

### Step 4: CI 統合 (`scripts/run/verify-harness.sh`)

`verify-harness.sh` の全パスに以下を追加する。

```bash
echo "=== doc example check ==="
python3 scripts/check/check-doc-examples.py --allow-skip
```

`--quick` フラグ付きでは実行しない（時間がかかるため）。

### Step 5: 既存 docs の修正

スクリプト初回実行で壊れているブロックを特定し、以下のいずれかで対処する。

1. コードを現在の構文に修正する
2. `# doc-skip` を追加する（実行不可なサンプルの場合）
3. fixture へのリンクコメント（`<!-- fixture: ... -->`）に切り替える

初回実行で発覚した壊れているブロックの件数と対処内容を issue のコメントに記録する。

### Step 6: `check-docs-consistency.py` への統合（オプション）

`check_doc_examples()` 関数を `check-docs-consistency.py` に追加し、既存の一括整合性チェックと同じフローで実行できるようにする。これにより `python3 scripts/check/check-docs-consistency.py` 1 本で全チェックが走る。本 issue では独立スクリプトとして実装し、統合は任意とする。

---

## 依存関係

- 依存なし（独立して着手可能）
- Issue 455（stdlib metadata v2）完了後、新しく追加されたサンプルがすぐ検証対象になる

---

## 影響範囲

- `scripts/check/check-doc-examples.py`（新規）
- `scripts/run/verify-harness.sh`（追加）
- `docs/` 内の壊れたコードブロック（修正 or skip タグ付け）

---

## 後方互換性

- docs の既存コンテンツに `# doc-skip` を追加するのみ。文書の意味は変わらない。

---

## 今回の範囲外

- `# doc-error` によるエラー期待ブロックの検証
- `--run` による実行結果の検証（`check` のみ）
- docs 外（README.md 等）のコードブロック

---

## 完了条件

- [x] `python3 scripts/check/check-doc-examples.py` が `docs/` 全体を走査して結果を出力する
- [x] 壊れているブロックには `# doc-skip` または修正が施されており、失敗ゼロで完了する
- [x] `verify-harness.sh` full pass にこのスクリプトが組み込まれている
- [x] 失敗時の出力に「どの .md ファイルの何番目のブロック」が明確に出る
- [x] `bash scripts/run/verify-harness.sh` 通過

---

## 必要なテスト

1. `check-doc-examples.py` 自身の unit test: 正常なブロック → pass、壊れたブロック → fail、`# doc-skip` 付き → skip
2. 既存 docs のブロック全件がスクリプト実行で全通過すること（初回実行後の修正完了を確認）

---

## 実装時の注意点

- `arukellt check` はファイルパスを引数に取るため、一時ファイルへの書き出しが必要。ファイル名が診断出力に出るが、一時ファイル名でも構わない（どの md のブロックかはスクリプト側で出力する）。
- `use std::host::*` を含むブロックは `--target wasm32-wasi-p2` が必要な場合がある。ブロック先頭のコメント `# target: wasm32-wasi-p2` で target を指定できるようにしておく（将来対応でも可）。
- 大量の docs がある場合はファイル数・ブロック数を `--docs docs/stdlib` のように絞れるようにしておく。
