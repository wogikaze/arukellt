#!/usr/bin/env python3
"""Resolve and compose component-model dependencies declared in ark.toml (#674)."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class ComponentDependency:
    name: str
    source: Path
    cached: Path
    sha256: str
    wit: str
    world: str | None


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _manifest(path: Path) -> dict:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ValueError(f"unable to read manifest {path}: {exc}") from exc


def _component_candidate(root: Path, name: str, spec: dict) -> Path:
    raw = spec.get("component")
    if isinstance(raw, str) and raw:
        candidate = root / raw
        return candidate if candidate.is_absolute() else candidate.resolve()
    path_value = spec.get("path")
    if not isinstance(path_value, str) or not path_value:
        raise ValueError(f"dependency {name!r} must declare path = ...")
    base = (root / path_value).resolve()
    if base.is_file():
        return base
    leaf = name.split(":")[-1].replace("/", "-")
    candidates = [
        base / "component.wasm",
        base / f"{leaf}.component.wasm",
        base / "mod.component.wasm",
    ]
    existing = [p for p in candidates if p.is_file()]
    if not existing and base.is_dir():
        existing = sorted(base.glob("*.component.wasm"))
    if len(existing) == 1:
        return existing[0]
    if len(existing) > 1:
        raise ValueError(f"dependency {name!r} has multiple component wasm artifacts; set component = ...")
    return candidates[0]


def _extract_wit(component: Path) -> str:
    wasm_tools = shutil.which("wasm-tools")
    if wasm_tools:
        run = subprocess.run(
            [wasm_tools, "component", "wit", str(component)],
            capture_output=True,
            text=True,
            timeout=60,
        )
        if run.returncode == 0 and run.stdout.strip():
            return run.stdout
    for sidecar in (
        component.with_suffix(".wit"),
        component.parent / "mod.wit",
        component.parent / "interface.wit",
    ):
        if sidecar.is_file():
            return sidecar.read_text(encoding="utf-8")
    raise ValueError(
        f"cannot extract WIT from {component}; install wasm-tools or provide mod.wit/interface.wit"
    )


def _package_from_wit(wit: str) -> str | None:
    for line in wit.splitlines():
        stripped = line.strip()
        if stripped.startswith("package "):
            value = stripped[len("package ") :].split(";", 1)[0].strip()
            return value.split("@", 1)[0]
    return None


def _validate_contract(name: str, spec: dict, wit: str) -> None:
    expected_package = spec.get("package")
    if not isinstance(expected_package, str) and ":" in name:
        expected_package = name.split("/", 1)[0]
    actual_package = _package_from_wit(wit)
    if isinstance(expected_package, str) and expected_package:
        expected_package = expected_package.split("@", 1)[0]
        if actual_package != expected_package:
            raise ValueError(
                f"dependency {name!r} package mismatch: expected {expected_package!r}, got {actual_package!r}"
            )
    world = spec.get("world")
    if isinstance(world, str) and world:
        tokens = (f"world {world} ", f"world {world}{{", f"world {world}\n")
        if not any(token in wit for token in tokens):
            raise ValueError(f"dependency {name!r} incompatible world: {world!r} not exported by component WIT")


def resolve(manifest: Path, cache_dir: Path | None = None, lock_path: Path | None = None) -> list[ComponentDependency]:
    manifest = manifest.resolve()
    data = _manifest(manifest)
    deps = data.get("dependencies", {})
    if not isinstance(deps, dict):
        raise ValueError("[dependencies] must be a table")
    root = manifest.parent
    cache = (cache_dir or root / ".build" / "components").resolve()
    lock = (lock_path or root / "ark.lock").resolve()
    cache.mkdir(parents=True, exist_ok=True)
    resolved: list[ComponentDependency] = []
    for name in sorted(deps):
        spec = deps[name]
        if isinstance(spec, str):
            raise ValueError(f"dependency {name!r} uses unresolved version {spec!r}; component dependencies require a path")
        if not isinstance(spec, dict):
            raise ValueError(f"dependency {name!r} must be an inline table")
        component = _component_candidate(root, name, spec)
        if not component.is_file():
            raise ValueError(f"dependency {name!r} component wasm is missing: {component}")
        wit = _extract_wit(component)
        _validate_contract(name, spec, wit)
        digest = _sha256(component)
        cached = cache / f"{digest}.component.wasm"
        if not cached.exists():
            shutil.copy2(component, cached)
        world = spec.get("world") if isinstance(spec.get("world"), str) else None
        resolved.append(ComponentDependency(name, component, cached, digest, wit, world))
    payload = {
        "version": 1,
        "components": [
            {
                "name": dep.name,
                "path": dep.source.relative_to(root).as_posix() if dep.source.is_relative_to(root) else str(dep.source),
                "sha256": dep.sha256,
                "cache": dep.cached.relative_to(root).as_posix() if dep.cached.is_relative_to(root) else str(dep.cached),
                **({"world": dep.world} if dep.world else {}),
            }
            for dep in resolved
        ],
    }
    lock.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return resolved


def compose(manifest: Path, socket: Path, output: Path, cache_dir: Path | None = None) -> None:
    resolved = resolve(manifest, cache_dir=cache_dir)
    if not socket.is_file():
        raise ValueError(f"composition socket component is missing: {socket}")
    wac = shutil.which("wac")
    if not wac:
        raise ValueError("wac not found in PATH; binary composition requires wac plug")
    current = socket.resolve()
    temp_dir = (manifest.resolve().parent / ".build" / "compose").resolve()
    temp_dir.mkdir(parents=True, exist_ok=True)
    for index, dep in enumerate(resolved):
        target = output.resolve() if index == len(resolved) - 1 else temp_dir / f"stage-{index}.component.wasm"
        run = subprocess.run(
            [wac, "plug", "--plug", str(dep.cached), str(current), "-o", str(target)],
            capture_output=True,
            text=True,
            timeout=120,
        )
        if run.returncode != 0:
            raise ValueError(f"component composition failed for {dep.name}: {(run.stderr or run.stdout).strip()}")
        current = target
    if not resolved:
        shutil.copy2(current, output)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    resolve_p = sub.add_parser("resolve")
    resolve_p.add_argument("--manifest", default="ark.toml")
    resolve_p.add_argument("--cache-dir")
    compose_p = sub.add_parser("compose")
    compose_p.add_argument("--manifest", default="ark.toml")
    compose_p.add_argument("--socket", required=True)
    compose_p.add_argument("-o", "--output", required=True)
    compose_p.add_argument("--cache-dir")
    args = parser.parse_args()
    try:
        cache = Path(args.cache_dir) if args.cache_dir else None
        if args.command == "resolve":
            deps = resolve(Path(args.manifest), cache_dir=cache)
            for dep in deps:
                print(f"{dep.name}\t{dep.cached}\t{dep.sha256}")
        else:
            compose(Path(args.manifest), Path(args.socket), Path(args.output), cache_dir=cache)
    except ValueError as exc:
        print(f"component-deps: error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
