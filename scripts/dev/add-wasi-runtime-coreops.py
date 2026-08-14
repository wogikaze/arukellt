#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
path = root / "scripts/check/check-false-done-close-gates.py"
text = path.read_text(encoding="utf-8")
text = text.replace(
    '"076": ["P2 filesystem fixture in-tree compile + wasm-tools validate (runtime I/O tracked by #076)"],',
    '"076": ["real-WASI P2 filesystem production gate (gate-076-wasi-p2-filesystem.py)"],',
)
pattern = re.compile(r'def gate_076\(\) -> tuple\[int, str\]:\n.*?\n\ndef _gate_076_locked\(\)', re.S)
replacement = '''def gate_076() -> tuple[int, str]:
    entry = "component-compile:wasi_fs_p2.ark"
    if not _manifest_contains(entry):
        return 1, f"manifest missing {entry}"
    script = REPO_ROOT / "scripts" / "check" / "gate-076-wasi-p2-filesystem.py"
    if not script.is_file():
        return 1, "missing scripts/check/gate-076-wasi-p2-filesystem.py"
    result = subprocess.run(
        [sys.executable, str(script)],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        return 1, (result.stdout + result.stderr)[-800:]
    return 0, ""


def _gate_076_locked()'''
text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit("gate_076 replacement anchor not found")
path.write_text(text, encoding="utf-8")
