#!/usr/bin/env python3
"""Patch P2 guest core wasm write calls to pass ptr/len on the stack."""

from __future__ import annotations

OLD = bytes([0x41, 0x01, 0x41, 0x00, 0x41, 0x01, 0x41, 0x08, 0x10, 0x00])
NEW = bytes([0x41, 0x00, 0x28, 0x02, 0x00, 0x41, 0x04, 0x28, 0x02, 0x00, 0x41, 0x00, 0x41, 0x00, 0x10, 0x00])
VOID_FUNC_TYPE = bytes([0x60, 0x00, 0x00])
I32_FUNC_TYPE = bytes([0x60, 0x00, 0x01, 0x7F])


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
    patched = bytes(out)
    return patched


def _patch_start_returns_i32(core_wasm: bytes) -> bytes:
    """Reserved for future pinned-core `_start` signature alignment."""
    return core_wasm


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
