#!/usr/bin/env python3
"""Patch P2 guest core wasm write calls to pass ptr/len on the stack."""

from __future__ import annotations

# Emitter stub: write(1, 0, 1, 8). Canonical ABI needs write(ret, ptr, len, 0)
# with ptr/len loaded from guest mem[0]/mem[4] (SCRATCH_I32BUF list head).
OLD = bytes([0x41, 0x01, 0x41, 0x00, 0x41, 0x01, 0x41, 0x08, 0x10, 0x00])
NEW = bytes(
    [
        0x41,
        0x10,  # i32.const 16 (result area at SCRATCH_I32BUF)
        0x41,
        0x00,  # i32.const 0
        0x28,
        0x02,
        0x00,  # i32.load mem[0] -> string ptr
        0x41,
        0x04,  # i32.const 4
        0x28,
        0x02,
        0x00,  # i32.load mem[4] -> string len
        0x41,
        0x00,  # i32.const 0 (unused)
        0x10,
        0x00,  # call write import
    ]
)
VOID_FUNC_TYPE = bytes([0x60, 0x00, 0x00])
I32_FUNC_TYPE = bytes([0x60, 0x00, 0x01, 0x7F])
RETURN_I32 = bytes([0x41, 0x00, 0x0B])
VOID_RETURN_END = bytes([0x0F, 0x0B])


def _leb_read(data: bytes, pos: int) -> tuple[int, int]:
    result = 0
    shift = 0
    while pos < len(data):
        byte = data[pos]
        pos += 1
        result |= (byte & 0x7F) << shift
        if not (byte & 0x80):
            return result, pos
        shift += 7
    raise ValueError("truncated leb128")


def _leb_write(value: int) -> bytes:
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


def patch_guest_core(core_wasm: bytes) -> bytes:
    """Apply stdio + `_start` signature patches needed by the P2 wrap helper."""
    return _add_run_export(_patch_start_returns_i32(patch_guest_write_calls(core_wasm)))


def _add_run_export(core_wasm: bytes) -> bytes:
    """Export `run` alias for `wasi:cli/command` embed + component-new."""
    start_idx = _export_func_index(core_wasm, "_start")
    if start_idx is None:
        return core_wasm
    for name in ("run", "wasi:cli/run@0.2.0#run"):
        if _export_func_index(core_wasm, name) is not None:
            continue
        payload = bytearray(_section_payload(core_wasm, 7) or b"")
        count, pos = _leb_read(payload, 0)
        extra = bytearray()
        extra.extend(_leb_write(len(name)))
        extra.extend(name.encode("utf-8"))
        extra.append(0x00)
        extra.extend(_leb_write(start_idx))
        new_payload = _leb_write(count + 1) + bytes(payload[pos:]) + bytes(extra)
        core_wasm = _replace_section(core_wasm, 7, bytes(new_payload))
    return core_wasm


def patch_guest_write_calls(core_wasm: bytes) -> bytes:
    if core_wasm[:4] != b"\x00asm":
        return core_wasm
    pos = 8
    sections: list[tuple[int, bytes]] = []
    while pos < len(core_wasm):
        section_id = core_wasm[pos]
        pos += 1
        section_size, pos = _leb_read(core_wasm, pos)
        payload = core_wasm[pos : pos + section_size]
        pos += section_size
        if section_id == 10:
            payload = _patch_code_section(payload)
        sections.append((section_id, payload))

    out = bytearray(core_wasm[:8])
    for section_id, payload in sections:
        out.append(section_id)
        out.extend(_leb_write(len(payload)))
        out.extend(payload)
    return bytes(out)


def _patch_start_returns_i32(core_wasm: bytes) -> bytes:
    """Align pinned bootstrap `_start: () -> ()` with wasi:cli/run canon lift."""
    if core_wasm[:4] != b"\x00asm":
        return core_wasm

    start_func_idx = _export_func_index(core_wasm, "_start")
    if start_func_idx is None:
        return core_wasm

    import_func_count = _import_func_count(core_wasm)
    defined_idx = start_func_idx - import_func_count
    if defined_idx < 0:
        return core_wasm

    type_idx = _defined_func_type_index(core_wasm, defined_idx)
    if type_idx is None:
        return core_wasm

    type_payload = _section_payload(core_wasm, 1)
    if type_payload is None:
        return core_wasm
    if _type_at(type_payload, type_idx) != VOID_FUNC_TYPE:
        return core_wasm

    i32_type_idx = _find_type_index(type_payload, I32_FUNC_TYPE)
    if i32_type_idx is None:
        return core_wasm

    core_wasm = _set_defined_func_type(core_wasm, defined_idx, i32_type_idx)
    return _patch_defined_func_body(core_wasm, defined_idx, _append_return_i32)


def _export_func_index(core_wasm: bytes, name: str) -> int | None:
    payload = _section_payload(core_wasm, 7)
    if payload is None:
        return None
    count, pos = _leb_read(payload, 0)
    for _ in range(count):
        name_len, pos = _leb_read(payload, pos)
        export_name = payload[pos : pos + name_len].decode("utf-8")
        pos += name_len
        kind = payload[pos]
        pos += 1
        index, pos = _leb_read(payload, pos)
        if kind == 0x00 and export_name == name:
            return index
    return None


def _import_func_count(core_wasm: bytes) -> int:
    payload = _section_payload(core_wasm, 2)
    if payload is None:
        return 0
    count, pos = _leb_read(payload, 0)
    import_funcs = 0
    for _ in range(count):
        mod_len, pos = _leb_read(payload, pos)
        pos += mod_len
        field_len, pos = _leb_read(payload, pos)
        pos += field_len
        kind = payload[pos]
        pos += 1
        if kind == 0x00:
            _, pos = _leb_read(payload, pos)
            import_funcs += 1
        elif kind == 0x01:
            flags = payload[pos]
            pos += 1
            if flags & 0x03 == 0x00:
                pos += 1
            elif flags & 0x03 == 0x01:
                pos += 2
            else:
                pos += 3
        elif kind == 0x02:
            pos += 1
        elif kind == 0x03:
            pos += 1
        else:
            break
    return import_funcs


def _defined_func_type_index(core_wasm: bytes, defined_idx: int) -> int | None:
    payload = _section_payload(core_wasm, 3)
    if payload is None:
        return None
    count, pos = _leb_read(payload, 0)
    if defined_idx >= count:
        return None
    for _ in range(defined_idx):
        _, pos = _leb_read(payload, pos)
    type_idx, _ = _leb_read(payload, pos)
    return type_idx


def _set_defined_func_type(core_wasm: bytes, defined_idx: int, type_idx: int) -> bytes:
    payload = bytearray(_section_payload(core_wasm, 3) or b"")
    count, pos = _leb_read(payload, 0)
    for _ in range(defined_idx):
        _, pos = _leb_read(payload, pos)
    start = pos
    _, pos = _leb_read(payload, pos)
    end = pos
    new_payload = bytes(payload[:start]) + _leb_write(type_idx) + bytes(payload[end:])
    return _replace_section(core_wasm, 3, new_payload)


def _patch_defined_func_body(
    core_wasm: bytes,
    defined_idx: int,
    patch_body: callable,
) -> bytes:
    payload = bytearray(_section_payload(core_wasm, 10) or b"")
    count, pos = _leb_read(payload, 0)
    out = bytearray()
    out.extend(_leb_write(count))
    for idx in range(count):
        body_size, pos = _leb_read(payload, pos)
        body = bytearray(payload[pos : pos + body_size])
        pos += body_size
        if idx == defined_idx:
            body = bytearray(patch_body(bytes(body)))
        out.extend(_leb_write(len(body)))
        out.extend(body)
    return _replace_section(core_wasm, 10, bytes(out))


def _append_return_i32(body: bytes) -> bytes:
    if body.endswith(RETURN_I32):
        return body
    if body.endswith(VOID_RETURN_END):
        return body[:-2] + RETURN_I32
    if body.endswith(b"\x0B"):
        return body[:-1] + RETURN_I32
    return body + RETURN_I32


def _section_payload(core_wasm: bytes, section_id: int) -> bytes | None:
    pos = 8
    while pos < len(core_wasm):
        sid = core_wasm[pos]
        pos += 1
        size, pos = _leb_read(core_wasm, pos)
        payload = core_wasm[pos : pos + size]
        pos += size
        if sid == section_id:
            return payload
    return None


def _replace_section(core_wasm: bytes, section_id: int, payload: bytes) -> bytes:
    pos = 8
    out = bytearray(core_wasm[:8])
    while pos < len(core_wasm):
        sid = core_wasm[pos]
        pos += 1
        size, pos = _leb_read(core_wasm, pos)
        old_payload = core_wasm[pos : pos + size]
        pos += size
        if sid == section_id:
            old_payload = payload
        out.append(sid)
        out.extend(_leb_write(len(old_payload)))
        out.extend(old_payload)
    return bytes(out)


def _type_at(type_payload: bytes, index: int) -> bytes | None:
    count, pos = _leb_read(type_payload, 0)
    if index >= count:
        return None
    for _ in range(index):
        form = type_payload[pos]
        pos += 1
        if form != 0x60:
            return None
        param_count, pos = _leb_read(type_payload, pos)
        pos += param_count
        result_count, pos = _leb_read(type_payload, pos)
        pos += result_count
    start = pos
    form = type_payload[pos]
    pos += 1
    if form != 0x60:
        return None
    param_count, pos = _leb_read(type_payload, pos)
    pos += param_count
    result_count, pos = _leb_read(type_payload, pos)
    pos += result_count
    return type_payload[start:pos]


def _find_type_index(type_payload: bytes, needle: bytes) -> int | None:
    count, pos = _leb_read(type_payload, 0)
    for idx in range(count):
        start = pos
        form = type_payload[pos]
        pos += 1
        if form != 0x60:
            return None
        param_count, pos = _leb_read(type_payload, pos)
        pos += param_count
        result_count, pos = _leb_read(type_payload, pos)
        pos += result_count
        if type_payload[start:pos] == needle:
            return idx
    return None


def _patch_code_section(payload: bytes) -> bytes:
    if OLD not in payload:
        return payload
    count, pos = _leb_read(payload, 0)
    out = bytearray()
    out.extend(_leb_write(count))
    for _ in range(count):
        body_size, pos = _leb_read(payload, pos)
        body = bytearray(payload[pos : pos + body_size])
        pos += body_size
        if OLD in body:
            body = body.replace(OLD, NEW)
        out.extend(_leb_write(len(body)))
        out.extend(body)
    return bytes(out)
