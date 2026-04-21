# Agent Harness Guide

This is a short pointer document for developers and agents working in this repository.

## Quick Start

1. **Setup**: Follow `docs/process/agent-harness.md` for bootstrap instructions
2. **Tasks**: See `AGENTS.md` for repository contract and `scripts/manager.py` for verification
3. **Queue**: Check `issues/open/` for active work; see `issues/done/` for completed items
4. **Decisions**: Review `docs/adr/` for workflow and design decisions

## Verification Loop

```bash
python3 scripts/manager.py verify --quick
```

All verification is routed through `scripts/manager.py`. The shell script
`scripts/run/verify-harness.sh` is kept as an optional local convenience wrapper.

### Verify subcommands

```bash
python3 scripts/manager.py verify --quick       # fast local gate (default)
python3 scripts/manager.py verify --fixtures    # fixture harness
python3 scripts/manager.py verify --size        # hello.wasm size gate
python3 scripts/manager.py verify --wat         # WAT roundtrip
python3 scripts/manager.py verify --component   # component interop
python3 scripts/manager.py verify --full        # all of the above
```

### Other domains

```bash
python3 scripts/manager.py selfhost fixpoint       # selfhost bootstrap fixpoint
python3 scripts/manager.py selfhost fixture-parity # fixture parity
python3 scripts/manager.py selfhost diag-parity    # diagnostic parity
python3 scripts/manager.py selfhost parity         # full parity

python3 scripts/manager.py docs check              # consistency + freshness + examples
python3 scripts/manager.py docs regenerate         # regenerate docs

python3 scripts/manager.py perf gate               # perf regression gate
python3 scripts/manager.py perf baseline           # collect baseline
python3 scripts/manager.py perf benchmarks         # run benchmarks

python3 scripts/manager.py gate local              # full local CI equivalent
python3 scripts/manager.py gate pre-commit         # pre-commit checks
python3 scripts/manager.py gate pre-push           # pre-push checks
python3 scripts/manager.py gate repro              # reproducible build check
```

All subcommands accept `--dry-run` to print intent without executing.

## Making Changes

1. Pick a task from `issues/open/`
2. Move it conceptually through: planning → implementation → verification
3. Update relevant docs/ADRs if approach changes
4. Run `python3 scripts/manager.py verify --quick` for a fast local verify pass
5. Move completed task to `issues/done/`

## Key Files

- **AGENTS.md** - Repository contract and boundaries
- **docs/adr/** - Architectural and workflow decisions
- **docs/process/agent-harness.md** - This document
- **issues/open/** - Active work queue
- **scripts/manager.py** - Primary verification and tooling CLI

## If Something Breaks

1. Check `python3 scripts/manager.py verify --quick` output for specifics
2. Review recent ADRs in `docs/adr/` for context
3. Examine the queue in `issues/open/` for ongoing work
4. Refer to language-specific documentation for domain issues
