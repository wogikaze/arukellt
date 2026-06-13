# Formatter canonical surface

The selfhost formatter (`src/compiler/fmt/`) is the single source of truth for
canonical Arukellt source layout. CLI `arukellt fmt`, LSP
`textDocument/formatting` / `textDocument/rangeFormatting`, VS Code format
actions, and the playground **Format** button all call the same `format_source()`
API.

## Behavior contract

- **Parse gate**: invalid sources are left unchanged; CLI exits non-zero and LSP
  returns no edits (#344).
- **Indentation**: 4 spaces per level; `} else {` keeps the `else` branch body
  indented one level inside the else block.
- **Whitespace**: tabs expand to spaces, trailing whitespace is removed, and
  consecutive blank lines collapse to a single blank line.
- **Comments**: full-line `//` comments and lines containing `/*` block comment
  openers keep their trimmed text (#343).
- **Imports**: `sort_imports()` orders `use` lines as stdlib (`std::…`) first,
  then other paths in source order, then aliased imports last (#346). Full
  `format_source()` applies import sorting; `source.organizeImports` calls
  `sort_imports()` only.
- **Range formatting**: LSP range requests snap to whole lines; only the
  selected line span may change (#347).
- **Output**: formatted text ends with a single trailing newline.

## Idempotency and validity

Golden fixtures under `tests/fixtures/fmt/` verify:

1. output matches committed `.expected` files
2. `fmt(fmt(x)) == fmt(x)`
3. formatted output parses successfully (`arukellt check`)

## CLI

```bash
arukellt fmt path/to/file.ark          # rewrite in place
arukellt fmt path/to/file.ark --check  # exit 1 when formatting would change text
```

## References

- Implementation: `src/compiler/fmt/mod.ark`
- Tooling matrix: [../tooling-feature-matrix.md](../tooling-feature-matrix.md)
- Issues: #216 (surface), #343–#347 (formatter contracts)
