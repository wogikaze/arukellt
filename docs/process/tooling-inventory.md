# Tooling inventory (file families)

Machine-readable SSOT: [`docs/data/tooling-inventory.toml`](../data/tooling-inventory.toml).
Policy: [ADR-047](../adr/ADR-047-code-quality-tooling-and-gates.md).

Each extension has at most one formatter. Deferred tools are assigned as
canonical owners but are not enforced until a follow-on issue lands.
The quality-contract check compares this inventory with every extension in
`git ls-files`, so a newly tracked file family must declare an owner and its
single formatter (or explicitly declare that no formatter applies).

The `.ark` enforced roots are `src/compiler/` and `std/`. Every tracked file
under those roots parses under the canonical formatter; issue #791 removed
the last content-addressed parse exceptions and deleted
`docs/data/ark-formatter-baseline.toml`. Reintroducing a baseline file
requires the metadata contract enforced by `quality structure` (owner,
issue, removal condition, re-evaluation date). Fixtures intentionally
containing invalid Ark syntax remain outside the default repository-wide
format roots.
