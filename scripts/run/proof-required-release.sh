#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
: "${GITHUB_SHA:?GITHUB_SHA is required}"
: "${GITHUB_REF:?GITHUB_REF is required}"
: "${GITHUB_REF_NAME:?GITHUB_REF_NAME is required}"
: "${GITHUB_WORKFLOW:?GITHUB_WORKFLOW is required}"
: "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"

case "$GITHUB_REF" in
  refs/tags/v*) ;;
  *)
    echo "proof-required-release: FAIL: expected refs/tags/v*, got $GITHUB_REF" >&2
    exit 1
    ;;
esac

CHECKED_OUT_SHA="$(git rev-parse HEAD)"
if [[ "$CHECKED_OUT_SHA" != "$GITHUB_SHA" ]]; then
  echo "proof-required-release: FAIL: checkout $CHECKED_OUT_SHA != event $GITHUB_SHA" >&2
  exit 1
fi
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "proof-required-release: FAIL: tracked working tree is dirty" >&2
  exit 1
fi

PROOF_ROOT=".build/typed-contract-proof"
TOOLCHAIN_ROOT=".build/proof-release-toolchain"
RELEASE_ROOT=".build/proof-required-release"
RUNTIME=".build/selfhost/arukellt-s2-runtime.wasm"
RAW_TYPED_COREHIR="$PROOF_ROOT/typed-corehir.json"
CANONICAL_TYPED_COREHIR="$PROOF_ROOT/typed-corehir-v3.json"
VERIFIED_CORE_MACHINE="$PROOF_ROOT/verified-core-machine.json"
VERIFIED_CORE="$PROOF_ROOT/verified-core.json"
SOLVER_INPUT="$PROOF_ROOT/verified-core-vcs.smt2"
PAYLOAD="$RELEASE_ROOT/arukellt-s2-runtime.wasm"
PROVENANCE="$RELEASE_ROOT/release-provenance.json"
PAYLOAD_MANIFEST="$RELEASE_ROOT/release-payload-manifest.json"
AUTHORIZATION="$RELEASE_ROOT/release-authorization.json"
POLICY="$PROOF_ROOT/release-policy.json"
BOUNDARY_REGISTRY="release/boundary-registry.json"
BOUNDARY_REGISTRY_RECEIPT=".build/proof/boundary-registry-validation.json"
PHASE5_BOUNDARY=".build/proof/proof-phase5-boundary.json"
PHASE6_BOUNDARY=".build/proof/proof-phase6-boundary.json"
PHASE7_BOUNDARY=".build/proof/proof-phase7-boundary.json"

rm -rf "$TOOLCHAIN_ROOT" "$RELEASE_ROOT"
mkdir -p "$PROOF_ROOT" "$TOOLCHAIN_ROOT" "$RELEASE_ROOT" .build/proof

# The major-boundary inventory is pinned to immutable commits and independently
# revalidated before any local architecture evidence or solver process runs.
python3 scripts/check/check-boundary-registry.py \
  --registry "$BOUNDARY_REGISTRY" \
  --repository "$GITHUB_REPOSITORY" \
  --receipt-output "$BOUNDARY_REGISTRY_RECEIPT"
python3 scripts/check/check-boundary-registry-receipt.py \
  --registry "$BOUNDARY_REGISTRY" \
  --receipt "$BOUNDARY_REGISTRY_RECEIPT" \
  --repository "$GITHUB_REPOSITORY"

# Architecture evidence must be generated from this exact checkout.
python3 scripts/check/check-backend-name-lookup-audit.py
python3 scripts/check/check-backend-typeid-first.py
python3 scripts/gen/write-backend-typeid-layout-receipt.py
python3 scripts/check/check-gc-hint-translation-validation.py
python3 scripts/gen/write-mir-opt-translation-registry.py
python3 scripts/check/check-corehir-body-boundary.py
python3 scripts/gen/write-corehir-body-boundary-receipt.py
python3 scripts/gen/write-proof-phase5-boundary-receipt.py --output "$PHASE5_BOUNDARY"
python3 scripts/check/check-proof-phase5-boundary.py "$PHASE5_BOUNDARY"
python3 scripts/gen/write-proof-phase6-boundary-receipt.py --output "$PHASE6_BOUNDARY"
python3 scripts/check/check-proof-phase6-boundary.py "$PHASE6_BOUNDARY"
python3 scripts/gen/write-proof-phase7-boundary-receipt.py --output "$PHASE7_BOUNDARY"
python3 scripts/check/check-proof-phase7-boundary.py "$PHASE7_BOUNDARY"

python3 - <<'PY'
import sys
from pathlib import Path

sys.path.insert(0, "scripts")
from selfhost.checks import rebuild_current_s2

path, error, elapsed = rebuild_current_s2(Path.cwd(), force=True)
if path is None:
    raise SystemExit(error or "stage-2 build failed")
expected = Path(".build/selfhost/arukellt-s2-runtime.wasm").resolve()
if path.resolve() != expected:
    raise SystemExit(f"unexpected stage-2 runtime path: {path}")
print(f"proof-release-s2: PASS: path={path} elapsed={elapsed:.3f}s")
PY

test -s "$RUNTIME"
ARUKELLT_SELFHOST_WASM="$RUNTIME" \
  python3 scripts/check/check-typed-contract-emission.py

# The producer still emits frozen TypedCoreHIR v1.  Upgrade only the strictly
# admitted scalar subset, then stay in the Phase 6/7 machine/read-only profile.
python3 scripts/gen/upgrade-typed-corehir-v1-scalar-v3.py \
  --input "$RAW_TYPED_COREHIR" \
  --output "$CANONICAL_TYPED_COREHIR"
python3 scripts/gen/convert-typed-corehir-v7.py \
  --input "$CANONICAL_TYPED_COREHIR" \
  --output "$VERIFIED_CORE_MACHINE"
cp "$VERIFIED_CORE_MACHINE" "$VERIFIED_CORE"
cmp "$VERIFIED_CORE_MACHINE" "$VERIFIED_CORE"
python3 scripts/gen/write-smt-vcs-v7.py \
  --subject "$VERIFIED_CORE" \
  --output "$SOLVER_INPUT"

cp "$RUNTIME" "$PAYLOAD"
python3 scripts/gen/write-release-provenance.py \
  --repository "$GITHUB_REPOSITORY" \
  --commit "$GITHUB_SHA" \
  --ref-type tag \
  --ref-name "$GITHUB_REF_NAME" \
  --workflow "$GITHUB_WORKFLOW" \
  --run-id "$GITHUB_RUN_ID" \
  --output "$PROVENANCE"
python3 scripts/gen/write-release-payload-manifest.py \
  --payload "arukellt-wasm=$PAYLOAD" \
  --output "$PAYLOAD_MANIFEST"

python3 scripts/gen/write-source-proof-binding.py \
  --source tests/verified-core/contract_identity.ark \
  --producer-executable "$RUNTIME" \
  --typed-corehir "$RAW_TYPED_COREHIR" \
  --typed-corehir-canonical "$CANONICAL_TYPED_COREHIR" \
  --verified-core-machine "$VERIFIED_CORE_MACHINE" \
  --verified-core-normalized "$VERIFIED_CORE" \
  --solver-input "$SOLVER_INPUT" \
  --boundary-registry "$BOUNDARY_REGISTRY" \
  --boundary-registry-validation-receipt "$BOUNDARY_REGISTRY_RECEIPT" \
  --backend-typeid-layout-receipt .build/proof/backend-typeid-layout.json \
  --optimizer-translation-registry .build/proof/mir-opt-translation-registry.json \
  --corehir-body-boundary-receipt .build/proof/corehir-body-boundary.json \
  --release-provenance "$PROVENANCE" \
  --release-payload-manifest "$PAYLOAD_MANIFEST" \
  --output "$PROOF_ROOT/source-proof-binding.json"

Z3_BIN="${Z3_BIN:-$(command -v z3)}"
python3 scripts/gen/prepare-proof-release-toolchain-v7.py \
  --runtime "$RUNTIME" \
  --source-binding "$PROOF_ROOT/source-proof-binding.json" \
  --typed-corehir-raw "$RAW_TYPED_COREHIR" \
  --typed-corehir "$CANONICAL_TYPED_COREHIR" \
  --phase6-boundary "$PHASE6_BOUNDARY" \
  --phase7-boundary "$PHASE7_BOUNDARY" \
  --output-dir "$TOOLCHAIN_ROOT" \
  --toolchain-output "$TOOLCHAIN_ROOT/toolchain.json" \
  --z3 "$Z3_BIN"

python3 scripts/run/run-proof-solver.py \
  --subject "$VERIFIED_CORE" \
  --solver-input "$SOLVER_INPUT" \
  --toolchain "$TOOLCHAIN_ROOT/toolchain.json" \
  --solver-output "$PROOF_ROOT/solver-output.txt" \
  --trust-manifest-output "$PROOF_ROOT/trust-manifest.json" \
  --proof-receipt-output "$PROOF_ROOT/proof-receipt.json" \
  --solver-result-output "$PROOF_ROOT/solver-result.json"

python3 scripts/gen/write-proof-required-release-policy.py --output "$POLICY"
python3 scripts/check/check-proof-required-release.py \
  "$POLICY" \
  --source-binding "$PROOF_ROOT/source-proof-binding.json" \
  --source tests/verified-core/contract_identity.ark \
  --producer-executable "$RUNTIME" \
  --typed-corehir "$RAW_TYPED_COREHIR" \
  --typed-corehir-canonical "$CANONICAL_TYPED_COREHIR" \
  --verified-core-machine "$VERIFIED_CORE_MACHINE" \
  --verified-core-normalized "$VERIFIED_CORE" \
  --solver-input "$SOLVER_INPUT" \
  --boundary-registry "$BOUNDARY_REGISTRY" \
  --boundary-registry-validation-receipt "$BOUNDARY_REGISTRY_RECEIPT" \
  --backend-typeid-layout-receipt .build/proof/backend-typeid-layout.json \
  --optimizer-translation-registry .build/proof/mir-opt-translation-registry.json \
  --corehir-body-boundary-receipt .build/proof/corehir-body-boundary.json \
  --release-provenance "$PROVENANCE" \
  --release-payload-manifest "$PAYLOAD_MANIFEST" \
  --release-payload "arukellt-wasm=$PAYLOAD" \
  --expected-repository "$GITHUB_REPOSITORY" \
  --expected-commit "$GITHUB_SHA" \
  --expected-tag "$GITHUB_REF_NAME" \
  --authorization-output "$AUTHORIZATION"

test -s "$PROOF_ROOT/solver-result.json"
test -s "$AUTHORIZATION"
echo "proof-required-release: AUTHORIZED: tag=$GITHUB_REF_NAME commit=$GITHUB_SHA payload=$PAYLOAD"
