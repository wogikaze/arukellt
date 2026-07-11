# Docs size / split plan (manual documents)

Tracked under #765. Manual docs that are too coarse for review / LLM retrieval:

| Document | Approx size | Proposed split |
|----------|-------------|----------------|
| `docs/language/spec.md` | ~1600 lines | Keep normative core; move examples to guide/cookbook |
| `docs/stdlib/cookbook.md` | ~900 lines | Topic chapters under `docs/cookbook/` |
| `docs/compiler/ir-spec.md` | ~1800 lines | Per-IR chapter files + landing README |
| `docs/stdlib/reference.md` | generated | Keep generated; module pages remain primary entry |
| `docs/stdlib/name-index.md` | generated | Secondary artifact only |

Do not start splits until #765 CI gates are green and owners agree per chapter.
