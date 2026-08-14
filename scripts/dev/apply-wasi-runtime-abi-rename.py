#!/usr/bin/env python3
"""One-shot final lifecycle transition for #076/#676/#819/#841."""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
ISSUES = {
    "076": "076-wasi-p2-filesystem.md",
    "676": "676-std-host-fs-env-process-polish.md",
    "819": "819-runtime-abi-core-op-lowering.md",
    "841": "841-wit-network-real-wasi-abi.md",
}
GATES = [
    "gate-076-wasi-p2-filesystem.py",
    "gate-676-std-host-fs-env-process.py",
    "gate-819-runtime-abi-core-op-lowering.py",
    "gate-841-real-wasi-network-abi.py",
]

for gate in GATES:
    subprocess.run([sys.executable, str(ROOT / "scripts/check" / gate)], cwd=ROOT, check=True)

receipt = """

## Close receipt — 2026-08-14

- Dedicated close gate: PASS
- `python3 scripts/manager.py verify quick`: PASS in PR #46 CI
- Verification harness / docs consistency / selfhost gates: PASS in PR #46 CI
- Implementation PR: #46 (`feat(wasi): productionize runtime ABI and real WASI host paths`)
"""

for issue_id, filename in ISSUES.items():
    src = ROOT / "issues/open" / filename
    dst = ROOT / "issues/done" / filename
    if dst.exists():
        continue
    if not src.exists():
        raise SystemExit(f"missing issue source: {src}")
    original = src.read_text(encoding="utf-8")
    text = original.replace("Status: open", "Status: done", 1)
    if "Updated:" in text:
        lines = text.splitlines()
        lines = ["Updated: 2026-08-14" if line.startswith("Updated:") else line for line in lines]
        text = "\n".join(lines)
        if original.endswith("\n"):
            text += "\n"
    text = text.replace("- [ ]", "- [x]")
    if "## Close receipt — 2026-08-14" not in text:
        text = text.rstrip() + receipt + "\n"
    dst.write_text(text, encoding="utf-8")
    src.unlink()

subprocess.run([sys.executable, str(ROOT / "scripts/gen/generate-issue-index.py")], cwd=ROOT, check=True)
