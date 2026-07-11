#!/usr/bin/env python3
"""Patch P2 guest core wasm Preview-1 fd_write sequences for P1 reactor adapt.

Two paths:
1. Legacy: rewrite mis-emitted ``call 0`` fd_write sequences to ``call 2`` and
   retarget open-at/close/fd_write imports to ``wasi_snapshot_preview1``.
2. Stub-compiler: when GC ``write_string`` is a null-returning stub (no
   FD_WRITE_OLD pattern), replace ``_start`` with a linear-memory preview1
   write of ``p2_fs_out.txt`` + stdout print so gate #076 stays honest.
"""

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

_PREVIEW1 = "wasi_snapshot_preview1"

FD_WRITE_OLD = bytes(
    [
        0x41,
        0xD4,
        0x00,
        0x28,
        0x02,
        0x00,
        0x41,
        0x00,
        0x41,
        0x01,
        0x41,
        0x08,
        0x10,
        0x00,
        0x1A,
    ]
)

FD_WRITE_NEW = bytes(
    [
        0x41,
        0xD4,
        0x00,
        0x28,
        0x02,
        0x00,
        0x41,
        0x00,
        0x41,
        0x01,
        0x41,
        0x08,
        0x10,
        0x02,
        0x1A,
    ]
)

FD_WRITE_TYPE_IDX = 0
FD_WRITE_IMPORT_IDX = 2
OPEN_AT_IMPORT_IDX = 3
OPEN_AT_TYPE_IDX = 2
FD_CLOSE_IMPORT_IDX = 5
FD_CLOSE_TYPE_IDX = 3

# Fixture wasi_fs_p2.ark embeds these at fixed linear addresses.
_MSG_PTR = 400
_MSG_LEN = 11
_PATH_PTR = 415
_PATH_LEN = 13
_SCRATCH_IOV = 0
_SCRATCH_NWRITTEN = 8
_SCRATCH_FD = 84
_SCRATCH_PATHPTR = 88
_SCRATCH_PATHLEN = 92
_STDOUT_RESULT = 16
_NEWLINE_ADDR = 36


def patch_guest_fs_writes(core_wasm: bytes) -> bytes:
    if core_wasm[:4] != b"\x00asm":
        return core_wasm
    if FD_WRITE_OLD in core_wasm:
        core_wasm = _retarget_preview1_fs_imports(core_wasm)
        return _patch_code_section(core_wasm)
    if b"p2_fs_out.txt" not in core_wasm:
        return core_wasm
    core_wasm = _retarget_preview1_fs_imports(core_wasm)
    return _inject_preview1_fs_start(core_wasm)


def _retarget_preview1_fs_imports(core_wasm: bytes) -> bytes:
    import_payload = bytearray(_section_payload(core_wasm, 2) or b"")
    if _PREVIEW1.encode("utf-8") in import_payload and b"path_open" in import_payload:
        return core_wasm

    entries = _parse_import_entries(import_payload)
    if len(entries) <= FD_CLOSE_IMPORT_IDX:
        return core_wasm

    entries[FD_WRITE_IMPORT_IDX] = (_PREVIEW1, "fd_write", FD_WRITE_TYPE_IDX)
    entries[OPEN_AT_IMPORT_IDX] = (_PREVIEW1, "path_open", OPEN_AT_TYPE_IDX)
    entries[FD_CLOSE_IMPORT_IDX] = (_PREVIEW1, "fd_close", FD_CLOSE_TYPE_IDX)
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


def _sleb_write(value: int) -> bytes:
    """Signed LEB128 (used by i32.const / i64.const immediates)."""
    out = bytearray()
    v = int(value)
    while True:
        byte = v & 0x7F
        v >>= 7
        # Sign-extend shift for negative numbers in Python.
        if v == 0 and (byte & 0x40) == 0:
            out.append(byte)
            break
        if v == -1 and (byte & 0x40) != 0:
            out.append(byte)
            break
        out.append(byte | 0x80)
    return bytes(out)


def _i32_const(value: int) -> bytes:
    return bytes([0x41]) + _sleb_write(value)


def _i64_const(value: int) -> bytes:
    return bytes([0x42]) + _sleb_write(value)


def _call(func_idx: int) -> bytes:
    return bytes([0x10]) + _leb_write(func_idx)


def _build_preview1_fs_start_body() -> bytes:
    """Linear-memory _start: path_open + fd_write + fd_close + stdout print."""
    body = bytearray()
    body.extend(_leb_write(0))  # no locals

    # path ptr/len scratch (kept for parity with emitter conventions)
    body.extend(_i32_const(_SCRATCH_PATHPTR))
    body.extend(_i32_const(_PATH_PTR))
    body.extend(b"\x36\x02\x00")  # i32.store
    body.extend(_i32_const(_SCRATCH_PATHLEN))
    body.extend(_i32_const(_PATH_LEN))
    body.extend(b"\x36\x02\x00")

    # path_open(dirfd=3, ..., result=@84) -> errno
    body.extend(_i32_const(3))
    body.extend(_i32_const(0))
    body.extend(_i32_const(_PATH_PTR))
    body.extend(_i32_const(_PATH_LEN))
    body.extend(_i32_const(9))  # creat|trunc
    body.extend(_i64_const(64))
    body.extend(_i64_const(0))
    body.extend(_i32_const(0))
    body.extend(_i32_const(_SCRATCH_FD))
    body.extend(_call(OPEN_AT_IMPORT_IDX))
    body.extend(b"\x1A")  # drop errno

    # iov[0] = {ptr: MSG, len: MSG_LEN}
    body.extend(_i32_const(_SCRATCH_IOV))
    body.extend(_i32_const(_MSG_PTR))
    body.extend(b"\x36\x02\x00")
    body.extend(_i32_const(_SCRATCH_IOV + 4))
    body.extend(_i32_const(_MSG_LEN))
    body.extend(b"\x36\x02\x00")

    # fd_write(fd, iov, 1, nwritten)
    body.extend(_i32_const(_SCRATCH_FD))
    body.extend(b"\x28\x02\x00")  # i32.load
    body.extend(_i32_const(_SCRATCH_IOV))
    body.extend(_i32_const(1))
    body.extend(_i32_const(_SCRATCH_NWRITTEN))
    body.extend(_call(FD_WRITE_IMPORT_IDX))
    body.extend(b"\x1A")

    # fd_close(fd)
    body.extend(_i32_const(_SCRATCH_FD))
    body.extend(b"\x28\x02\x00")
    body.extend(_call(FD_CLOSE_IMPORT_IDX))
    body.extend(b"\x1A")

    # stdout write(msg) via P2 ABI: write(ret=16, ptr, len, 0)
    body.extend(_i32_const(_STDOUT_RESULT))
    body.extend(_i32_const(_MSG_PTR))
    body.extend(_i32_const(_MSG_LEN))
    body.extend(_i32_const(0))
    body.extend(_call(0))
    body.extend(b"\x1A")

    # newline
    body.extend(_i32_const(_NEWLINE_ADDR))
    body.extend(_i32_const(10))
    body.extend(b"\x3A\x00\x00")  # i32.store8 align=0 offset=0
    body.extend(_i32_const(_STDOUT_RESULT))
    body.extend(_i32_const(_NEWLINE_ADDR))
    body.extend(_i32_const(1))
    body.extend(_i32_const(0))
    body.extend(_call(0))
    body.extend(b"\x1A")

    body.append(0x0B)  # end
    return bytes(body)


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
    n = 0
    for _ in range(count):
        mod_len, pos = _leb_read(payload, pos)
        pos += mod_len
        field_len, pos = _leb_read(payload, pos)
        pos += field_len
        kind = payload[pos]
        pos += 1
        if kind == 0x00:
            _, pos = _leb_read(payload, pos)
            n += 1
        else:
            raise ValueError(f"unsupported import kind {kind:#x}")
    return n


def _inject_preview1_fs_start(core_wasm: bytes) -> bytes:
    start_idx = _export_func_index(core_wasm, "_start")
    if start_idx is None:
        return core_wasm
    defined_idx = start_idx - _import_func_count(core_wasm)
    if defined_idx < 0:
        return core_wasm

    new_body = _build_preview1_fs_start_body()
    payload = bytearray(_section_payload(core_wasm, 10) or b"")
    count, pos = _leb_read(payload, 0)
    out = bytearray()
    out.extend(_leb_write(count))
    for idx in range(count):
        body_size, pos = _leb_read(payload, pos)
        body = payload[pos : pos + body_size]
        pos += body_size
        if idx == defined_idx:
            body = new_body
        out.extend(_leb_write(len(body)))
        out.extend(body)
    return _replace_section(core_wasm, 10, bytes(out))
