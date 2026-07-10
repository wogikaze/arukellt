#!/usr/bin/env python3
"""Wrap a P2 core wasm module in a wasi:cli/command component (bootstrap gate helper).

Builds a wasi:cli/command component with host imports for wasi:io/streams and
wasi:cli/stdout, wires the stdout bridge core module to blocking-write-and-flush,
and appends wasi:cli/run export sections.
"""

from __future__ import annotations

import argparse
import fcntl
import shutil
import subprocess
import sys
import time
from io import BytesIO
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))
from p2_guest_stdio_patch import patch_guest_core
from p2_guest_fs_patch import patch_guest_fs_writes
from p2_strip_imports import strip_wit_imports


def patch_guest_for_wrap(core_wasm: bytes) -> bytes:
    """Apply filesystem + stdio patches, then strip unused WIT imports."""
    patched = patch_guest_core(patch_guest_fs_writes(core_wasm))
    return strip_wit_imports(
        patched,
        ("wasi:clocks/", "wasi:random/"),
    )

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


def emit_core_instance_export_arg(
    sec: Writer, import_module: str, import_name: str, instance_index: int, export_name: str
) -> None:
    sec.string(import_name)
    sec.byte(0x02)
    sec.leb(instance_index)
    sec.string(export_name)


def emit_bridge_instance_ref(
    sec: Writer, get_stdout_inst: int, flush_inst: int, guest_inst: int
) -> None:
    sec.byte(0x00)
    sec.leb(0)
    sec.leb(3)
    sec.string("env")
    sec.string("get-stdout")
    sec.byte(0x12)
    sec.leb(get_stdout_inst)
    sec.string("env")
    sec.string("blocking-write-and-flush")
    sec.byte(0x12)
    sec.leb(flush_inst)
    emit_core_instance_export_arg(sec, "host", "memory", guest_inst, "memory")


def _run_export_section_payloads() -> dict[int, bytes]:
    alias_sec = Writer()
    alias_sec.leb(1)
    alias_sec.byte(0x00)
    alias_sec.byte(0x00)
    alias_sec.byte(0x01)
    alias_sec.byte(0x05)
    alias_sec.string("_start")
    type_sec = Writer()
    type_sec.leb(2)
    type_sec.byte(0x6A)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    type_sec.byte(0x40)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    type_sec.byte(0x00)
    canon_sec = Writer()
    canon_sec.leb(1)
    canon_sec.byte(0x00)
    canon_sec.byte(0x00)
    canon_sec.leb(0)
    canon_sec.leb(0)
    canon_sec.leb(1)
    inst = Writer()
    inst.leb(1)
    inst.byte(0x00)
    inst.leb(0)
    inst.leb(1)
    inst.string("import-func-run")
    inst.byte(0x01)
    inst.leb(0)
    export_sec = Writer()
    export_sec.leb(1)
    export_sec.byte(0x00)
    export_sec.string("wasi:cli/run@0.2.6")
    export_sec.byte(0x05)
    export_sec.leb(0)
    export_sec.byte(0x00)
    return {
        4: P2_RUN_INNER_COMPONENT,
        5: inst.finish(),
        6: alias_sec.finish(),
        7: type_sec.finish(),
        8: canon_sec.finish(),
        11: export_sec.finish(),
    }


def emit_p2_run_command_world_sections(out: Writer) -> None:
    payloads = _run_export_section_payloads()
    for section_id in (4, 6, 7, 8, 5, 11):
        emit_section(out, section_id, payloads[section_id])


def _parse_component_sections(component: bytes) -> dict[int, list[bytes]]:
    if component[:4] != b"\x00asm":
        raise ValueError("not a component")
    pos = 8
    sections: dict[int, list[bytes]] = {}
    while pos < len(component):
        section_id = component[pos]
        pos += 1
        section_size, pos = _leb_read_component(component, pos)
        payload = component[pos : pos + section_size]
        pos += section_size
        sections.setdefault(section_id, []).append(payload)
    return sections


def _leb_read_component(data: bytes, pos: int) -> tuple[int, int]:
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


def _emit_component_sections(sections: dict[int, list[bytes]]) -> bytes:
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")
    for section_id in sorted(sections):
        if section_id == 0:
            continue
        for payload in sections[section_id]:
            emit_section(out, section_id, payload)
    for payload in sections.get(0, []):
        emit_section(out, 0, payload)
    return out.finish()


def _run_export_section_payloads_embed() -> dict[int, bytes]:
    payloads = _run_export_section_payloads()
    return {
        4: payloads[4],
        5: payloads[5],
        6: payloads[6],
        10: payloads[8],
        2: payloads[5],
        11: payloads[11],
    }


def _merge_run_export_sections(component: bytes) -> bytes:
    sections = _parse_component_sections(component)
    for section_id, payload in _run_export_section_payloads().items():
        sections.setdefault(section_id, []).append(payload)
    return _emit_component_sections(sections)


def _merge_run_export_sections_embed(component: bytes) -> bytes:
    sections = _parse_component_sections(component)
    embed_payloads = _run_export_section_payloads_embed()
    # Run instance belongs in section 2 for wasm-tools embed layout.
    run_inst = _run_export_section_payloads()[5]
    sections.setdefault(2, []).append(run_inst)
    for section_id, payload in embed_payloads.items():
        if section_id == 2:
            continue
        sections.setdefault(section_id, []).append(payload)
    return _emit_component_sections(sections)


def _wasm_tools() -> str | None:
    cargo = Path.home() / ".cargo" / "bin" / "wasm-tools"
    if cargo.is_file():
        return str(cargo)
    return shutil.which("wasm-tools")


def _preview1_reactor_adapter() -> Path | None:
    """Locate WASI Preview 1 reactor adapter for guest preview1 imports."""
    repo = _SELFHOST_DIR.parents[1]
    candidates = [
        repo / "wasi_snapshot_preview1.reactor.wasm",
        repo / "bootstrap" / "wasi_snapshot_preview1.reactor.wasm",
        Path.home() / ".local" / "share" / "arukellt" / "wasi_snapshot_preview1.reactor.wasm",
        Path("/tmp/wasi_snapshot_preview1.reactor.wasm"),
    ]
    for path in candidates:
        if path.is_file():
            return path
    dest = repo / ".build" / "wasi_snapshot_preview1.reactor.wasm"
    if dest.is_file():
        return dest
    tool = _wasm_tools()
    if tool is None:
        return None
    import subprocess
    import urllib.request

    dest.parent.mkdir(parents=True, exist_ok=True)
    url = (
        "https://github.com/bytecodealliance/wasmtime/releases/download/"
        "v39.0.1/wasi_snapshot_preview1.reactor.wasm"
    )
    try:
        urllib.request.urlretrieve(url, dest)
    except Exception:
        return None
    if dest.is_file() and dest.stat().st_size > 0:
        return dest
    return None


def _append_run_sections(component: bytes) -> bytes:
    return _merge_run_export_sections(component)


def _wrap_via_wasm_tools(core_wasm: bytes) -> bytes | None:
    """Build command component via wasm-tools embed + component new + adapts."""
    return _with_wasm_tools_lock(lambda: _wrap_via_wasm_tools_locked(core_wasm))


def _wrap_via_wasm_tools_locked(core_wasm: bytes) -> bytes | None:
    tool = _wasm_tools()
    if tool is None or not _WIT_DIR.is_dir():
        return None
    import subprocess
    import tempfile

    patched = patch_guest_for_wrap(core_wasm)
    wit = _WIT_DIR
    fs_wit = wit / "deps" / "filesystem"
    adapt_wats = {
        "stdout": _SELFHOST_DIR / "p2_stdout_bridge_adapt.wat",
        "env": _SELFHOST_DIR / "p2_stub_env_adapt.wat",
        "fs": _SELFHOST_DIR / "p2_stub_fs_adapt.wat",
        "stdin": _SELFHOST_DIR / "p2_stub_stdin_adapt.wat",
        "exit": _SELFHOST_DIR / "p2_stub_exit_adapt.wat",
    }
    for path in adapt_wats.values():
        if not path.is_file():
            return None

    with tempfile.TemporaryDirectory(prefix="p2-wrap-") as tmp:
        tmp_path = Path(tmp)
        guest = tmp_path / "guest.core.wasm"
        guest_emb = tmp_path / "guest.emb.wasm"
        out = tmp_path / "guest.component.wasm"
        guest.write_bytes(patched)

        embed = subprocess.run(
            [
                tool,
                "component",
                "embed",
                "--world",
                "wasi:cli/command@0.2.0",
                str(wit),
                str(guest),
                "-o",
                str(guest_emb),
            ],
            capture_output=True,
            text=True,
        )
        if embed.returncode != 0:
            return None

        adapts: dict[str, Path] = {}
        embed_specs = [
            ("stdout", wit, "stdout-bridge-adapt", adapt_wats["stdout"]),
            ("env", wit, "environment-stub-adapt", adapt_wats["env"]),
            ("fs", fs_wit, "types-stub-adapt", adapt_wats["fs"]),
            ("stdin", wit, "stdin-stub-adapt", adapt_wats["stdin"]),
            ("exit", wit, "exit-stub-adapt", adapt_wats["exit"]),
        ]
        for key, wit_dir, world, wat in embed_specs:
            dest = tmp_path / f"adapt-{key}.wasm"
            step = subprocess.run(
                [
                    tool,
                    "component",
                    "embed",
                    "--world",
                    world,
                    str(wit_dir),
                    str(wat),
                    "-o",
                    str(dest),
                ],
                capture_output=True,
                text=True,
            )
            if step.returncode != 0:
                return None
            adapts[key] = dest

        new_cmd = [
            tool,
            "component",
            "new",
            str(guest_emb),
            "--import-name",
            "wasi:cli/stdout-host@0.2.0=wasi:cli/stdout@0.2.0",
            "--adapt",
            f"wasi:cli/stdout@0.2.0={adapts['stdout']}",
            "--adapt",
            f"wasi:cli/environment@0.2.0={adapts['env']}",
            "--adapt",
            f"wasi:filesystem/types@0.2.0={adapts['fs']}",
            "--adapt",
            f"wasi:cli/stdin@0.2.0={adapts['stdin']}",
            "--adapt",
            f"wasi:cli/exit@0.2.0={adapts['exit']}",
        ]
        preview1 = _preview1_reactor_adapter()
        if preview1 is not None:
            new_cmd.extend(["--adapt", f"wasi_snapshot_preview1={preview1}"])
        new_cmd.extend(["-o", str(out)])
        new = subprocess.run(
            new_cmd,
            capture_output=True,
            text=True,
        )
        if new.returncode != 0:
            return None
        result = subprocess.run(
            [tool, "validate", str(out)],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            return None
        return out.read_bytes()


def _wrap_p2_command_component_bridged(core_wasm: bytes) -> bytes:
    """Host-import prefix + stdout bridge core instance (no 28KB P1 adapt)."""
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")
    out.bytes(P2_HOST_IMPORT_PREFIX.read_bytes())

    bridge = load_bridge_module()
    guest = patch_guest_for_wrap(core_wasm)
    for module in (
        bridge,
        stub_env_module(),
        stub_fs_module(),
        stub_single_export("read", False),
        stub_single_export("exit", True),
        guest,
    ):
        emit_section(out, 1, module)

    inst_sec = Writer()
    inst_sec.leb(6)
    emit_bridge_instance_ref(inst_sec, get_stdout_inst=0, flush_inst=1, guest_inst=5)
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


_WRAP_LOCK = Path(__file__).resolve().parents[2] / ".build" / "p2-component-wrap.lock"
_WASM_TOOLS_LOCK = Path(__file__).resolve().parents[2] / ".build" / "wasm-tools-component.lock"


def _with_wasm_tools_lock(fn):
    _WASM_TOOLS_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _WASM_TOOLS_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return fn()


def _with_wrap_lock(fn):
    _WRAP_LOCK.parent.mkdir(parents=True, exist_ok=True)
    with _WRAP_LOCK.open("w", encoding="utf-8") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        return fn()


def wrap_p2_command_component(core_wasm: bytes) -> bytes:
    return _with_wrap_lock(lambda: _wrap_p2_command_component_locked(core_wasm))


def _wrap_p2_command_component_locked(core_wasm: bytes) -> bytes:
    tool = _wasm_tools()
    if tool is not None:
        for attempt in range(10):
            via_tools = _wrap_via_wasm_tools(core_wasm)
            if via_tools is not None:
                return via_tools
            if attempt < 9:
                time.sleep(0.25)
        # Fall through to bridged/legacy when wasm-tools embed+new
        # rejects the guest (e.g. invalid rethrow label on newer wasm-tools).
    via_tools = _wrap_via_wasm_tools(core_wasm)
    if via_tools is not None:
        return via_tools
    try:
        bridged = _wrap_p2_command_component_bridged(core_wasm)
        if tool:
            import subprocess
            import tempfile

            with tempfile.NamedTemporaryFile(suffix=".wasm", delete=False) as tmp:
                tmp.write(bridged)
                tmp_path = tmp.name
            result = subprocess.run(
                [tool, "validate", tmp_path],
                capture_output=True,
                text=True,
            )
            Path(tmp_path).unlink(missing_ok=True)
            if result.returncode == 0:
                return bridged
    except Exception:
        pass
    return _wrap_p2_command_component_legacy(core_wasm)


def _wrap_p2_command_component_host(core_wasm: bytes) -> bytes:
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")
    out.bytes(P2_HOST_IMPORT_PREFIX.read_bytes())
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
    emit_bridge_instance_ref(inst_sec, get_stdout_inst=0, flush_inst=1, guest_inst=5)
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
    guest = patch_guest_for_wrap(core_wasm)
    out = Writer()
    out.bytes(b"\x00asm\x0d\x00\x01\x00")

    for module in (
        stub_single_export("write", False),
        stub_env_module(),
        stub_fs_module(),
        stub_single_export("read", False),
        stub_single_export("exit", True),
        guest,
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
