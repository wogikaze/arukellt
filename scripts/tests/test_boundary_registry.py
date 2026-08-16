from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.boundary_registry import (  # noqa: E402
    BoundaryRegistryError,
    iter_file_references,
    load_registry,
    validate_registry,
    validate_registry_files,
)
from proof.boundary_registry_receipt import (  # noqa: E402
    BoundaryRegistryReceiptError,
    create_validation_receipt,
    validate_validation_receipt,
)


class FakeSource:
    def __init__(self, files: dict[tuple[str, str], bytes]) -> None:
        self.files = files

    def fetch(self, repository: str, commit: str, path: str) -> bytes:
        del repository
        try:
            return self.files[(commit, path)]
        except KeyError as exc:
            raise BoundaryRegistryError(f"missing fake file: {commit}:{path}") from exc


class BoundaryRegistryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.registry_path = ROOT / "release" / "boundary-registry.json"
        self.registry = load_registry(self.registry_path)
        token_sets: dict[tuple[str, str], set[str]] = {}
        for _, ref in iter_file_references(self.registry):
            key = (ref["commit"], ref["path"])
            token_sets.setdefault(key, set()).update(ref["required_tokens"])
        self.files = {
            key: ("\n".join(sorted(tokens)) + "\n").encode("utf-8")
            for key, tokens in token_sets.items()
        }
        self.source = FakeSource(self.files)

    def test_complete_registry_validates_all_eight_boundaries(self) -> None:
        fetched = validate_registry_files(self.registry, self.source)
        self.assertEqual(set(self.registry["required_boundaries"]), {
            "typed-corehir",
            "corehir-body",
            "verified-core",
            "mir-optimizer",
            "backend-layout",
            "solver-result",
            "release-authorization",
            "proof-semantics",
        })
        self.assertGreaterEqual(len(fetched), 33)

    def test_missing_major_boundary_is_rejected(self) -> None:
        document = copy.deepcopy(self.registry)
        document["boundaries"] = document["boundaries"][:-1]
        with self.assertRaisesRegex(BoundaryRegistryError, "boundary count mismatch"):
            validate_registry(document)

    def test_short_commit_is_rejected(self) -> None:
        document = copy.deepcopy(self.registry)
        document["boundaries"][0]["producer"]["commit"] = "c6b502c"
        with self.assertRaisesRegex(BoundaryRegistryError, "full lowercase commit SHA"):
            validate_registry(document)

    def test_branch_name_is_rejected_as_commit(self) -> None:
        document = copy.deepcopy(self.registry)
        document["boundaries"][0]["producer"]["commit"] = "agent/typed-verified-core-emitter"
        with self.assertRaisesRegex(BoundaryRegistryError, "full lowercase commit SHA"):
            validate_registry(document)

    def test_producer_cannot_be_its_own_validator(self) -> None:
        document = copy.deepcopy(self.registry)
        document["boundaries"][0]["validator"] = copy.deepcopy(
            document["boundaries"][0]["producer"]
        )
        with self.assertRaisesRegex(BoundaryRegistryError, "independent files"):
            validate_registry(document)

    def test_missing_required_token_is_rejected(self) -> None:
        document = copy.deepcopy(self.registry)
        ref = document["boundaries"][0]["producer"]
        files = dict(self.files)
        files[(ref["commit"], ref["path"])] = b"unrelated content\n"
        with self.assertRaisesRegex(BoundaryRegistryError, "required token missing"):
            validate_registry_files(document, FakeSource(files))

    def test_receipt_binds_registry_and_remote_files(self) -> None:
        receipt = create_validation_receipt(self.registry_path, self.source)
        validated = validate_validation_receipt(
            receipt,
            registry_path=self.registry_path,
            source=self.source,
        )
        self.assertEqual(validated["status"], "validated")
        self.assertEqual(validated["boundary_count"], 8)

    def test_stale_remote_file_invalidates_receipt(self) -> None:
        receipt = create_validation_receipt(self.registry_path, self.source)
        files = dict(self.files)
        key = next(iter(files))
        files[key] += b"changed\n"
        with self.assertRaisesRegex(BoundaryRegistryReceiptError, "stale receipt"):
            validate_validation_receipt(
                receipt,
                registry_path=self.registry_path,
                source=FakeSource(files),
            )

    def test_duplicate_receipt_file_is_rejected(self) -> None:
        receipt = create_validation_receipt(self.registry_path, self.source)
        receipt["files"].append(copy.deepcopy(receipt["files"][0]))
        with self.assertRaisesRegex(BoundaryRegistryReceiptError, "duplicate receipt file"):
            validate_validation_receipt(receipt, registry_path=self.registry_path)

    def test_registry_digest_substitution_is_rejected(self) -> None:
        receipt = create_validation_receipt(self.registry_path, self.source)
        with tempfile.TemporaryDirectory() as directory:
            changed = Path(directory) / "registry.json"
            document = copy.deepcopy(self.registry)
            document["boundaries"][0]["failure_action"] += " changed"
            changed.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaisesRegex(BoundaryRegistryReceiptError, "does not bind"):
                validate_validation_receipt(receipt, registry_path=changed)


if __name__ == "__main__":
    unittest.main()
