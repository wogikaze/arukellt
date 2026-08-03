#!/usr/bin/env python3
"""Require every repository release/publish entrypoint to verify proof authorization."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / ".github" / "workflows"
CANONICAL = WORKFLOWS / "proof-required-release.yml"
CANONICAL_SCRIPT = ROOT / "scripts" / "run" / "proof-required-release.sh"
PUBLISH_AUTHORIZATION_COMMAND = "python3 scripts/check/check-release-authorization.py"

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


def main() -> int:
    if not CANONICAL.is_file() or not CANONICAL_SCRIPT.is_file():
        raise ValueError("canonical proof-required release entrypoint is missing")
    canonical = CANONICAL.read_text(encoding="utf-8")
    script = CANONICAL_SCRIPT.read_text(encoding="utf-8")
    for token in (
        "tags:",
        '"v*"',
        "permissions:",
        "contents: read",
        "bash scripts/run/proof-required-release.sh",
        PUBLISH_AUTHORIZATION_COMMAND,
        "release-authorization.json",
    ):
        if token not in canonical:
            raise ValueError(f"canonical release workflow missing token: {token}")
    if "contents: write" in canonical:
        raise ValueError("proof authorization workflow must not publish or write releases")
    for token in (
        "check-proof-required-release.py",
        "--authorization-output",
        "--expected-repository",
        "--expected-commit",
        "--expected-tag",
        "--release-payload-manifest",
        "--release-payload \"arukellt-wasm=$PAYLOAD\"",
        "test -s \"$AUTHORIZATION\"",
    ):
        if token not in script:
            raise ValueError(f"canonical release command missing token: {token}")

    violations: list[str] = []
    for workflow in sorted((*WORKFLOWS.glob("*.yml"), *WORKFLOWS.glob("*.yaml"))):
        text = workflow.read_text(encoding="utf-8")
        publish_positions = [
            (token, text.index(token))
            for token in PUBLISH_TOKENS
            if token in text
        ]
        if not publish_positions:
            continue
        relative = workflow.relative_to(ROOT)
        if PUBLISH_AUTHORIZATION_COMMAND not in text:
            violations.append(
                f"{relative}: publish token(s) {[token for token, _ in publish_positions]} "
                "without check-release-authorization.py"
            )
        else:
            authorization_position = text.index(PUBLISH_AUTHORIZATION_COMMAND)
            first_publish_position = min(position for _, position in publish_positions)
            if authorization_position > first_publish_position:
                violations.append(
                    f"{relative}: proof authorization is checked after the first publish action"
                )
            required_arguments = (
                "--authorization",
                "--repository",
                "--commit",
                "--tag",
                "--policy",
                "--source-binding",
                "--trust-manifest",
                "--proof-receipt",
                "--release-payload-manifest",
                "--release-payload",
            )
            prefix = text[authorization_position:first_publish_position]
            missing = [argument for argument in required_arguments if argument not in prefix]
            if missing:
                violations.append(
                    f"{relative}: authorization check before publish is missing argument(s): {missing}"
                )
        if "continue-on-error: true" in text:
            violations.append(
                f"{relative}: publish workflow allows continue-on-error"
            )
    if violations:
        raise ValueError("\n".join(violations))
    print(
        "proof-release-entrypoints: PASS: "
        "canonical authorization workflow present; every publisher verifies first"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"proof-release-entrypoints: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
