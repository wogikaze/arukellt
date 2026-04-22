# Type System Stage-Up Plan (HM Core + Coherent Traits) (Operational Guide)

> **Status:** Design / Implementation Guide — ready for phased execution with verification checkpoints
> **For agentic workers:** Execute phase-by-phase. Do not mix soundness work, solver semantics, and lowering work in the same slice.

**Goal:** Raise Arukellt’s type system one stage from a Rust-leaning, substitution-based local inference engine into a **principled static polymorphism core**:
- keep Arukellt’s current strengths: selfhost-first implementation, explicit item signatures, static dispatch, deterministic monomorphization
- adopt the best high-leverage ideas from Haskell: internal type schemes, let-generalization, qualified constraints
- keep the language predictable: coherent impl resolution, ambiguity rejection, conservative local generalization
- do **not** chase full GHC in this issue

**Target outcome:** After this issue, Arukellt should support **predictable polymorphic local helpers + principled trait constraints** in the selfhost compiler, with a lowering contract strong enough to unblock retirement of the Rust typechecker path.

**Work Streams (DO NOT MIX):**
1. Selfhost typechecker core — `src/compiler/typechecker.ark`
2. Frontend surface / syntax / resolver glue — `src/compiler/parser.ark`, `src/compiler/resolver.ark`, `docs/language/spec.md`
3. Lowering contract — `src/compiler/corehir.ark`, `src/compiler/mir.ark`, `src/compiler/emitter.ark`
4. Verification / fixtures / diagnostics — `tests/fixtures/*`, `scripts/check/*`, `scripts/run/*`
5. Rust parity / retirement guardrails — `crates/ark-typecheck`, `crates/ark-lsp`, `crates/ark-playground-wasm`, `issues/open/577-phase7-delete-ark-typecheck.md`

**Key Constraints:**
- This issue is **not** “make Arukellt full Haskell”.
- This issue is **not** “add every Rust type-system feature”.
- This issue **is** “establish a principled core”: sound unification, controlled let-polymorphism, coherent trait solving, and deterministic monomorphization.
- Keep **static dispatch only**. No `dyn`.
- Do not delete `crates/ark-typecheck` while the selfhost checker still lacks semantic parity for the features covered here.
- Surface syntax may stay close to current Arukellt; internal type-system machinery may become significantly more structured.

---

## Gap Summary

### Current state in repo

**Selfhost checker (`src/compiler/typechecker.ark`) currently has:**
- substitution-based `fresh_var` / `unify` / `bind_var` / `resolve_type`
- raw `TypeInfo` bindings in local scope via `scope_define` / `scope_lookup`
- generic call-site instantiation plus `mono_instances` / `mono_call_sites`
- basic trait-bound checks via `register_trait_impl` / `type_satisfies_trait_bound`
- bool-only match exhaustiveness
- no visible `TypeScheme`, `generalize`, `forall`, or occurs check

**Current open work already proves the gap exists:**
- #312: selfhost generic monomorphization is still incomplete
- #495: selfhost trait bounds / constraint solving is still blocked upstream
- #577: Rust typechecker deletion is planned, but only after selfhost semantics are strong enough to replace it

**Current language/docs constraints:**
- `docs/current-state.md` makes the selfhost driver and corehir path the default path
- `docs/language/spec.md` keeps traits static-dispatch-only and reserves `where` / `type`
- `docs/adr/ADR-004-trait-strategy.md` explicitly warns that coherence / orphan-style rules must be decided before trait resolution becomes large

### Gap to the next-stage target

Arukellt is currently closer to **“monomorphic local inference + ad hoc generic instantiation”** than to a principled polymorphic system.

The next-stage target is:

1. **From raw local types to type schemes**
   - current: `let` bindings store raw `TypeInfo`
   - target: selected bindings generalize into `Scheme(vars, constraints, ty)`

2. **From eager trait checks to obligation-based solving**
   - current: unresolved type variables effectively pass `type_satisfies_trait_bound`
   - target: unresolved bounds become obligations; obligations are solved after unification and rejected if ambiguous or unsatisfied

3. **From permissive substitution to sound substitution**
   - current: no occurs check is visible in `bind_var` / `unify`
   - target: infinite/self-referential types are rejected deterministically

4. **From partial body checking to item-boundary type contracts**
   - current: function body checking is present, but return-type enforcement is not a first-class explicit contract
   - target: item signatures become hard boundaries for bidirectional checking and diagnostics

5. **From partial generic recording to full lowering contract**
   - current: specialization metadata is recorded, but selfhost monomorphization is not yet closed end-to-end (#312)
   - target: generalized + constrained source types lower into deterministic monomorphic CoreHIR/MIR without semantic gaps

6. **From ad hoc trait support to coherent static overloading**
   - current: trait matching is shallow
   - target: one predictable impl selection model, no overlap ambiguity, no silent fallback through unresolved vars

---

## Execution Phases

### Phase 0: Baseline + Semantic Freeze

**Purpose:** Record current behavior and freeze the exact delta this issue is meant to close. Observe first. Do not redesign mid-flight.

**Execution:**

```bash
cargo build -p arukellt
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

**Record:**
- current selfhost pass/fail/skip counts
- whether nested generic instantiation still fails or is partial (#312 evidence)
- current trait-bound behavior on unresolved type vars
- current return-type mismatch behavior in selfhost
- current match exhaustiveness coverage
- current selfhost vs Rust-checker semantic differences visible from fixtures / playground / LSP

**Required design artifacts before implementation proceeds:**
- one ADR or design note defining the target core:
  - internal `TypeScheme`
  - local generalization policy
  - obligation representation
  - coherence / overlap policy
  - ambiguity policy
  - lowering contract into CoreHIR/MIR

**First work targets:**
- `src/compiler/typechecker.ark`
- `docs/language/spec.md`
- `docs/adr/`
- `issues/open/312-selfhost-generic-monomorphization.md`
- `issues/open/495-selfhost-trait-bounds.md`

---

### Phase 1: Soundness Floor

**Goal:** Make the existing selfhost inference engine safe and explicit enough to serve as a base for generalization.

#### 1-1. Add occurs check to substitution

**Target:** `src/compiler/typechecker.ark`

**Implementation:**
- add `occurs_in_type(var, ty)` or equivalent
- reject `bind_var(t, ty)` when `t` occurs inside `ty`
- ensure occurs check walks nested `type_args`

**Why first:** no principled polymorphism should be layered on top of unsound substitution.

#### 1-2. Make substitution and instantiation deep and total

**Target:** `src/compiler/typechecker.ark`

**Implementation:**
- ensure instantiation walks nested type arguments
- ensure resolve/deep-resolve is used at all specialization and comparison boundaries
- close shallow-substitution gaps that still leak into #312

#### 1-3. Enforce function boundary contracts

**Target:** `src/compiler/typechecker.ark`

**Implementation:**
- compare `return` expressions and final body type against the function’s declared / inferred return type
- reject silent drift between body result and item signature
- improve diagnostics to identify the function boundary that failed

#### 1-4. Expand exhaustiveness from bool-only toward current spec reality

**Target:** `src/compiler/typechecker.ark`, fixtures

**Implementation:**
- keep the bool case
- add at least the next immediately useful exhaustiveness layer for current enums / Option / Result patterns where the selfhost path can decide it deterministically

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

**Phase 1 Exit Condition:** the selfhost checker rejects infinite/self-referential types, enforces function boundary contracts, and no longer relies on shallow generic substitution.

---

### Phase 2: Type Schemes + Controlled Let-Generalization

**Goal:** Introduce principled polymorphism without making local inference unpredictable.

**Key policy:** Do **not** generalize everything blindly. Adopt a **conservative generalization rule** for this phase.

#### 2-1. Introduce internal `TypeScheme`

**Target:** `src/compiler/typechecker.ark`

**Implementation:**
- add an internal representation equivalent to:
  - quantified variables
  - deferred trait constraints
  - body type
- local scope stores schemes for generalizable bindings instead of only raw `TypeInfo`

#### 2-2. Instantiate on lookup

**Implementation:**
- `scope_lookup` (or successor API) instantiates fresh type variables from a stored scheme on each use
- repeated use of a polymorphic helper in the same scope must no longer alias the same raw type variable state

#### 2-3. Generalize at safe boundaries

**Required policy for this issue:**
- always generalize eligible top-level items
- local `let` generalization must be **conservative**, not “infer everything everywhere”
- use a syntactic value / non-expansive rule or an equivalently predictable restriction
- document the exact rule in spec + fixtures

**Rationale:** Arukellt is not pure Haskell. The language should gain polymorphism without turning local inference into a source of invisible surprises.

#### 2-4. Preserve explicitness at item boundaries

**Implementation:**
- explicit signatures remain the preferred API boundary
- local inference may improve; public item contracts must stay stable and visible

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

**Phase 2 Exit Condition:** local polymorphic helpers work in selfhost fixtures, repeated use instantiates freshly, and the generalization rule is documented and predictable.

---

### Phase 3: Qualified Constraints + Coherent Trait Solving

**Goal:** Upgrade traits from shallow checks into a real static overloading discipline.

#### 3-1. Replace “unresolved vars pass” with obligations

**Current problem:**
- `type_satisfies_trait_bound` currently treats unresolved type variables as effectively acceptable

**Implementation:**
- unresolved bounds become obligations attached to the current inference state / scheme
- only discharge obligations after enough unification information exists
- unsolved obligations at close-out become errors, not silent success

#### 3-2. Define and enforce coherence

**Required policy for this issue:**
- one visible impl per resolved `(Trait, SelfType)` pair
- overlapping impls are rejected
- if full orphan rules depend on package identity not yet stabilized, define the rule now and implement the strongest compilation-unit-local subset that is currently enforceable

**Do not defer this again.**
ADR-004 already identifies coherence as the critical line between “usable trait system” and “unpredictable trait system”.

#### 3-3. Add ambiguity checks

**Implementation:**
- reject generalized signatures and call sites whose constraints cannot be solved to a unique meaning
- prefer errors at definition sites when possible, not only at far-away use sites

#### 3-4. Keep surface syntax minimal in this phase

**Allowed:**
- continue using current `T: Trait` style bounds if it keeps parser changes small
- activate `where` only if it materially improves readability and does not explode implementation scope

**Not required in this phase:**
- associated types
- default methods
- specialization
- dynamic dispatch

**Verification (mandatory):**

```bash
python scripts/manager.py verify quick
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
```

**Phase 3 Exit Condition:** selfhost trait solving is obligation-based, unresolved variables no longer auto-pass, overlap ambiguity is rejected, and trait-bound fixtures demonstrate positive + negative + ambiguous cases.

---

### Phase 4: Monomorphization / Lowering Contract Closure

**Goal:** Ensure the richer frontend type system still lowers to deterministic static code.

#### 4-1. Close #312 as part of this phase

**Implementation:**
- finish generic specialization for direct calls, method calls, and nested generic concretizations
- ensure the typechecker → CoreHIR/MIR contract carries enough information to materialize concrete instances without ad hoc fallback

#### 4-2. Make trait-resolved calls deterministic

**Implementation:**
- trait method resolution must produce a deterministic concrete callee before or during lowering
- no unresolved trait dispatch enters the backend
- keep static dispatch only

#### 4-3. Preserve selfhost reproducibility

**Implementation:**
- no nondeterministic iteration in obligation solving or specialization emission
- stable ordering for generated specializations
- stable symbol mangling for generalized/constrained items

**Verification (mandatory):**

```bash
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/manager.py verify
```

**Phase 4 Exit Condition:** monomorphization is end-to-end complete for the feature set introduced here, and selfhost fixpoint still holds after the solver/type-scheme upgrade.

---

### Phase 5: Docs / Tooling / Retirement Unblock

**Goal:** Make the new semantics the documented and verified source of truth.

#### 5-1. Update normative docs

**Targets:**
- `docs/current-state.md`
- `docs/language/spec.md`
- `docs/language/type-system.md`
- relevant ADRs

**Required content:**
- generalization policy
- trait coherence policy
- ambiguity behavior
- lowering / monomorphization contract
- explicit non-goals for this stage

#### 5-2. Expand fixture coverage

**Must add:**
- positive / negative occurs-check fixtures
- positive / negative return-contract fixtures
- polymorphic local helper fixtures
- ambiguous trait-bound fixtures
- overlap / duplicate impl rejection fixtures
- nested generic monomorphization fixtures

#### 5-3. Align secondary consumers

**Targets:**
- `crates/ark-lsp`
- `crates/ark-playground-wasm`
- any remaining Rust-side typechecker consumers

**Requirement:**
- either consume the selfhost semantics
- or be explicitly marked / gated as lagging until parity lands

#### 5-4. Unblock `crates/ark-typecheck` retirement

**Requirement:**
- this issue does not delete `crates/ark-typecheck`
- it must leave behind a state where #577 is blocked only by remaining consumer migration / deletion work, not by missing type-system semantics

**Phase 5 Exit Condition:** docs, fixtures, and tool surfaces agree on the new type-system core; Rust typechecker retirement is semantically unblocked.

---

## Non-Goals

This plan explicitly does **not** include:

- higher-kinded types
- GADTs
- type families
- associated types
- specialization / overlapping instance priority rules
- dynamic dispatch / trait objects
- deriving / newtype-deriving
- effect systems / row polymorphism
- full Haskell-style inference in every local context

These may become follow-up issues **after** the core in this plan is stable.

---

## Existing Issues This Plan Must Coordinate (Do Not Duplicate)

- #312 — selfhost generic instantiation and monomorphization
- #495 — selfhost trait bounds and constraint solving
- #512 — stdlib trait-oriented reuse surface (downstream consumer)
- #577 — delete `crates/ark-typecheck` (downstream retirement gate)

This plan should either:
- subsume them explicitly with child slices, or
- stay as the orchestration umbrella while those issues remain the implementation leaves

Do **not** let the queue drift into duplicate partial fixes.

---

## Daily Operational Procedure

**Per work unit (single concern only):**

1. **Select one slice**
   - Example: occurs check only
   - Or: let-generalization storage only
   - Or: trait obligation emission only
   - Or: monomorphization of generic method calls only

2. **Observe before change**

   ```bash
   python scripts/manager.py verify quick
   python scripts/manager.py selfhost fixpoint
   python scripts/manager.py selfhost fixture-parity
   python scripts/manager.py selfhost diag-parity
   ```

3. **Implement one concern only**

4. **Verify immediately**

   ```bash
   python scripts/manager.py verify quick
   python scripts/manager.py selfhost fixture-parity
   python scripts/manager.py selfhost diag-parity
   ```

5. **Run full gate for boundary-crossing changes**

   ```bash
   python scripts/manager.py selfhost fixpoint
   python scripts/manager.py verify
   ```

6. **Record deltas**
   - fixture pass/fail/skip counts
   - diagnostic parity deltas
   - selfhost fixpoint status
   - newly accepted / newly rejected programs
   - docs/spec updates required by behavior changes

---

## Close Gate

Close this issue only when all of the following are true:

- selfhost typechecker has:
  - sound substitution with occurs check
  - controlled let-generalization via internal type schemes
  - obligation-based trait solving
  - coherence / ambiguity rejection for the supported feature set
  - deterministic monomorphization through lowering
- #312 is closed or fully subsumed by completed work under this plan
- #495 is closed or fully subsumed by completed work under this plan
- normative docs describe the new semantics precisely
- fixture + diagnostic coverage prove both acceptance and rejection behavior
- `python scripts/manager.py selfhost fixpoint` passes
- `python scripts/manager.py verify` passes
- #577 is no longer blocked by missing frontend type-system semantics
