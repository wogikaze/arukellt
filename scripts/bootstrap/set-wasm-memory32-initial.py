#!/usr/bin/env python3
"""Raise the first memory32 initial size without parsing code or GC types."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

WASM_MAGIC_VERSION = b"\x00asm\x01\x00\x00\x00"
MEMORY_SECTION_ID = 5
FLAG_HAS_MAX = 0x01
FLAG_SHARED = 0x02
FLAG_MEMORY64 = 0x04
FLAG_CUSTOM_PAGE_SIZE = 0x08


def read_uleb(data: bytes, offset: int, *, max_bits: int = 64) -> tuple[int, int]:
    value = 0
    shift = 0
    while offset < len(data):
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if byte & 0x80 == 0:
            return value, offset
        shift += 7
        if shift >= max_bits + 7:
            raise ValueError("ULEB value is too large")
    raise ValueError("truncated ULEB value")


def write_uleb(value: int) -> bytes:
    if value < 0:
        raise ValueError("ULEB value must be non-negative")
    output = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            byte |= 0x80
        output.append(byte)
        if not value:
            return bytes(output)


def rewrite_memory_section(payload: bytes, pages: int) -> bytes:
    count, offset = read_uleb(payload, 0, max_bits=32)
    if count < 1:
        raise ValueError("memory section is empty")

    output = bytearray(write_uleb(count))
    for memory_index in range(count):
        flags, offset = read_uleb(payload, offset, max_bits=32)
        if flags & FLAG_MEMORY64:
            raise ValueError("expected memory32, found memory64")
        minimum, offset = read_uleb(payload, offset, max_bits=32)
        maximum: int | None = None
        if flags & FLAG_HAS_MAX:
            maximum, offset = read_uleb(payload, offset, max_bits=32)
        page_size_log2: int | None = None
        if flags & FLAG_CUSTOM_PAGE_SIZE:
            page_size_log2, offset = read_uleb(payload, offset, max_bits=32)

        if memory_index == 0:
            minimum = pages
            if maximum is not None and maximum < pages:
                maximum = pages
        if flags & FLAG_SHARED and maximum is None:
            raise ValueError("shared memory is missing a maximum")

        output.extend(write_uleb(flags))
        output.extend(write_uleb(minimum))
        if maximum is not None:
            output.extend(write_uleb(maximum))
        if page_size_log2 is not None:
            output.extend(write_uleb(page_size_log2))

    if offset != len(payload):
        raise ValueError("unexpected trailing bytes in memory section")
    return bytes(output)


def rewrite_module(data: bytes, pages: int) -> bytes:
    if not data.startswith(WASM_MAGIC_VERSION):
        raise ValueError("not a WebAssembly module")
    output = bytearray(WASM_MAGIC_VERSION)
    offset = len(WASM_MAGIC_VERSION)
    saw_memory = False

    while offset < len(data):
        section_id = data[offset]
        offset += 1
        size, payload_start = read_uleb(data, offset, max_bits=32)
        payload_end = payload_start + size
        if payload_end > len(data):
            raise ValueError("section extends past end of module")
        payload = data[payload_start:payload_end]
        if section_id == MEMORY_SECTION_ID:
            if saw_memory:
                raise ValueError("multiple memory sections")
            payload = rewrite_memory_section(payload, pages)
            saw_memory = True
        output.append(section_id)
        output.extend(write_uleb(len(payload)))
        output.extend(payload)
        offset = payload_end

    if not saw_memory:
        raise ValueError("module has no memory section")
    return bytes(output)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--pages", type=int, default=65535)
    args = parser.parse_args()
    if not 1 <= args.pages <= 65536:
        raise ValueError("memory32 pages must be between 1 and 65536")

    rewritten = rewrite_module(args.input.read_bytes(), args.pages)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(rewritten)
    print(
        "set-wasm-memory32-initial: PASS: "
        f"pages={args.pages} bytes={len(rewritten)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"set-wasm-memory32-initial: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
