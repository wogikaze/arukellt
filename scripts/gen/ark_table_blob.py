"""Emit compact generated tables: hex i32 blobs + concatenated string slices.

Do not emit `if index == N` arms or rebuild a Vec on each `_at` call.
32-arm chunk dispatch is not the representation (ADR-053 Phase 1).
"""
from __future__ import annotations

I32_CHUNK = 16
STR_CHUNK = 80


def ark_string(text: str) -> str:
    escaped = text.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def i32_lit(value: int) -> str:
    if value < 0:
        return f"0 - {abs(value)}"
    return str(value)


def pack_i32_hex(values: list[int]) -> str:
    parts: list[str] = []
    for value in values:
        unsigned = value & 0xFFFFFFFF
        parts.append(
            bytes(
                (
                    unsigned & 255,
                    (unsigned >> 8) & 255,
                    (unsigned >> 16) & 255,
                    (unsigned >> 24) & 255,
                )
            ).hex()
        )
    return "".join(parts)


def unpack_i32_hex8(hex_text: str, offset: int) -> int:
    raw = bytes.fromhex(hex_text[offset : offset + 8])
    unsigned = raw[0] | (raw[1] << 8) | (raw[2] << 16) | (raw[3] << 24)
    if unsigned >= 0x80000000:
        return unsigned - 0x100000000
    return unsigned


def emit_i32_table(name: str, values: list[int], default: int = -1) -> list[str]:
    at_name = f"{name}_at"
    if not values:
        return [
            f"fn {at_name}(index: i32) -> i32 {{",
            f"    return {i32_lit(default)}",
            "}",
            "",
        ]

    hex_all = pack_i32_hex(values)
    chunk_hex_len = I32_CHUNK * 8
    chunks = [hex_all[i : i + chunk_hex_len] for i in range(0, len(hex_all), chunk_hex_len)]
    lines = [f"fn {name}_hex_chunk(chunk: i32) -> String {{"]
    for chunk_index, chunk_hex in enumerate(chunks):
        lines.append(f"    if chunk == {chunk_index} {{ return {ark_string(chunk_hex)} }}")
    lines.extend(
        [
            "    return String_new()",
            "}",
            "",
            f"fn {at_name}(index: i32) -> i32 {{",
            f"    if index < 0 || index >= {len(values)} {{",
            f"        return {i32_lit(default)}",
            "    }",
            f"    let chunk = index / {I32_CHUNK}",
            f"    let local = index - chunk * {I32_CHUNK}",
            f"    return table_blob::table_blob_i32_from_hex8({name}_hex_chunk(chunk), local * 8)",
            "}",
            "",
        ]
    )
    return lines


def emit_string_table(name: str, values: list[str]) -> list[str]:
    at_name = f"{name}_at"
    if not values:
        return [
            f"fn {at_name}(index: i32) -> String {{",
            "    return String_new()",
            "}",
            "",
        ]

    offsets: list[int] = []
    cursor = 0
    for value in values:
        offsets.append(cursor)
        cursor += len(value)
    offsets.append(cursor)
    data = "".join(values)
    lines = emit_i32_table(f"{name}_off", offsets, default=0)
    if not data:
        lines.extend(
            [
                f"fn {at_name}(index: i32) -> String {{",
                "    return String_new()",
                "}",
                "",
            ]
        )
        return lines

    chunks = [data[i : i + STR_CHUNK] for i in range(0, len(data), STR_CHUNK)]
    lines.append(f"fn {name}_data_chunk(chunk: i32) -> String {{")
    for chunk_index, chunk_text in enumerate(chunks):
        lines.append(f"    if chunk == {chunk_index} {{ return {ark_string(chunk_text)} }}")
    lines.extend(
        [
            "    return String_new()",
            "}",
            "",
            f"fn {at_name}(index: i32) -> String {{",
            f"    if index < 0 || index >= {len(values)} {{",
            "        return String_new()",
            "    }",
            f"    let start = {name}_off_at(index)",
            f"    let end = {name}_off_at(index + 1)",
            "    if start >= end {",
            "        return String_new()",
            "    }",
            f"    let start_chunk = start / {STR_CHUNK}",
            "    let last = end - 1",
            f"    let end_chunk = last / {STR_CHUNK}",
            "    if start_chunk == end_chunk {",
            f"        let piece = {name}_data_chunk(start_chunk)",
            f"        let local = start - start_chunk * {STR_CHUNK}",
            "        return substring(piece, local, local + (end - start))",
            "    }",
            "    let mut out = String_new()",
            "    let mut i = start",
            "    while i < end {",
            f"        let chunk = i / {STR_CHUNK}",
            f"        let local = i - chunk * {STR_CHUNK}",
            f"        let piece = {name}_data_chunk(chunk)",
            f"        let avail = {STR_CHUNK} - local",
            "        let remain = end - i",
            "        let mut take = avail",
            "        if remain < avail {",
            "            take = remain",
            "        }",
            "        out = concat(out, substring(piece, local, local + take))",
            "        i = i + take",
            "    }",
            "    return out",
            "}",
            "",
        ]
    )
    return lines


def emit_bool_table(name: str, values: list[bool], default: bool = False) -> list[str]:
    at_name = f"{name}_at"
    if not values:
        return [
            f"fn {at_name}(index: i32) -> bool {{",
            f"    return {'true' if default else 'false'}",
            "}",
            "",
        ]
    flag_name = f"{name}_flag"
    lines = emit_i32_table(flag_name, [1 if value else 0 for value in values], default=1 if default else 0)
    lines.extend(
        [
            f"fn {at_name}(index: i32) -> bool {{",
            f"    return {flag_name}_at(index) != 0",
            "}",
            "",
        ]
    )
    return lines
