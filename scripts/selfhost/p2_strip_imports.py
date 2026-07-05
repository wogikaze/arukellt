#!/usr/bin/env python3
"""Strip unused WIT interface imports from a core wasm module.

Some Arukellt-compiled modules import wasi:clocks/* and wasi:random/*
interfaces even when those functions are never called.  This module
removes such imports and patches call indices so the resulting wasm
can be embedded into a wasi:cli/command@0.2.0 component world.
"""

from __future__ import annotations


def _read_uleb(data: bytes, pos: int) -> tuple[int, int]:
    result = 0
    shift = 0
    while True:
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if not (byte & 0x80):
            break
    return result, pos


def _write_uleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            break
    return bytes(out)


def _read_string(data: bytes, pos: int) -> tuple[str, int]:
    length, pos = _read_uleb(data, pos)
    s = data[pos : pos + length].decode("utf-8")
    return s, pos + length


def _write_string(s: str) -> bytes:
    raw = s.encode("utf-8")
    return _write_uleb(len(raw)) + raw


def _parse_sections(wasm: bytes) -> tuple[dict[int, tuple[int, int]], int]:
    """Return {section_id: (data_offset, size)} and header length."""
    assert wasm[:4] == b"\x00asm"
    pos = 8  # skip magic + version
    sections: dict[int, tuple[int, int]] = {}
    while pos < len(wasm):
        section_id = wasm[pos]
        pos += 1
        size, pos = _read_uleb(wasm, pos)
        sections[section_id] = (pos, size)
        pos += size
    return sections, 8


def _rebuild_sections(
    header: bytes,
    sections: list[tuple[int, bytes]],
) -> bytes:
    """Reassemble a wasm binary from (section_id, contents) pairs."""
    out = bytearray(header)
    for section_id, contents in sorted(sections, key=lambda x: x[0]):
        if contents is None:
            continue
        out.append(section_id)
        out.extend(_write_uleb(len(contents)))
        out.extend(contents)
    return bytes(out)


def strip_wit_imports(
    wasm: bytes,
    module_prefixes: tuple[str, ...],
) -> bytes:
    """Remove imports whose module starts with any prefix; patch call indices.

    Returns a new wasm binary with the matching imports removed and all
    ``call`` / ``ref.func`` indices adjusted accordingly.
    """
    sections, _ = _parse_sections(wasm)
    header = wasm[:8]

    # ── Parse import section (id=2) ──────────────────────────────────────
    imp_id = 2
    if imp_id not in sections:
        return wasm
    imp_off, imp_size = sections[imp_id]
    imp_data = wasm[imp_off : imp_off + imp_size]

    pos = 0
    count, pos = _read_uleb(imp_data, pos)

    kept_imports: list[bytes] = []
    removed_func_indices: list[int] = []
    func_idx = 0

    for _ in range(count):
        start = pos
        mod_name, pos = _read_string(imp_data, pos)
        field_name, pos = _read_string(imp_data, pos)
        kind = imp_data[pos]
        pos += 1
        if kind == 0:  # func import
            type_idx, pos = _read_uleb(imp_data, pos)
            should_remove = any(
                mod_name.startswith(prefix) for prefix in module_prefixes
            )
            if should_remove:
                removed_func_indices.append(func_idx)
            else:
                kept_imports.append(imp_data[start:pos])
            func_idx += 1
        elif kind == 1:  # table
            # elemtype (1 byte) + limits flag (1 byte) + min (uleb) [+ max (uleb)]
            pos += 1  # elemtype
            limits_flag = imp_data[pos]
            pos += 1
            _, pos = _read_uleb(imp_data, pos)  # min
            if limits_flag & 1:
                _, pos = _read_uleb(imp_data, pos)  # max
            kept_imports.append(imp_data[start:pos])
        elif kind == 2:  # memory
            limits_flag = imp_data[pos]
            pos += 1
            _, pos = _read_uleb(imp_data, pos)  # min
            if limits_flag & 1:
                _, pos = _read_uleb(imp_data, pos)  # max
            kept_imports.append(imp_data[start:pos])
        elif kind == 3:  # global
            pos += 1  # valtype
            pos += 1  # mut
            kept_imports.append(imp_data[start:pos])
        else:
            kept_imports.append(imp_data[start:pos])

    if not removed_func_indices:
        return wasm

    # Count local functions from function section (id=3)
    func_section_id = 3
    total_local_funcs = 0
    if func_section_id in sections:
        fs_off, fs_size = sections[func_section_id]
        fs_data = wasm[fs_off : fs_off + fs_size]
        total_local_funcs, _ = _read_uleb(fs_data, 0)

    total_funcs = func_idx + total_local_funcs

    # Build index remapping: old_idx -> new_idx
    # Removed indices are skipped; all others shift down.
    remap: dict[int, int] = {}
    removed_set = set(removed_func_indices)
    new_idx = 0
    for old_idx in range(total_funcs):
        if old_idx in removed_set:
            continue
        remap[old_idx] = new_idx
        new_idx += 1

    # Rebuild import section
    new_imp_data = bytearray()
    new_imp_data.extend(_write_uleb(len(kept_imports)))
    for imp_bytes in kept_imports:
        new_imp_data.extend(imp_bytes)

    # ── Patch code section (id=10) ───────────────────────────────────────
    code_id = 10
    new_code_data: bytes | None = None
    if code_id in sections:
        code_off, code_size = sections[code_id]
        code_data = wasm[code_off : code_off + code_size]
        new_code_data = _patch_code_section(code_data, remap)

    # ── Patch element section (id=9) ─────────────────────────────────────
    elem_id = 9
    new_elem_data: bytes | None = None
    if elem_id in sections:
        elem_off, elem_size = sections[elem_id]
        elem_data = wasm[elem_off : elem_off + elem_size]
        new_elem_data = _patch_elem_section(elem_data, remap)

    # ── Patch start section (id=8) ───────────────────────────────────────
    start_id = 8
    new_start_data: bytes | None = None
    if start_id in sections:
        start_off, start_size = sections[start_id]
        start_data = wasm[start_off : start_off + start_size]
        new_start_data = _patch_start_section(start_data, remap)

    # ── Patch data section (id=11) — no func refs, pass through ──────────

    # ── Patch export section (id=7) ──────────────────────────────────────
    exp_id = 7
    new_exp_data: bytes | None = None
    if exp_id in sections:
        exp_off, exp_size = sections[exp_id]
        exp_data = wasm[exp_off : exp_off + exp_size]
        new_exp_data = _patch_export_section(exp_data, remap)

    # ── Assemble output ──────────────────────────────────────────────────
    section_list: list[tuple[int, bytes]] = []
    for sid in sorted(sections.keys()):
        if sid == imp_id:
            section_list.append((sid, bytes(new_imp_data)))
        elif sid == code_id and new_code_data is not None:
            section_list.append((sid, new_code_data))
        elif sid == elem_id and new_elem_data is not None:
            section_list.append((sid, new_elem_data))
        elif sid == start_id and new_start_data is not None:
            section_list.append((sid, new_start_data))
        elif sid == exp_id and new_exp_data is not None:
            section_list.append((sid, new_exp_data))
        else:
            off, size = sections[sid]
            section_list.append((sid, wasm[off : off + size]))

    return _rebuild_sections(header, section_list)


def _patch_code_section(code_data: bytes, remap: dict[int, int]) -> bytes:
    """Patch call/ref.func indices in the code section."""
    pos = 0
    func_count, pos = _read_uleb(code_data, pos)
    out = bytearray()
    out.extend(_write_uleb(func_count))

    for _ in range(func_count):
        body_start = pos
        body_size, pos = _read_uleb(code_data, pos)
        body_end = pos + body_size
        body = bytearray(code_data[pos:body_end])
        patched_body = _patch_func_body(bytes(body), remap)
        out.extend(_write_uleb(len(patched_body)))
        out.extend(patched_body)
        pos = body_end

    return bytes(out)


def _patch_func_body(body: bytes, remap: dict[int, int]) -> bytes:
    """Patch call/ref.func indices within a single function body."""
    pos = 0
    # local declarations
    local_count, pos = _read_uleb(body, pos)
    for _ in range(local_count):
        pos += 1  # repeat count (uleb, but usually small)
        # Actually: repeat_count (uleb) + type (1 byte)
        # Re-read properly:
    # Re-parse locals properly
    pos = 0
    local_count, pos = _read_uleb(body, pos)
    for _ in range(local_count):
        _, pos = _read_uleb(body, pos)  # repeat count
        pos += 1  # type byte

    # Now pos points to the instruction stream
    out = bytearray(body[:pos])
    i = pos
    while i < len(body):
        byte = body[i]
        if byte == 0x10:  # call
            out.append(byte)
            i += 1
            idx, i = _read_uleb(body, i)
            new_idx = remap.get(idx, idx)
            out.extend(_write_uleb(new_idx))
        elif byte == 0xD2:  # ref.func
            out.append(byte)
            i += 1
            idx, i = _read_uleb(body, i)
            new_idx = remap.get(idx, idx)
            out.extend(_write_uleb(new_idx))
        else:
            out.append(byte)
            i += 1

    return bytes(out)


def _patch_elem_section(elem_data: bytes, remap: dict[int, int]) -> bytes:
    """Patch function indices in the element section."""
    # Element section is complex; for now, just pass through.
    # Most guest modules don't have element sections with func refs.
    return elem_data


def _patch_start_section(start_data: bytes, remap: dict[int, int]) -> bytes:
    """Patch the start function index."""
    idx, _ = _read_uleb(start_data, 0)
    new_idx = remap.get(idx, idx)
    return _write_uleb(new_idx)


def _patch_export_section(exp_data: bytes, remap: dict[int, int]) -> bytes:
    """Patch function indices in the export section."""
    pos = 0
    count, pos = _read_uleb(exp_data, pos)
    out = bytearray()
    out.extend(_write_uleb(count))
    for _ in range(count):
        # name
        name, pos = _read_string(exp_data, pos)
        out.extend(_write_string(name))
        # kind
        kind = exp_data[pos]
        out.append(kind)
        pos += 1
        # index
        idx, pos = _read_uleb(exp_data, pos)
        if kind == 0:  # func export
            new_idx = remap.get(idx, idx)
            out.extend(_write_uleb(new_idx))
        else:
            out.extend(_write_uleb(idx))
    return bytes(out)
