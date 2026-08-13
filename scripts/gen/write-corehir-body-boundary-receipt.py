#!/usr/bin/env python3
"""Write a hash-bound receipt for the frozen CoreHIR body boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CHECKER = "scripts/check/check-corehir-body-boundary.py"
COMPONENTS = (
    ("artifact", "src/compiler/corehir/body_table.ark"),
    ("builder", "src/compiler/corehir/body_builder.ark"),
    ("validator", "src/compiler/corehir/body_validator.ark"),
    ("construction_facade", "src/compiler/corehir/body.ark"),
    ("construction_handoff", "src/compiler/corehir/body_roots.ark"),
    ("detached_mir_snapshot", "src/compiler/corehir/mir_body_source.ark"),
    ("static_boundary_gate", CHECKER),
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / ".build" / "proof" / "corehir-body-boundary.json",
    )
    args = parser.parse_args()

    subprocess.run([sys.executable, str(ROOT / CHECKER)], cwd=ROOT, check=True)
    components: list[dict[str, str]] = []
    for role, relative in COMPONENTS:
        path = ROOT / relative
        if not path.is_file():
            raise ValueError(f"boundary component missing: {relative}")
        components.append({"role": role, "path": relative, "sha256": sha256(path)})

    document = {
        "schema": "arukellt-corehir-body-boundary-receipt",
        "schema_version": 1,
        "artifact_schema_version": 1,
        "status": "enforced",
        "construction_policy": {
            "mutation_owner": "src/compiler/corehir/body_builder.ark",
            "handoff": "corehir_body_builder_finish",
            "invalid_artifact_action": "compiler-ice",
        },
        "consumption_policy": {
            "artifact_api": "count/index accessors; compatibility snapshots are deep copies",
            "storage_vector_aliases": "forbidden",
            "renderer_handoff": "detached deep snapshot",
            "mir_handoff": "detached deep snapshot",
        },
        "components": components,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"corehir-body-boundary-receipt: PASS: output={args.output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError, ValueError) as exc:
        print(f"corehir-body-boundary-receipt: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
