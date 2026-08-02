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
from proof.release_payload import (  # noqa: E402
    ReleasePayloadError,
    create_release_payload_manifest,
)
from proof.release_provenance import (  # noqa: E402
    ReleaseProvenanceError,
    create_release_provenance,
)
from proof.source_proof_binding import (  # noqa: E402
    SourceProofBindingError,
    write_binding,
)


class ProofRequiredReleaseGateTests(unittest.TestCase):
    REPOSITORY = "wogikaze/arukellt"
    COMMIT = "1" * 40
    TAG = "v1.2.3"

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.payload = self.root / "arukellt.wasm"
        self.payload.write_bytes(b"release-payload\n")
        self.payloads = {"arukellt-wasm": self.payload}

        self.provenance_path = self.root / "release-provenance.json"
        self.provenance_path.write_text(
            json.dumps(
                create_release_provenance(
                    repository=self.REPOSITORY,
                    commit_sha=self.COMMIT,
                    ref_type="tag",
                    ref_name=self.TAG,
                    workflow="Proof required release",
                    run_id="123",
                )
            ),
            encoding="utf-8",
        )
        self.payload_manifest_path = self.root / "release-payload-manifest.json"
        self.payload_manifest_path.write_text(
            json.dumps(create_release_payload_manifest(self.payloads)),
            encoding="utf-8",
        )

        self.paths = {
            "source": self.root / "source.ark",
            "producer_executable": self.root / "compiler.wasm",
            "typed_corehir": self.root / "typed-corehir.json",
            "verified_core_machine": self.root / "verified-core-machine.json",
            "verified_core_normalized": self.root / "verified-core.json",
            "solver_input": self.root / "vcs.smt2",
            "backend_typeid_layout_receipt": self.root / "backend-typeid-layout.json",
            "optimizer_translation_registry": self.root / "mir-opt-registry.json",
            "corehir_body_boundary_receipt": self.root / "body-boundary.json",
            "release_provenance": self.provenance_path,
            "release_payload_manifest": self.payload_manifest_path,
        }
        for index, (name, path) in enumerate(self.paths.items()):
            if name in {"release_provenance", "release_payload_manifest"}:
                continue
            if name == "producer_executable":
                path.write_bytes(self.payload.read_bytes())
            else:
                path.write_bytes(f"artifact-{index}\n".encode())

        self.binding_path = self.root / "source-proof-binding.json"
        write_binding(self.paths, self.binding_path)
        self.manifest_path = self.root / "trust-manifest.json"
        self.receipt_path = self.root / "proof-receipt.json"
        self.solver_output_path = self.root / "solver-output.txt"
        self.receipt_path.write_text("{}\n", encoding="utf-8")
        self.solver_output_path.write_text("unsat\n", encoding="utf-8")
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
                            "subject": self.paths["verified_core_normalized"].name,
                            "trust_manifest": self.manifest_path.name,
                            "receipt": self.receipt_path.name,
                            "solver_output": self.solver_output_path.name,
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_manifest(
        self,
        *,
        binding_digest: str | None = None,
        subject_digest: str | None = None,
        binding_version: str = "3",
    ) -> None:
        self.manifest_path.write_text(
            json.dumps(
                {
                    "subject": {
                        "sha256": subject_digest
                        or sha256_file(self.paths["verified_core_normalized"]),
                    },
                    "producer": {
                        "executable_sha256": sha256_file(
                            self.paths["producer_executable"]
                        )
                    },
                    "trusted_components": [
                        {
                            "role": "source-artifact-binding",
                            "version": binding_version,
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
                expected_repository=self.REPOSITORY,
                expected_commit=self.COMMIT,
                expected_tag=self.TAG,
                release_payloads=self.payloads,
            )

    def test_accepts_complete_commit_bound_payload_chain(self) -> None:
        self.assertEqual(self.validate(), ("proof-required", 1))

    def test_rejects_stale_source_digest(self) -> None:
        self.paths["source"].write_text("changed\n", encoding="utf-8")
        with self.assertRaisesRegex(SourceProofBindingError, "source: digest mismatch"):
            self.validate()

    def test_rejects_stale_optimizer_registry(self) -> None:
        self.paths["optimizer_translation_registry"].write_text(
            "substituted\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(
            SourceProofBindingError, "optimizer_translation_registry: digest mismatch"
        ):
            self.validate()

    def test_rejects_stale_backend_layout_receipt(self) -> None:
        self.paths["backend_typeid_layout_receipt"].write_text(
            "substituted\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(
            SourceProofBindingError, "backend_typeid_layout_receipt: digest mismatch"
        ):
            self.validate()

    def test_rejects_unbound_source_binding(self) -> None:
        self.write_manifest(binding_digest="0" * 64)
        with self.assertRaisesRegex(
            ProofRequiredReleaseError,
            "does not bind the supplied source proof binding",
        ):
            self.validate()

    def test_rejects_legacy_source_binding_version(self) -> None:
        self.write_manifest(binding_version="2")
        with self.assertRaisesRegex(ProofRequiredReleaseError, "version 3"):
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

    def test_rejects_unrelated_proof_subject(self) -> None:
        self.write_manifest(subject_digest="0" * 64)
        with self.assertRaisesRegex(ProofRequiredReleaseError, "normalized VerifiedCore"):
            self.validate()

    def test_rejects_receipt_replay_on_different_commit(self) -> None:
        with patch(
            "proof.release_gate.check_release_policy",
            return_value=("proof-required", 1),
        ), patch(
            "proof.release_gate.validate_trust_manifest",
            side_effect=lambda value: value,
        ):
            with self.assertRaisesRegex(ReleaseProvenanceError, "commit mismatch"):
                validate_proof_required_release(
                    self.policy_path,
                    self.binding_path,
                    self.paths,
                    expected_repository=self.REPOSITORY,
                    expected_commit="2" * 40,
                    expected_tag=self.TAG,
                    release_payloads=self.payloads,
                )

    def test_rejects_receipt_replay_on_different_tag(self) -> None:
        with patch(
            "proof.release_gate.check_release_policy",
            return_value=("proof-required", 1),
        ), patch(
            "proof.release_gate.validate_trust_manifest",
            side_effect=lambda value: value,
        ):
            with self.assertRaisesRegex(ReleaseProvenanceError, "tag mismatch"):
                validate_proof_required_release(
                    self.policy_path,
                    self.binding_path,
                    self.paths,
                    expected_repository=self.REPOSITORY,
                    expected_commit=self.COMMIT,
                    expected_tag="v9.9.9",
                    release_payloads=self.payloads,
                )

    def test_rejects_payload_substitution_after_proof(self) -> None:
        self.payload.write_bytes(b"different-release-payload\n")
        with self.assertRaisesRegex(ReleasePayloadError, "payload digest mismatch"):
            self.validate()

    def test_rejects_manifest_valid_but_unproved_payload(self) -> None:
        self.payload.write_bytes(b"different-release-payload\n")
        self.payload_manifest_path.write_text(
            json.dumps(create_release_payload_manifest(self.payloads)),
            encoding="utf-8",
        )
        write_binding(self.paths, self.binding_path)
        self.write_manifest()
        with self.assertRaisesRegex(
            ProofRequiredReleaseError,
            "not the proved producer executable",
        ):
            self.validate()

    def test_rejects_proof_optional_policy(self) -> None:
        with patch(
            "proof.release_gate.check_release_policy",
            return_value=("proof-optional", 1),
        ):
            with self.assertRaisesRegex(
                ProofRequiredReleaseError, "must be proof-required"
            ):
                validate_proof_required_release(
                    self.policy_path,
                    self.binding_path,
                    self.paths,
                    expected_repository=self.REPOSITORY,
                    expected_commit=self.COMMIT,
                    expected_tag=self.TAG,
                    release_payloads=self.payloads,
                )


if __name__ == "__main__":
    unittest.main()
