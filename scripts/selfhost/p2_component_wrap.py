#!/usr/bin/env python3
"""Wrap a P2 core wasm module in a wasi:cli/command component (bootstrap gate helper).

Builds a wasi:cli/command component with host imports for wasi:io/streams and
wasi:cli/stdout, wires the stdout bridge core module to blocking-write-and-flush,
and appends wasi:cli/run export sections.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from io import BytesIO
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))
from p2_guest_stdio_patch import patch_guest_write_calls

_DATA_DIR = Path(__file__).resolve().parent / "data"
_BRIDGE_WASM = Path(__file__).resolve().parent / "p2_stdout_bridge.wasm"
_SELFHOST_DIR = Path(__file__).resolve().parent
_WIT_DIR = _SELFHOST_DIR / "wit" / "deps" / "wasi-cli-0.2.0" / "wit"


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
        body.leb(0)
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

# Host import prefix (error + streams + stdout @0.2.0) extracted from a working
# wasip2 component; see scripts/selfhost/data/p2_host_import_prefix_020.bin.
P2_HOST_IMPORT_PREFIX = _DATA_DIR / "p2_host_import_prefix_020.bin"

# Canon/instance wiring for get-stdout + blocking-write-and-flush + guest memory.
P2_STDIO_HOST_WIRING = _DATA_DIR / "p2_stdio_host_wiring.bin"


def load_bridge_module() -> bytes:
    if _BRIDGE_WASM.is_file():
        return _BRIDGE_WASM.read_bytes()
    # Fallback: compile WAT when wasm-tools is available.
    wat = Path(__file__).resolve().parent / "p2_stdout_bridge.wat"
    if not wat.is_file():
        raise FileNotFoundError(f"missing stdout bridge wasm and {wat}")
    import shutil
    import subprocess

    tool = shutil.which("wasm-tools")
    if tool is None:
        cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
        tool = str(cargo) if cargo.is_file() else None
    if tool is None:
        raise FileNotFoundError("wasm-tools required to compile p2_stdout_bridge.wat")
    out = _BRIDGE_WASM
    result = subprocess.run(
        [tool, "parse", str(wat), "-o", str(out)],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"wasm-tools parse failed: {result.stderr[-400:]}")
    return out.read_bytes()


def emit_core_instance_ref(sec: Writer, module_index: int, instance_index: int) -> None:
    sec.byte(0x00)
    sec.leb(module_index)
    sec.leb(instance_index)


def emit_instance_arg(sec: Writer, import_name: str, instance_index: int) -> None:
    sec.string(import_name)
    sec.byte(0x12)
    sec.leb(instance_index)


def emit_bridge_instance_ref(sec: Writer, get_stdout_core: int, flush_core: int) -> None:
    sec.byte(0x00)
    sec.leb(0)
    sec.leb(2)
    sec.string("env")
    sec.string("get-stdout")
    sec.byte(0x01)
    sec.leb(get_stdout_core)
    sec.string("env")
    sec.string("blocking-write-and-flush")
    sec.byte(0x01)
    sec.leb(flush_core)


def emit_p2_run_command_world_sections(out: Writer) -> None:
    emit_section(out, 4, P2_RUN_INNER_COMPONENT)

    alias_sec = Writer()
    alias_sec.leb(1)
    alias_sec.byte(0x00)
    alias_sec.byte(0x00)
    alias_sec.byte(0x01)
    alias_sec.byte(0x05)
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

    inst = Writer()
    inst.leb(1)
    inst.byte(0x00)
    inst.leb(0)
    inst.leb(1)
    inst.string("import-func-run")
    inst.byte(0x01)
    inst.leb(0)
    emit_section(out, 5, inst.finish())

    export_sec = Writer()
    export_sec.leb(1)
    export_sec.byte(0x00)
    export_sec.string("wasi:cli/run@0.2.6")
    export_sec.byte(0x05)
    export_sec.leb(0)
    export_sec.byte(0x00)
    emit_section(out, 11, export_sec.finish())


def _wasm_tools() -> str | None:
    tool = shutil.which("wasm-tools")
    if tool:
        return tool
    cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
    return str(cargo) if cargo.is_file() else None


def _append_run_sections(component: bytes) -> bytes:
    out = Writer()
    out.bytes(component)
    emit_p2_run_command_world_sections(out)
    return out.finish()


def _wrap_via_wasm_tools(core_wasm: bytes) -> bytes | None:
    tool = _wasm_tools()
    if tool is None or not _WIT_DIR.is_dir():
        return None
    import tempfile

    patched = patch_guest_write_calls(core_wasm)
    sh = _SELFHOST_DIR
    with tempfile.TemporaryDirectory(prefix="p2-wrap-") as tmp:
        tmp_path = Path(tmp)
        core = tmp_path / "guest.core.wasm"
        emb = tmp_path / "guest.emb.wasm"
        new = tmp_path / "guest.component.wasm"
        core.write_bytes(patched)
        embed = subprocess.run(
            [
                tool,
                "component",
                "embed",
                "--world",
                "wasi:cli/imports@0.2.0",
                str(_WIT_DIR),
                str(core),
                "-o",
                str(emb),
            ],
            capture_output=True,
            text=True,
        )
        if embed.returncode != 0:
            return None
        adapt_stdout = sh / "p2_stdout_adapt.wat"
        cmd = [
            tool,
            "component",
            "new",
            str(emb),
            "--adapt",
            f"wasi:cli/stdout@0.2.0={adapt_stdout}",
            "--adapt",
            f"wasi:cli/environment@0.2.0={sh / 'p2_stub_env_adapt.wat'}",
            "--adapt",
            f"wasi:filesystem/types@0.2.0={sh / 'p2_stub_fs_adapt.wat'}",
            "--adapt",
            f"wasi:cli/stdin@0.2.0={sh / 'p2_stub_stdin_adapt.wat'}",
            "--adapt",
            f"wasi:cli/exit@0.2.0={sh / 'p2_stub_exit_adapt.wat'}",
            "-o",
            str(new),
        ]
        created = subprocess.run(cmd, capture_output=True, text=True)
        if created.returncode != 0:
            return None
        return _append_run_sections(new.read_bytes())


def wrap_p2_command_component(core_wasm: bytes) -> bytes:
    via_tools = _wrap_via_wasm_tools(core_wasm)
    if via_tools is not None:
        return via_tools
    return _wrap_p2_command_component_legacy(core_wasm)


def _wrap_p2_command_component_host(core_wasm: bytes) -> bytes:
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")
    out.bytes(P2_HOST_IMPORT_PREFIX.read_bytes())
    out.bytes(P2_STDIO_HOST_WIRING.read_bytes())

    bridge = load_bridge_module()
    for module in (
        bridge,
        stub_env_module(),
        stub_fs_module(),
        stub_single_export("read", False),
        stub_single_export("exit", True),
        core_wasm,
    ):
        emit_section(out, 1, module)

    inst_sec = Writer()
    inst_sec.leb(6)
    emit_bridge_instance_ref(inst_sec, get_stdout_core=0, flush_core=1)
    for module_index in range(1, 5):
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


def _wrap_p2_command_component_legacy(core_wasm: bytes) -> bytes:
    """Stub-only wrap (validate + run export shape; stdout stays empty)."""
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
