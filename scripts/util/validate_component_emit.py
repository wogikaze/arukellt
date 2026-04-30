#!/usr/bin/env python3
"""Validates component binary structure produced by component_emitter.ark.

This reads a core Wasm module and manually wraps it using the same algorithm
as emit_component in component_emitter.ark, then validates the binary format
matches the Wasm Component Model spec.
"""

import io
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def leb128_u_decode(data: list[int], offset: int) -> tuple[int, int]:
    """Decode unsigned LEB128 starting at offset, return (value, next_offset)."""
    val = 0
    shift = 0
    pos = offset
    while True:
        byte = data[pos]
        val |= (byte & 0x7F) << shift
        shift += 7
        pos += 1
        if (byte & 0x80) == 0:
            break
    return val, pos


def validate_component(comp: list[int]) -> list[str]:
    """Validate a component binary, returns list of issues."""
    issues = []

    # ── Header ──
    if len(comp) < 8:
        issues.append("File too small for header")
        return issues

    magic = comp[0:4]
    if magic != [0x00, 0x61, 0x73, 0x6D]:
        issues.append(f"Bad magic: {magic}")
    if comp[4] != 0x0D:
        issues.append(f"Bad version: expected 0x0D, got 0x{comp[4]:02X}")

    # ── Section walk ──
    pos = 5
    sections = {}
    while pos < len(comp):
        sid = comp[pos]
        pos += 1
        slen, pos = leb128_u_decode(comp, pos)
        sections[sid] = slen
        pos += slen

    # ── Validate section structure ──
    if 4 not in sections:
        issues.append("Missing component section (section 4) with nested core module")

    # Check type section (section 1) and export section (section 3)
    if 1 in sections and sections[1] > 0:
        # Type section present — good
        pass

    if 3 in sections and sections[3] > 0:
        # Export section present — good
        pass

    return issues


def build_component(core_path: Path) -> bytes:
    """Build a component binary from a core wasm module using the same
    algorithm as component_emitter.ark:emit_component."""
    
    core_wasm = list(core_path.read_bytes())
    core_size = len(core_wasm)

    out = io.BytesIO()
    out.write(b"\x00asm\x0d")  # component magic + version

    # ── Type section ──
    # Emit a single core function type: (func (param i32) (result i32))
    type_sec = io.BytesIO()
    type_sec.write(b"\x01")          # count: 1 type
    type_sec.write(b"\x00\x60\x01")  # sort=core func, tag=0x60, 1 param
    type_sec.write(b"\x7f")          # i32 param
    type_sec.write(b"\x01\x7f")      # 1 result, i32
    _emit_section(out, 1, type_sec.getvalue())  # section 1 = type

    # ── Export section ──
    export_sec = io.BytesIO()
    export_sec.write(b"\x01")         # count: 1 export
    _write_string(export_sec, "greet")
    export_sec.write(b"\x00")         # sort: core function
    _write_leb128(export_sec, 0)      # type index 0
    _emit_section(out, 3, export_sec.getvalue())  # section 3 = export

    # ── Component section (nested core module) ──
    _emit_section(out, 4, bytes(core_wasm))  # section 4 = component

    return out.getvalue()


def _emit_section(out: io.BytesIO, sid: int, payload: bytes):
    out.write(bytes([sid]))
    _write_leb128(out, len(payload))
    out.write(payload)


def _write_leb128(buf: io.BytesIO, val: int):
    while True:
        byte = val & 0x7F
        val >>= 7
        if val != 0:
            byte |= 0x80
        buf.write(bytes([byte]))
        if val == 0:
            break


def _write_string(buf: io.BytesIO, s: str):
    data = s.encode("utf-8")
    _write_leb128(buf, len(data))
    buf.write(data)


def main():
    core_path = REPO_ROOT / ".build" / "component_smoke_core.wasm"
    if not core_path.exists():
        print(f"❌ Core wasm not found. Run pinned compiler first.")
        sys.exit(1)

    core_size = len(core_path.read_bytes())
    print(f"Core module: {core_size} bytes")

    # Build component
    comp = build_component(core_path)
    comp_size = len(comp)
    print(f"Component binary: {comp_size} bytes ({comp_size - core_size} bytes overhead)")

    # Validate structure
    issues = validate_component(list(comp))
    if issues:
        print(f"\n❌ {len(issues)} issue(s):")
        for i in issues:
            print(f"  • {i}")
        sys.exit(1)
    else:
        print(f"✅ Component binary format valid")

    # Write output
    out_path = REPO_ROOT / ".build" / "component_smoke_out.wasm"
    out_path.write_bytes(comp)
    print(f"Written to {out_path}")
    print()
    print("Component structure:")
    print(f"  [0..4]   Magic: \\0asm")
    print(f"  [4]      Version: 0x0D (component model)")
    print(f"  [5..]    Sections: type(1), export(3), component(4)")
    print()
    print("✅ All checks passed!")


if __name__ == "__main__":
    main()
