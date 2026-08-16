from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.source_proof_binding import (  # noqa: E402
    REQUIRED_ARTIFACTS,
    VERSION,
    SourceProofBindingError,
    validate_binding,
    write_binding,
)


class SourceProofBindingTests(unittest.TestCase):
    def test_binding_detects_stale_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths: dict[str, Path] = {}
            for index, name in enumerate(REQUIRED_ARTIFACTS):
                path = root / f"{name}.bin"
                path.write_bytes(f"artifact-{index}".encode())
                paths[name] = path
            output = root / "binding.json"
            written = write_binding(paths, output)
            loaded = json.loads(output.read_text())
            self.assertEqual(written, loaded)
            self.assertEqual(loaded["schema_version"], 5)
            validate_binding(loaded, paths)

            paths["typed_corehir_canonical"].write_bytes(b"changed")
            with self.assertRaisesRegex(SourceProofBindingError, "digest mismatch"):
                validate_binding(loaded, paths)

    def test_v5_requires_raw_canonical_and_registry_evidence(self) -> None:
        self.assertEqual(VERSION, 5)
        self.assertIn("typed_corehir", REQUIRED_ARTIFACTS)
        self.assertIn("typed_corehir_canonical", REQUIRED_ARTIFACTS)
        self.assertIn("boundary_registry", REQUIRED_ARTIFACTS)
        self.assertIn("boundary_registry_validation_receipt", REQUIRED_ARTIFACTS)

    def test_stale_registry_receipt_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths: dict[str, Path] = {}
            for index, name in enumerate(REQUIRED_ARTIFACTS):
                path = root / f"{name}.bin"
                path.write_bytes(f"artifact-{index}".encode())
                paths[name] = path
            output = root / "binding.json"
            written = write_binding(paths, output)
            paths["boundary_registry_validation_receipt"].write_bytes(b"changed")
            with self.assertRaisesRegex(
                SourceProofBindingError,
                "boundary_registry_validation_receipt: digest mismatch",
            ):
                validate_binding(written, paths)

    def test_binding_requires_complete_artifact_set(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            paths = {
                name: root / f"{name}.bin"
                for name in REQUIRED_ARTIFACTS[:-1]
            }
            for path in paths.values():
                path.write_bytes(b"x")
            with self.assertRaisesRegex(SourceProofBindingError, "missing binding artifact"):
                write_binding(paths, root / "binding.json")


if __name__ == "__main__":
    unittest.main()
