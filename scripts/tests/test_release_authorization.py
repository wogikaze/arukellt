from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.release_authorization import (  # noqa: E402
    ReleaseAuthorizationError,
    create_release_authorization,
    validate_bound_release_authorization,
    write_release_authorization,
)


class ReleaseAuthorizationTests(unittest.TestCase):
    def test_authorization_binds_complete_bundle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            files = {
                name: root / name
                for name in (
                    "policy.json",
                    "binding.json",
                    "manifest.json",
                    "receipt.json",
                    "payload.json",
                )
            }
            for index, path in enumerate(files.values()):
                path.write_text(f"artifact-{index}\n", encoding="utf-8")
            authorization_path = root / "authorization.json"
            document = create_release_authorization(
                repository="wogikaze/arukellt",
                commit_sha="a" * 40,
                tag="v1.0.0",
                policy_path=files["policy.json"],
                source_binding_path=files["binding.json"],
                trust_manifest_path=files["manifest.json"],
                proof_receipt_path=files["receipt.json"],
                payload_manifest_path=files["payload.json"],
            )
            write_release_authorization(document, authorization_path)
            validate_bound_release_authorization(
                authorization_path,
                repository="wogikaze/arukellt",
                commit_sha="a" * 40,
                tag="v1.0.0",
                policy_path=files["policy.json"],
                source_binding_path=files["binding.json"],
                trust_manifest_path=files["manifest.json"],
                proof_receipt_path=files["receipt.json"],
                payload_manifest_path=files["payload.json"],
            )

            files["receipt.json"].write_text("substituted\n", encoding="utf-8")
            with self.assertRaisesRegex(
                ReleaseAuthorizationError, "proof_receipt_sha256 mismatch"
            ):
                validate_bound_release_authorization(
                    authorization_path,
                    repository="wogikaze/arukellt",
                    commit_sha="a" * 40,
                    tag="v1.0.0",
                    policy_path=files["policy.json"],
                    source_binding_path=files["binding.json"],
                    trust_manifest_path=files["manifest.json"],
                    proof_receipt_path=files["receipt.json"],
                    payload_manifest_path=files["payload.json"],
                )

    def test_authorization_rejects_different_tag(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            files = [root / f"f{index}" for index in range(5)]
            for path in files:
                path.write_text("x", encoding="utf-8")
            authorization_path = root / "authorization.json"
            write_release_authorization(
                create_release_authorization(
                    repository="wogikaze/arukellt",
                    commit_sha="b" * 40,
                    tag="v1.0.0",
                    policy_path=files[0],
                    source_binding_path=files[1],
                    trust_manifest_path=files[2],
                    proof_receipt_path=files[3],
                    payload_manifest_path=files[4],
                ),
                authorization_path,
            )
            with self.assertRaisesRegex(ReleaseAuthorizationError, "tag mismatch"):
                validate_bound_release_authorization(
                    authorization_path,
                    repository="wogikaze/arukellt",
                    commit_sha="b" * 40,
                    tag="v2.0.0",
                    policy_path=files[0],
                    source_binding_path=files[1],
                    trust_manifest_path=files[2],
                    proof_receipt_path=files[3],
                    payload_manifest_path=files[4],
                )


if __name__ == "__main__":
    unittest.main()
