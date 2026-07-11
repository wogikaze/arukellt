# ADR-0000: ADR Process and Status Lifecycle

ステータス: **ACCEPTED** — ADR の識別子・状態遷移・supersession 規則を固定する

決定日: 2026-07-11

---

## Context

`docs/adr/` に設計判断・調査・実装計画・進捗メモが混在し、番号重複・未定義 status・
未来日付・欠番参照が発生していた。ADR を意思決定履歴の正本として使うには、
プロセス自体を ADR として固定する必要がある。

## Decision

### 1. Allowed status values

Primary lifecycle (exactly one per ADR):

| Status | Meaning |
|--------|---------|
| `PROPOSED` | 提案中。採択前。本文は候補判断として書く。 |
| `ACCEPTED` | 採択済み。現行の有効な決定。 |
| `SUPERSEDED` | 後継 ADR により置換済み。`Superseded-by: ADR-NNN` を必須とする。 |

Auxiliary terminal states (also allowed; mutually exclusive with the primary trio):

| Status | Meaning |
|--------|---------|
| `REJECTED` | 提案を却下。後継は不要。 |
| `DEFERRED` | 判断を保留。再開条件（trigger）を本文に書く。 |

**Forbidden / legacy aliases** (must not appear as the canonical status token):

- `DECIDED` → use `ACCEPTED`
- `DRAFT` → use `PROPOSED`
- `SURVEY` → not an ADR status; move survey content out of `docs/adr/` or convert to a narrow `ACCEPTED` decision
- Free-form status dashboards (PR progress, fixture counts) → keep in issues / `current-state` / RFCs

Canonical header form (Japanese or English):

```text
ステータス: **ACCEPTED** — <one-line decision summary>
```

or

```text
**Status**: ACCEPTED — <one-line decision summary>
```

### 2. Identity rules

- ADR numbers are permanent identifiers. Never reuse a number for a different decision.
- Exactly one non-tombstone body file may own a given number, except a tombstone redirect
  that points at the surviving file.
- Filename pattern: `ADR-NNN-kebab-slug.md` (zero-padding optional for legacy `ADR-0001`).
- Gaps are allowed. Do not renumber existing ADRs to fill gaps.
- Deleted or consolidated ADRs leave a **tombstone** file that records:
  - original title
  - status `SUPERSEDED` (or `REJECTED`)
  - successor / absorption target when applicable
  - deletion/consolidation date and one-line reason

### 3. Supersession

- `Supersedes: ADR-NNN` / `Superseded-by: ADR-NNN` must name an existing ADR file
  (body or tombstone).
- Self-reversal inside one file is allowed only as a dated revision of an `ACCEPTED`
  decision; do not mark `SUPERSEDED` without a successor ADR.

### 4. Dates

- Decision and revision dates must be on or before the commit date (no future dates).
- Planned work uses explicit wording (`Planned`, issue links), never a future “改訂/完了” date.

### 5. What belongs in an ADR

Keep: context, decision, rejected alternatives, consequences, revisit triggers, links.

Do not keep as living state: fixture counts, PR lane dashboards, runtime version pins,
phase checklists that change weekly. Those belong in issues, RFCs, plans, or
`docs/current-state.md`.

### 6. Index and CI

- `docs/adr/README.md` is generated and groups ADRs by status
  (`Accepted` / `Proposed` / `Superseded` / `Deferred` / `Rejected`).
- `scripts/check/check-adrs.py` enforces ID uniqueness, allowed status tokens,
  no future dates, and supersession target existence.

## Consequences

- Existing ADRs migrate aliases (`DECIDED`/`DRAFT`/`SURVEY`) to the table above.
- Survey and epic-sized documents may remain temporarily under `docs/adr/` during
  registry repair, but later PRs should split RFC/plan/state content out.
- Verify gates treat `ACCEPTED` as the decided state (legacy `DECIDED` is rejected by CI).

## References

- Audit repair plan (2026-07): ADR registry repair before RFC/state separation
- `scripts/check/check-adrs.py`
- `scripts/gen/generate-docs.py` (ADR README rendering)
