from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.common import sha256_file  # noqa: E402
from proof.release_gate import (  # noqa: E402
    ProofRequiredReleaseError,
    validate_proof_required_release,
)
from proof.source_proof_binding import (  # noqa: E402
    SourceProofBindingError,
    write_binding,
)


class ProofRequiredReleaseGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.paths = {
            "source": self.root / "source.ark",
            "producer_executable": self.root / "compiler.wasm",
            "typed_corehir": self.root / "typed-corehir.json",
            "verified_core_machine": self.root / "verified-core-machine.json",
            "verified_core_normalized": self.root / "verified-core.json",
            "solver_input": self.root / "vcs.smt2",
        }
        for index, path in enumerate(self.paths.values()):
            path.write_bytes(f"artifact-{index}\n".encode())

        self.binding_path = self.root / "source-proof-binding.json"
        write_binding(self.paths, self.binding_path)
        self.manifest_path = self.root / "trust-manifest.json"
        self.policy_path = self.root / "release-policy.json"
        self.write_manifest()
        self.policy_path.write_text(
            json.dumps(
                {
                    "schema": "arukellt-proof-release-policy",
                    "schema_version": 1,
                    "mode": "proof-required",
                    "hard_gates": {},
                    "artifacts": [
                        {
                            "trust_manifest": self.manifest_path.name,
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_manifest(self, *, binding_digest: str | None = None) -> None:
        self.manifest_path.write_text(
            json.dumps(
                {
                    "producer": {
                        "executable_sha256": sha256_file(
                            self.paths["producer_executable"]
                        )
                    },
                    "trusted_components": [
                        {
                            "role": "source-artifact-binding",
                            "artifact_sha256": binding_digest
                            or sha256_file(self.binding_path),
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

    def validate(self) -> tuple[str, int]:
        with patch(
            "proof.release_gate.check_release_policy",
            return_value=("proof-required", 1),
        ), patch(
            "proof.release_gate.validate_trust_manifest",
            side_effect=lambda value: value,
        ):
            return validate_proof_required_release(
                self.policy_path,
                self.binding_path,
                self.paths,
            )

    def test_accepts_complete_source_to_receipt_chain(self) -> None:
        self.assertEqual(self.validate(), ("proof-required", 1))

    def test_rejects_stale_source_digest(self) -> None:
        self.paths["source"].write_text("changed\n", encoding="utf-8")
        with self.assertRaisesRegex(SourceProofBindingError, "source: digest mismatch"):
            self.validate()

    def test_rejects_unbound_source_binding(self) -> None:
        self.write_manifest(binding_digest="0" * 64)
        with self.assertRaisesRegex(
            ProofRequiredReleaseError,
            "does not bind the supplied source proof binding",
        ):
            self.validate()

    def test_rejects_unbound_compiler_executable(self) -> None:
        manifest = json.loads(self.manifest_path.read_text(encoding="utf-8"))
        manifest["producer"]["executable_sha256"] = "0" * 64
        self.manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
        with self.assertRaisesRegex(
            ProofRequiredReleaseError,
            "producer does not bind the supplied compiler executable",
        ):
            self.validate()

    def test_rejects_proof_optional_policy(self) -> None:
        with patch(
            "proof.release_gate.check_release_policy",
            return_value=("proof-optional", 1),
        ):
            with self.assertRaisesRegex(
                ProofRequiredReleaseError,
                "must be proof-required",
            ):
                validate_proof_required_release(
                    self.policy_path,
                    self.binding_path,
                    self.paths,
                )


if __name__ == "__main__":
    unittest.main()
