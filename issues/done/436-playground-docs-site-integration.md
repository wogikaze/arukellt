---
Status: open
Created: 2026-04-03
Updated: 2026-04-21
ID: 436
Track: playground
Depends on: 437, 438, 464
Orchestration class: verification-ready
Orchestration upstream: —
Implementation target: "Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan."
---

## Reopened by audit

## Summary

playground を独立ページで終わらせず、docs site から自然に辿れるようにする。examples や language/stdlib docs と行き来できる navigation を作る。

## Current state

- docs site と playground は分離されているどころか、playground 自体がない。
- docs examples から playground を開きたい需要がある。
- navigation を決めないと hidden feature になりやすい。

## Acceptance

- [x] docs site から playground への入口が追加される。
- [x] language / stdlib docs から example を playground で開ける導線がある。
- [x] playground から docs へ戻る導線がある。
- [x] site navigation に統合される。

## References

- ``docs/index.html``
- ``docs/examples/**``
- ``docs/stdlib/**``
- ``docs/language/**``
