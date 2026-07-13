# CODEOWNERS plan (not yet enforced)

[ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md) does not require
CODEOWNERS for acceptance. When owners are known, add `.github/CODEOWNERS`
with paths such as:

```text
/src/compiler/    @OWNER
/std/             @OWNER
/docs/data/       @OWNER
/scripts/gen/     @OWNER
/scripts/check/   @OWNER
```

Replace `@OWNER` with the repository maintainer team or users. Until then,
path ownership is tracked by issue assignees and this document.
