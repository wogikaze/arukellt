#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError as exc:
    raise SystemExit("PyYAML is required to validate Agent Skills") from exc

ROOT = Path(__file__).resolve().parent
NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
errors: list[str] = []
names: set[str] = set()

for skill_dir in sorted(p for p in ROOT.iterdir() if p.is_dir()):
    path = skill_dir / "SKILL.md"
    if not path.is_file():
        errors.append(f"{skill_dir.name}: missing SKILL.md")
        continue
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        errors.append(f"{path}: missing YAML frontmatter")
        continue
    try:
        _, raw, body = text.split("---\n", 2)
        metadata = yaml.safe_load(raw) or {}
    except Exception as exc:  # noqa: BLE001
        errors.append(f"{path}: invalid frontmatter: {exc}")
        continue

    name = metadata.get("name")
    description = metadata.get("description")
    if name != skill_dir.name:
        errors.append(f"{path}: name must match parent directory ({skill_dir.name})")
    if not isinstance(name, str) or len(name) > 64 or not NAME_RE.fullmatch(name):
        errors.append(f"{path}: invalid name")
    if name in names:
        errors.append(f"{path}: duplicate name {name}")
    names.add(name)
    if not isinstance(description, str) or not (1 <= len(description) <= 1024):
        errors.append(f"{path}: description must be 1..1024 characters")
    if not body.strip():
        errors.append(f"{path}: empty instructions")

legacy_files = list(ROOT.parent.glob("*/SKILL-ja.md"))
if legacy_files:
    errors.append("legacy SKILL-ja.md files remain: " + ", ".join(map(str, legacy_files)))

if errors:
    print("Agent Skills validation failed:")
    for error in errors:
        print(f"- {error}")
    sys.exit(1)

print(f"Agent Skills validation passed: {len(names)} skills")
