#!/usr/bin/env python3
"""Require every repository release/publish entrypoint to consume proof authorization."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
CANONICAL = WORKFLOWS / "proof-required-release.yml"
CANONICAL_SCRIPT = ROOT / "scripts" / "run" / "proof-required-release.sh"

PUBLISH_TOKENS = (
    "gh release create",
    "softprops/action-gh-release",
    "actions/create-release",
    "ncipollo/release-action",
    "cargo publish",
    "npm publish",
    "pnpm publish",
    "yarn npm publish",
    "twine upload",
    "docker push",
    "docker/build-push-action",
)
AUTHORIZATION_TOKENS = (
    "release-authorization.json",
    "check-release-authorization.py",
    "proof-required-release.sh",
)


def main() -> int:
    if not CANONICAL.is_file() or not CANONICAL_SCRIPT.is_file():
        raise ValueError("canonical proof-required release entrypoint is missing")
    canonical = CANONICAL.read_text(encoding="utf-8")
    script = CANONICAL_SCRIPT.read_text(encoding="utf-8")
    for token in (
        'tags:',
        '"v*"',
        "permissions:",
        "contents: read",
        "bash scripts/run/proof-required-release.sh",
        "release-authorization.json",
    ):
        if token not in canonical:
            raise ValueError(f"canonical release workflow missing token: {token}")
    if "contents: write" in canonical:
        raise ValueError("proof authorization workflow must not publish or write releases")
    for token in (
        "check-proof-required-release.py",
        "--authorization-output",
        "--expected-commit",
        "--expected-tag",
        "--release-payload-manifest",
        "test -s \"$AUTHORIZATION\"",
    ):
        if token not in script:
            raise ValueError(f"canonical release command missing token: {token}")

    violations: list[str] = []
    for workflow in sorted((*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml"))):
        text = workflow.read_text(encoding="utf-8")
        matched = [token for token in PUBLISH_TOKENS if token in text]
        if not matched:
            continue
        if not any(token in text for token in AUTHORIZATION_TOKENS):
            violations.append(
                f"{workflow.relative_to(ROOT)}: publish token(s) {matched} without proof authorization"
            )
        if "continue-on-error: true" in text:
            violations.append(
                f"{workflow.relative_to(ROOT)}: publish workflow allows continue-on-error"
            )
    if violations:
        raise ValueError("\n".join(violations))
    print(
        "proof-release-entrypoints: PASS: "
        "canonical authorization workflow present; no unguarded publisher"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"proof-release-entrypoints: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
