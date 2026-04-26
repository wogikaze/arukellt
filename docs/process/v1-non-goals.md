# v1 Non-Goals

> **Archive / current-first note**:
> this document preserves design boundaries and non-goals, but it is **not** the source of truth for current shipped semantics.
> For current behavior, use `docs/current-state.md`, executable baselines, and current language docs.

This document lists design decisions and implementation constraints that are explicitly out of scope or prohibited during v1 work.

## Current-first corrections

The older wording in this file must be read with these corrections in mind:

- current production path is T1; do not treat full T3 semantics as shipped reality
- assignment / capture semantics are **shared reference** for current reference-like values, not deep copy on assignment
- `W0004` is now treated as a hard error in the backend validation gate

## Non-goals that still apply

1. Do not add native-only features to T4
2. Do not start WASI p3 implementation
3. Do not expose allocator internals as public user API
4. Do not grow the public CLI surface just for refactor internals
5. Do not move backend-specific optimization policy into frontend semantics

## Scope reminder

This file is useful as a guardrail, not as a snapshot of current implementation status.
