#!/usr/bin/env python3
"""Write release commit/ref provenance for proof binding."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.release_provenance import create_release_provenance  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", default=os.environ.get("GITHUB_REPOSITORY"))
    parser.add_argument("--commit", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--ref-type", choices=("tag", "branch", "pull_request"))
    parser.add_argument("--ref-name", default=os.environ.get("GITHUB_REF_NAME"))
    parser.add_argument("--workflow", default=os.environ.get("GITHUB_WORKFLOW", "local"))
    parser.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", "local"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    ref_type = args.ref_type
    if ref_type is None:
        ref = os.environ.get("GITHUB_REF", "")
        if ref.startswith("refs/tags/"):
            ref_type = "tag"
        elif ref.startswith("refs/pull/"):
            ref_type = "pull_request"
        else:
            ref_type = "branch"
    if not args.repository or not args.commit or not args.ref_name:
        raise ValueError("repository, commit, and ref name are required")

    document = create_release_provenance(
        repository=args.repository,
        commit_sha=args.commit,
        ref_type=ref_type,
        ref_name=args.ref_name,
        workflow=args.workflow,
        run_id=str(args.run_id),
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "release-provenance: PASS: "
        f"repository={document['repository']} ref={document['ref_type']}:{document['ref_name']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"release-provenance: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
