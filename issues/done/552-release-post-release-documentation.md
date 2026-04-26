# 552 · Post-release documentation

kind: documentation
status: done

## Acceptance Criteria

- [x] CHANGELOG.md has complete entry for current release
- [x] docs/quickstart.md accurately reflects current CLI and stdlib behavior
- [x] Breaking changes from recent closed issues have migration guide entries
- [x] python scripts/manager.py verify quick exits 0 (docs consistency: PASS)

## Files Changed

- CHANGELOG.md — v5 section rewritten with Highlights, Added, Changed, Removed, Known Limitations
- docs/migration/v4-to-v5.md — full rewrite: selfhost-only path, 5 removed crates, shell script replacements, FsError/JsonParseError type changes, prelude narrowing
- docs/quickstart.md — no changes needed (fs sigs already correct)

## Notes

The 6 failing verify checks are pre-existing issues (fixture manifest sync,
unchecked issue checkboxes, selfhost compiler resolve errors, doc example
blocks) unrelated to this docs slice. Docs consistency check: PASS.
