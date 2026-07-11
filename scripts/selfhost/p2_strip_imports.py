#!/usr/bin/env python3
"""Strip unused WIT interface imports from a core wasm module.

Some Arukellt-compiled modules import wasi:clocks/* and wasi:random/*
interfaces even when those functions are never called.  This module
removes such imports and replaces them with local stub functions that
preserve the original function indices, so the resulting wasm can be
embedded into a wasi:cli/command@0.2.0 component world without rewriting
call sites in the code section (which is unsafe for Wasm GC binaries).
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


def _skip_valtype(data: bytes, pos: int) -> int:
    byte = data[pos]
    pos += 1
    if byte in (0x64, 0x63):  # ref / ref.null with heap type
        if pos < len(data) and data[pos] < 0x40:
            _, pos = _read_uleb(data, pos)
    return pos


def _skip_comptype(data: bytes, pos: int) -> int:
    form = data[pos]
    pos += 1
    if form == 0x60:  # func
        param_count, pos = _read_uleb(data, pos)
        for _ in range(param_count):
            pos = _skip_valtype(data, pos)
        result_count, pos = _read_uleb(data, pos)
        for _ in range(result_count):
            pos = _skip_valtype(data, pos)
    elif form == 0x5e:  # array
        pos += 1  # mut
        pos = _skip_valtype(data, pos)
    elif form == 0x5f:  # struct
        field_count, pos = _read_uleb(data, pos)
        for _ in range(field_count):
            pos += 1  # mut
            pos = _skip_valtype(data, pos)
    return pos


def _func_type_results(type_data: bytes, type_idx: int) -> list[int]:
    """Return result valtype bytes for a func type at *type_idx*."""
    pos = 0
    count, pos = _read_uleb(type_data, pos)
    for idx in range(count):
        form = type_data[pos]
        pos += 1
        if form == 0x60:
            param_count, pos = _read_uleb(type_data, pos)
            for _ in range(param_count):
                pos = _skip_valtype(type_data, pos)
            result_count, pos = _read_uleb(type_data, pos)
            results = list(type_data[pos : pos + result_count])
            pos += result_count
            if idx == type_idx:
                return results
        elif form == 0x5e:
            pos += 1
            pos = _skip_valtype(type_data, pos)
        elif form == 0x5f:
            field_count, pos = _read_uleb(type_data, pos)
            for _ in range(field_count):
                pos += 1
                pos = _skip_valtype(type_data, pos)
        elif form in (0x4e, 0x4f):  # sub / sub final
            super_count, pos = _read_uleb(type_data, pos)
            for _ in range(super_count):
                _, pos = _read_uleb(type_data, pos)
            pos = _skip_comptype(type_data, pos)
        elif form == 0x50:
            pos = _skip_comptype(type_data, pos)
        else:
            raise ValueError(f"unsupported type form {form:#x}")
    raise KeyError(type_idx)


def _stub_func_body(results: list[int]) -> bytes:
    """Build a stub function body that returns zero/default for *results*."""
    body = bytearray()
    body.extend(_write_uleb(0))  # local decls
    for r in results:
        if r == 0x7F:  # i32
            body.extend(b"\x41\x00")
        elif r == 0x7E:  # i64
            body.extend(b"\x42\x00")
        elif r == 0x7D:  # f32
            body.extend(b"\x43\x00\x00\x00\x00")
        elif r == 0x7C:  # f64
            body.extend(b"\x44\x00\x00\x00\x00\x00\x00\x00\x00")
        else:
            raise ValueError(f"unsupported stub result type {r:#x}")
    body.append(0x0B)
    return bytes(body)


def _parse_import_entries(
    imp_data: bytes,
) -> list[tuple[str, str, int, bytes]]:
    """Return list of (module, field, type_idx, raw_entry_bytes) for func imports.

    Non-func imports are returned with type_idx=-1.
    """
    pos = 0
    count, pos = _read_uleb(imp_data, pos)
    entries: list[tuple[str, str, int, bytes]] = []
    for _ in range(count):
        start = pos
        mod_name, pos = _read_string(imp_data, pos)
        field_name, pos = _read_string(imp_data, pos)
        kind = imp_data[pos]
        pos += 1
        if kind == 0:  # func
            type_idx, pos = _read_uleb(imp_data, pos)
            entries.append((mod_name, field_name, type_idx, imp_data[start:pos]))
        elif kind == 1:  # table
            pos += 1  # elemtype
            limits_flag = imp_data[pos]
            pos += 1
            _, pos = _read_uleb(imp_data, pos)
            if limits_flag & 1:
                _, pos = _read_uleb(imp_data, pos)
            entries.append((mod_name, field_name, -1, imp_data[start:pos]))
        elif kind == 2:  # memory
            limits_flag = imp_data[pos]
            pos += 1
            _, pos = _read_uleb(imp_data, pos)
            if limits_flag & 1:
                _, pos = _read_uleb(imp_data, pos)
            entries.append((mod_name, field_name, -1, imp_data[start:pos]))
        elif kind == 3:  # global
            pos += 1  # valtype
            pos += 1  # mut
            entries.append((mod_name, field_name, -1, imp_data[start:pos]))
        else:
            entries.append((mod_name, field_name, -1, imp_data[start:pos]))
    return entries


def strip_wit_imports(
    wasm: bytes,
    module_prefixes: tuple[str, ...],
) -> bytes:
    """Remove matching imports and replace them with index-preserving stubs.

    Removed func imports must form a trailing contiguous suffix of the func
    import list (Arukellt emits wasi:clocks/* and wasi:random/* last).  Stubs
    are prepended to the function/code sections so their function indices match
    the removed imports — no call-site rewriting is required.
    """
    sections, _ = _parse_sections(wasm)
    header = wasm[:8]

    if 2 not in sections:
        return wasm
    imp_off, imp_size = sections[2]
    imp_data = wasm[imp_off : imp_off + imp_size]
    entries = _parse_import_entries(imp_data)

    kept_raw: list[bytes] = []
    stub_types: list[int] = []
    removing = False
    for mod_name, _field, type_idx, raw in entries:
        should_remove = type_idx >= 0 and any(
            mod_name.startswith(prefix) for prefix in module_prefixes
        )
        if should_remove:
            removing = True
            stub_types.append(type_idx)
        else:
            if removing:
                # Non-trailing removal would shift later import indices.
                raise ValueError(
                    "strip_wit_imports requires removed imports to be a "
                    "trailing suffix of the import list"
                )
            kept_raw.append(raw)

    if not stub_types:
        return wasm

    if 1 not in sections or 3 not in sections or 10 not in sections:
        return wasm

    type_off, type_size = sections[1]
    type_data = wasm[type_off : type_off + type_size]

    new_imp = bytearray(_write_uleb(len(kept_raw)))
    for raw in kept_raw:
        new_imp.extend(raw)

    fs_off, fs_size = sections[3]
    fs_data = wasm[fs_off : fs_off + fs_size]
    fcount, fpos = _read_uleb(fs_data, 0)
    new_fs = bytearray(_write_uleb(fcount + len(stub_types)))
    for t in stub_types:
        new_fs.extend(_write_uleb(t))
    new_fs.extend(fs_data[fpos:])

    code_off, code_size = sections[10]
    code_data = wasm[code_off : code_off + code_size]
    ccount, cpos = _read_uleb(code_data, 0)
    new_code = bytearray(_write_uleb(ccount + len(stub_types)))
    for t in stub_types:
        body = _stub_func_body(_func_type_results(type_data, t))
        new_code.extend(_write_uleb(len(body)))
        new_code.extend(body)
    new_code.extend(code_data[cpos:])

    section_list: list[tuple[int, bytes]] = []
    for sid in sorted(sections.keys()):
        if sid == 2:
            section_list.append((sid, bytes(new_imp)))
        elif sid == 3:
            section_list.append((sid, bytes(new_fs)))
        elif sid == 10:
            section_list.append((sid, bytes(new_code)))
        else:
            off, size = sections[sid]
            section_list.append((sid, wasm[off : off + size]))

    return _rebuild_sections(header, section_list)
