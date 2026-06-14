#!/usr/bin/env python3
"""Wrap a P2 core wasm module in a wasi:cli/command component (bootstrap gate helper)."""

from __future__ import annotations

import argparse
import sys
from io import BytesIO


def leb128_u(value: int) -> bytes:
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


def wasm_string(text: str) -> bytes:
    raw = text.encode("utf-8")
    return leb128_u(len(raw)) + raw


class Writer:
    def __init__(self) -> None:
        self._buf = BytesIO()

    def byte(self, value: int) -> None:
        self._buf.write(bytes((value & 0xFF,)))

    def bytes(self, data: bytes) -> None:
        self._buf.write(data)

    def leb(self, value: int) -> None:
        self._buf.write(leb128_u(value))

    def string(self, text: str) -> None:
        self._buf.write(wasm_string(text))

    def size(self) -> int:
        return self._buf.tell()

    def finish(self) -> bytes:
        return self._buf.getvalue()


def emit_section(out: Writer, section_id: int, contents: bytes) -> None:
    out.byte(section_id)
    out.leb(len(contents))
    out.bytes(contents)


def core_header() -> bytes:
    return b"\x00asm\x01\x00\x00\x00"


def emit_func_type(type_sec: Writer, params: list[int], results: list[int]) -> None:
    type_sec.byte(0x60)
    type_sec.leb(len(params))
    for param in params:
        type_sec.byte(param)
    type_sec.leb(len(results))
    for result in results:
        type_sec.byte(result)


def emit_return_zero_bodies(out: Writer, body_count: int) -> None:
    code_sec = Writer()
    code_sec.leb(body_count)
    for _ in range(body_count):
        body = Writer()
        body.leb(0)  # locals
        body.byte(0x41)
        body.leb(0)
        body.byte(0x0B)
        entry = body.finish()
        code_sec.leb(len(entry))
        code_sec.bytes(entry)
    emit_section(out, 10, code_sec.finish())


def emit_void_body(out: Writer) -> None:
    code_sec = Writer()
    code_sec.leb(1)
    body = Writer()
    body.leb(0)
    body.byte(0x0B)
    entry = body.finish()
    code_sec.leb(len(entry))
    code_sec.bytes(entry)
    emit_section(out, 10, code_sec.finish())


def emit_exports_list(out: Writer, names: list[str], func_idxs: list[int]) -> None:
    export_sec = Writer()
    export_sec.leb(len(names))
    for name, func_idx in zip(names, func_idxs):
        export_sec.string(name)
        export_sec.byte(0x00)
        export_sec.leb(func_idx)
    emit_section(out, 7, export_sec.finish())


def stub_single_export(export_name: str, void_ret: bool) -> bytes:
    out = Writer()
    out.bytes(core_header())
    type_sec = Writer()
    type_sec.leb(1)
    if void_ret:
        emit_func_type(type_sec, [0x7F], [])
    else:
        emit_func_type(type_sec, [0x7F, 0x7F, 0x7F, 0x7F], [0x7F])
    emit_section(out, 1, type_sec.finish())
    func_sec = Writer()
    func_sec.leb(1)
    func_sec.leb(0)
    emit_section(out, 3, func_sec.finish())
    emit_exports_list(out, [export_name], [0])
    if void_ret:
        emit_void_body(out)
    else:
        emit_return_zero_bodies(out, 1)
    return out.finish()


def stub_env_module() -> bytes:
    out = Writer()
    out.bytes(core_header())
    type_sec = Writer()
    type_sec.leb(1)
    emit_func_type(type_sec, [0x7F, 0x7F], [0x7F])
    emit_section(out, 1, type_sec.finish())
    func_sec = Writer()
    func_sec.leb(2)
    func_sec.leb(0)
    func_sec.leb(0)
    emit_section(out, 3, func_sec.finish())
    emit_exports_list(out, ["args-sizes", "arguments"], [0, 1])
    emit_return_zero_bodies(out, 2)
    return out.finish()


def stub_fs_module() -> bytes:
    out = Writer()
    out.bytes(core_header())
    type_sec = Writer()
    type_sec.leb(2)
    emit_func_type(
        type_sec,
        [0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7E, 0x7E, 0x7F, 0x7F],
        [0x7F],
    )
    emit_func_type(type_sec, [0x7F], [0x7F])
    emit_section(out, 1, type_sec.finish())
    func_sec = Writer()
    func_sec.leb(2)
    func_sec.leb(0)
    func_sec.leb(1)
    emit_section(out, 3, func_sec.finish())
    emit_exports_list(out, ["open-at", "close"], [0, 1])
    emit_return_zero_bodies(out, 2)
    return out.finish()


P2_RUN_INNER_COMPONENT = bytes.fromhex(
    "0061736d0d0001000708026a0000400000000a1401000f696d706f72742d66756e632d72756e"
    "01010708026a0000400000020b0b01000372756e0100010103"
)


def emit_core_instance_ref(sec: Writer, module_index: int, instance_index: int) -> None:
    sec.byte(0x00)
    sec.leb(module_index)
    sec.leb(instance_index)


def emit_instance_arg(sec: Writer, import_name: str, instance_index: int) -> None:
    sec.string(import_name)
    sec.byte(0x12)
    sec.leb(instance_index)


def emit_p2_run_command_world_sections(out: Writer) -> None:
    emit_section(out, 4, P2_RUN_INNER_COMPONENT)

    inst = Writer()
    inst.leb(1)
    inst.byte(0x00)
    inst.leb(0)
    inst.leb(1)
    inst.string("import-func-run")
    inst.byte(0x01)
    inst.leb(0)
    emit_section(out, 5, inst.finish())

    alias_sec = Writer()
    alias_sec.leb(1)
    alias_sec.byte(0x00)
    alias_sec.byte(0x00)
    alias_sec.byte(0x01)
    alias_sec.leb(5)
    alias_sec.string("_start")
    emit_section(out, 6, alias_sec.finish())

    type_sec = Writer()
    type_sec.leb(2)
    type_sec.byte(0x6A)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    type_sec.byte(0x40)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    emit_section(out, 7, type_sec.finish())

    canon_sec = Writer()
    canon_sec.leb(1)
    canon_sec.byte(0x00)
    canon_sec.byte(0x00)
    canon_sec.leb(0)
    canon_sec.leb(0)
    canon_sec.leb(1)
    emit_section(out, 8, canon_sec.finish())

    export_sec = Writer()
    export_sec.leb(1)
    export_sec.byte(0x00)
    export_sec.string("wasi:cli/run@0.2.6")
    export_sec.byte(0x05)
    export_sec.leb(0)
    emit_section(out, 11, export_sec.finish())


def wrap_p2_command_component(core_wasm: bytes) -> bytes:
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")

    for module in (
        stub_single_export("write", False),
        stub_env_module(),
        stub_fs_module(),
        stub_single_export("read", False),
        stub_single_export("exit", True),
        core_wasm,
    ):
        emit_section(out, 1, module)

    inst_sec = Writer()
    inst_sec.leb(6)
    for module_index in range(5):
        emit_core_instance_ref(inst_sec, module_index, 0)
    inst_sec.byte(0x00)
    inst_sec.leb(5)
    inst_sec.leb(5)
    for import_name, instance_index in (
        ("wasi:cli/stdout@0.2.0", 0),
        ("wasi:cli/environment@0.2.0", 1),
        ("wasi:filesystem/types@0.2.0", 2),
        ("wasi:cli/stdin@0.2.0", 3),
        ("wasi:cli/exit@0.2.0", 4),
    ):
        emit_instance_arg(inst_sec, import_name, instance_index)
    emit_section(out, 2, inst_sec.finish())

    emit_p2_run_command_world_sections(out)
    return out.finish()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("core_wasm", type=argparse.FileType("rb"))
    parser.add_argument("output", type=argparse.FileType("wb"))
    args = parser.parse_args()
    try:
        component = wrap_p2_command_component(args.core_wasm.read())
    except Exception as exc:  # noqa: BLE001
        print(f"p2_component_wrap: {exc}", file=sys.stderr)
        return 1
    args.output.write(component)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
