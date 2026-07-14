# Tooling inventory (file families)

Machine-readable SSOT: [`docs/data/tooling-inventory.toml`](../data/tooling-inventory.toml).
Policy: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

Each extension has at most one formatter. Deferred tools are assigned as
canonical owners but are not enforced until a follow-on issue lands.
The quality-contract check compares this inventory with every extension in
`git ls-files`, so a newly tracked file family must declare an owner and its
single formatter (or explicitly declare that no formatter applies).

The `.ark` enforced roots are `src/compiler/` and `std/`. Existing parse gaps
are not glob exclusions: they are exact-hash exceptions in
[`ark-formatter-baseline.toml`](../data/ark-formatter-baseline.toml), and any
content change invalidates the exception. Fixtures intentionally containing
invalid Ark syntax remain outside the default repository-wide format roots.
