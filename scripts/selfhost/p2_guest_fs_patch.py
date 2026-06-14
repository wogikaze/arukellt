#!/usr/bin/env python3
"""Patch P2 guest core wasm Preview-1 fd_write sequences to P2 stream writes."""

from __future__ import annotations

import sys
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from p2_guest_stdio_patch import (
    _leb_read,
    _leb_write,
    _replace_section,
    _section_payload,
)

# P1-style fd_write(fd@84, iov@0, 1, nwritten@8) misrouted to stdout import 0.
FD_WRITE_OLD = bytes(
    [
        0x41,
        0xD4,
        0x00,  # i32.const 84
        0x28,
        0x02,
        0x00,  # i32.load
        0x41,
        0x00,
        0x41,
        0x01,
        0x41,
        0x08,
        0x10,
        0x00,  # call 0
        0x1A,  # drop
    ]
)

# write-via-stream import 2; blocking-write-and-flush import 4 (replaced guest imports).
# Stream handle lives at result@132+4 after write-via-stream.
FD_WRITE_NEW = bytes(
    [
        0x41,
        0xD4,
        0x00,
        0x28,
        0x02,
        0x00,
        0x41,
        0x84,
        0x01,
        0x41,
        0x00,
        0x10,
        0x02,  # call write-via-stream (adapt performs flush)
        0x1A,
    ]
)

WRITE_VIA_STREAM_TYPE_IDX = 5
WRITE_VIA_STREAM_IMPORT_IDX = 2
BLOCKING_WRITE_FLUSH_IMPORT_IDX = 4


def patch_guest_fs_writes(core_wasm: bytes) -> bytes:
    """Rewrite misrouted fd_write calls and retarget unused guest imports."""
    if core_wasm[:4] != b"\x00asm":
        return core_wasm
    if FD_WRITE_OLD not in core_wasm:
        return core_wasm
    core_wasm = _retarget_fs_stream_imports(core_wasm)
    return _patch_code_section(core_wasm)


def _retarget_fs_stream_imports(core_wasm: bytes) -> bytes:
    import_payload = bytearray(_section_payload(core_wasm, 2) or b"")
    if b"write-via-stream" in import_payload:
        return core_wasm

    entries = _parse_import_entries(import_payload)
    if len(entries) <= BLOCKING_WRITE_FLUSH_IMPORT_IDX:
        return core_wasm

    entries[WRITE_VIA_STREAM_IMPORT_IDX] = (
        "wasi:filesystem/types@0.2.0",
        "write-via-stream",
        WRITE_VIA_STREAM_TYPE_IDX,
    )
    return _replace_section(core_wasm, 2, _encode_import_entries(entries))


def _parse_import_entries(import_payload: bytes) -> list[tuple[str, str, int]]:
    count, pos = _leb_read(import_payload, 0)
    entries: list[tuple[str, str, int]] = []
    for _ in range(count):
        mod_len, pos = _leb_read(import_payload, pos)
        module = import_payload[pos : pos + mod_len].decode("utf-8")
        pos += mod_len
        field_len, pos = _leb_read(import_payload, pos)
        field = import_payload[pos : pos + field_len].decode("utf-8")
        pos += field_len
        kind = import_payload[pos]
        pos += 1
        if kind != 0x00:
            raise ValueError(f"unsupported import kind {kind:#x}")
        type_idx, pos = _leb_read(import_payload, pos)
        entries.append((module, field, type_idx))
    return entries


def _encode_import_entries(entries: list[tuple[str, str, int]]) -> bytes:
    out = bytearray(_leb_write(len(entries)))
    for module, field, type_idx in entries:
        mod_bytes = module.encode("utf-8")
        field_bytes = field.encode("utf-8")
        out.extend(_leb_write(len(mod_bytes)))
        out.extend(mod_bytes)
        out.extend(_leb_write(len(field_bytes)))
        out.extend(field_bytes)
        out.append(0x00)
        out.extend(_leb_write(type_idx))
    return bytes(out)


def _patch_code_section(core_wasm: bytes) -> bytes:
    payload = bytearray(_section_payload(core_wasm, 10) or b"")
    if FD_WRITE_OLD not in payload:
        return core_wasm
    count, pos = _leb_read(payload, 0)
    out = bytearray()
    out.extend(_leb_write(count))
    for _ in range(count):
        body_size, pos = _leb_read(payload, pos)
        body = bytearray(payload[pos : pos + body_size])
        pos += body_size
        if FD_WRITE_OLD in body:
            body = body.replace(FD_WRITE_OLD, FD_WRITE_NEW)
        out.extend(_leb_write(len(body)))
        out.extend(body)
    return _replace_section(core_wasm, 10, bytes(out))
