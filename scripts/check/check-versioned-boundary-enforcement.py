#!/usr/bin/env python3
"""Reject a versioned-boundary hard gate that is not wired through release proof."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from proof.boundary_registry import REQUIRED_BOUNDARIES, load_registry  # noqa: E402
from proof.source_proof_binding import REQUIRED_ARTIFACTS, VERSION as BINDING_VERSION  # noqa: E402

POLICY = ROOT / "release" / "proof-policy.json"
REGISTRY = ROOT / "release" / "boundary-registry.json"
RELEASE_COMMAND = ROOT / "scripts" / "run" / "proof-required-release.sh"
BINDING_WRITER = ROOT / "scripts" / "gen" / "write-source-proof-binding.py"
RELEASE_CHECKER = ROOT / "scripts" / "check" / "check-proof-required-release.py"
TOOLCHAIN_WRITER = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain.py"
TOOLCHAIN_WRITER_IMPL = ROOT / "scripts" / "gen" / "prepare_proof_release_toolchain_impl.py"
TOOLCHAIN_WRITER_V7 = ROOT / "scripts" / "gen" / "prepare-proof-release-toolchain-v7.py"
RELEASE_WORKFLOW = ROOT / ".github" / "workflows" / "proof-required-release.yml"
PROOF_WORKFLOW = ROOT / ".github" / "workflows" / "typed-contract-frontend.yml"
REGISTRY_WORKFLOW = ROOT / ".github" / "workflows" / "versioned-boundary-registry.yml"
ENFORCEMENT_RECEIPT = ROOT / "scripts" / "gen" / "write-proof-release-enforcement-receipt.py"


def require(text: str, token: str, label: str) -> None:
    if token not in text:
        raise ValueError(f"missing {label}: {token}")


def require_order(text: str, earlier: str, later: str, label: str) -> None:
    left = text.find(earlier)
    right = text.find(later)
    if left < 0 or right < 0 or left >= right:
        raise ValueError(f"invalid {label}: {earlier!r} must precede {later!r}")


def main() -> int:
    registry = load_registry(REGISTRY)
    if set(registry["required_boundaries"]) != REQUIRED_BOUNDARIES:
        raise ValueError("boundary registry does not cover the complete required set")

    policy = json.loads(POLICY.read_text(encoding="utf-8"))
    gates = policy.get("hard_gates")
    if not isinstance(gates, dict) or gates.get("versioned_boundary_artifacts") is not True:
        raise ValueError("release policy does not enable versioned_boundary_artifacts")

    if BINDING_VERSION != 5:
        raise ValueError(f"source proof binding must be v5, found v{BINDING_VERSION}")
    required_binding_artifacts = {
        "typed_corehir",
        "typed_corehir_canonical",
        "boundary_registry",
        "boundary_registry_validation_receipt",
    }
    if not required_binding_artifacts <= set(REQUIRED_ARTIFACTS):
        raise ValueError("source proof binding omits raw/canonical proof source or registry evidence")

    release_command = RELEASE_COMMAND.read_text(encoding="utf-8")
    binding_writer = BINDING_WRITER.read_text(encoding="utf-8")
    release_checker = RELEASE_CHECKER.read_text(encoding="utf-8")
    toolchain_writer = (
        TOOLCHAIN_WRITER.read_text(encoding="utf-8")
        + "\n"
        + TOOLCHAIN_WRITER_IMPL.read_text(encoding="utf-8")
        + "\n"
        + TOOLCHAIN_WRITER_V7.read_text(encoding="utf-8")
    )
    release_workflow = RELEASE_WORKFLOW.read_text(encoding="utf-8")
    proof_workflow = PROOF_WORKFLOW.read_text(encoding="utf-8")
    registry_workflow = REGISTRY_WORKFLOW.read_text(encoding="utf-8")
    enforcement_receipt = ENFORCEMENT_RECEIPT.read_text(encoding="utf-8")

    for token, label in (
        ("check-boundary-registry.py", "registry validation command"),
        ("check-boundary-registry-receipt.py", "independent registry receipt command"),
        ("upgrade-typed-corehir-v1-scalar-v3.py", "raw-to-canonical source upgrade"),
        ("convert-typed-corehir-v7.py", "Phase 7 source converter"),
        ("write-smt-vcs-v7.py", "Phase 7 SMT writer"),
        ("--typed-corehir-canonical", "canonical source binding argument"),
        ("--typed-corehir-raw", "raw source toolchain argument"),
        ("--boundary-registry", "source binding registry argument"),
        ("--boundary-registry-validation-receipt", "source binding receipt argument"),
        ("write-source-proof-binding.py", "source binding writer invocation"),
        ("run-proof-solver.py", "solver invocation"),
        ("--solver-result-output", "complete SolverResult output"),
        ("check-proof-required-release.py", "release authorization invocation"),
    ):
        require(release_command, token, label)
    require_order(release_command, "check-boundary-registry.py", "check-boundary-registry-receipt.py", "registry validation sequence")
    require_order(release_command, "upgrade-typed-corehir-v1-scalar-v3.py", "convert-typed-corehir-v7.py", "source upgrade sequence")
    require_order(release_command, "convert-typed-corehir-v7.py", "write-smt-vcs-v7.py", "v7 translation sequence")
    require_order(release_command, "write-smt-vcs-v7.py", "write-source-proof-binding.py", "solver-input-to-binding sequence")
    require_order(release_command, "check-boundary-registry-receipt.py", "write-source-proof-binding.py", "registry-to-source-binding sequence")
    require_order(release_command, "write-source-proof-binding.py", "prepare-proof-release-toolchain-v7.py", "binding-to-toolchain sequence")
    require_order(release_command, "prepare-proof-release-toolchain-v7.py", "run-proof-solver.py", "toolchain-to-solver sequence")

    for text, label in (
        (binding_writer, "source binding writer"),
        (release_checker, "release checker"),
    ):
        require(text, '"typed_corehir_canonical"', f"{label} canonical source path map")
        require(text, '"boundary_registry"', f"{label} registry path map")
        require(text, '"boundary_registry_validation_receipt"', f"{label} validation receipt path map")

    for token, label in (
        ('"source-artifact-binding", "5"', "source binding v5 TrustManifest component"),
        ('"proof-source-upgrader", "2"', "v1 to v3 proof upgrader TrustManifest component"),
        ('"proof-source-converter-cli", "7"', "v7 converter CLI TrustManifest component"),
        ('"typed-smt-adapter-cli", "7"', "v7 SMT CLI TrustManifest component"),
        ('"major-boundary-registry"', "registry TrustManifest component"),
        ('"major-boundary-registry-validator"', "registry validator TrustManifest component"),
        ('"major-boundary-receipt-validator"', "receipt validator TrustManifest component"),
        ('"major-boundary-receipt-checker"', "receipt checker TrustManifest component"),
    ):
        require(toolchain_writer, token, label)
    if "source-proof-binding-v4" in toolchain_writer or '"source-artifact-binding", "4"' in toolchain_writer:
        raise ValueError("legacy source-artifact-binding v4 remains in release toolchain")

    for workflow, label in (
        (release_workflow, "release workflow"),
        (proof_workflow, "proof workflow"),
    ):
        require(workflow, "release/boundary-registry.json", f"{label} registry retention")
        require(workflow, ".build/proof/boundary-registry-validation.json", f"{label} validation receipt retention")
        require(workflow, "test_boundary_registry", f"{label} registry negative tests")
        require(workflow, "solver-result.json", f"{label} complete solver result")

    for token, label in (
        ("check-boundary-registry.py", "registry workflow generator"),
        ("check-boundary-registry-receipt.py", "registry workflow independent checker"),
        ("check-versioned-boundary-enforcement.py", "registry workflow integration checker"),
        ("release/boundary-registry.json", "registry workflow artifact"),
        ("boundary-registry-validation.json", "registry workflow receipt artifact"),
    ):
        require(registry_workflow, token, label)

    for token, label in (
        ('"release/boundary-registry.json"', "release enforcement registry binding"),
        ('"scripts/proof/boundary_registry.py"', "release enforcement registry validator binding"),
        ('"scripts/proof/boundary_registry_receipt.py"', "release enforcement receipt validator binding"),
        ('"scripts/check/check-versioned-boundary-enforcement.py"', "release enforcement integration checker binding"),
        ('"scripts/tests/test_boundary_registry.py"', "release enforcement negative tests binding"),
        ('"scripts/tests/test_source_proof_binding.py"', "release enforcement binding-v5 tests"),
        ('"scripts/proof/typed_corehir_v1_scalar_v3.py"', "raw-to-canonical upgrader binding"),
        ('"scripts/gen/convert-typed-corehir-v7.py"', "v7 converter CLI binding"),
        ('"scripts/gen/write-smt-vcs-v7.py"', "v7 SMT CLI binding"),
        ("pinned major boundary registry and validation receipt", "authorization binding declaration"),
    ):
        require(enforcement_receipt, token, label)

    print(
        "versioned-boundary-enforcement: PASS: "
        f"boundaries={len(REQUIRED_BOUNDARIES)} binding=v{BINDING_VERSION}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"versioned-boundary-enforcement: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
