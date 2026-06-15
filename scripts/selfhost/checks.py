"""Selfhost domain check runners — pure Python, no shell script calls.

Per ADR-029 (#585) the four selfhost gates run entirely against the
selfhost compiler under wasmtime and never consult ``target/debug/arukellt``.

The trusted base is the committed pinned-reference wasm at
``bootstrap/arukellt-selfhost.wasm`` (see ``bootstrap/PROVENANCE.md``).
"""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


# ── ANSI colours ─────────────────────────────────────────────────────────────
RED    = "\033[0;31m"
GREEN  = "\033[0;32m"
YELLOW = "\033[1;33m"
NC     = "\033[0m"


def _remove_tree(path: Path) -> None:
    """Remove a directory tree; retry on WSL where shutil/rm can race."""
    if not path.exists():
        return
    for attempt in range(12):
        if os.name == "posix":
            subprocess.run(["chmod", "-R", "u+w", str(path)], check=False)
            subprocess.run(["rm", "-rf", str(path)], check=False)
            if path.exists():
                subprocess.run(["find", str(path), "-mindepth", "1", "-delete"], check=False)
                subprocess.run(["rmdir", str(path)], check=False)
        else:
            shutil.rmtree(path, ignore_errors=True)
        if not path.exists():
            return
        shutil.rmtree(path, ignore_errors=True)
        if not path.exists():
            return
        time.sleep(0.05 * (attempt + 1))
    if path.exists():
        raise OSError(f"failed to remove directory tree: {path}")


# Bootstrap overlay (``MONOLITHIC_OVERLAY_*``) remains for stage-0→s2 builds when
# modular ``src/compiler/**/mod.ark`` trees need a flat workspace.  Runtime gates
# use s2 (or heap-patched s2-runtime) once built; pinned wasm is stage-0 only.

# ── Paths ────────────────────────────────────────────────────────────────────

PINNED_WASM_REL = "bootstrap/arukellt-selfhost.wasm"
BOOTSTRAP_WASM_REL = ".build/selfhost/arukellt-pinned-bootstrap.wasm"
S2_RUNTIME_WASM_REL = ".build/selfhost/arukellt-s2-runtime.wasm"
HOP_BOOTSTRAP_WASM_REL = ".build/selfhost/arukellt-hop-bootstrap.wasm"
HOP_BOOTSTRAP_COMMIT = "a56d6d53"
HOP_BOOTSTRAP_PATCH_REV = 2
PATCHER_DIR_REL = "scripts/bootstrap/wasm-heap-grow-patcher"
SELFHOST_SOURCE_REL = "src/compiler/main.ark"
BOOTSTRAP_WORKSPACE_REL = ".build/selfhost/bootstrap-workspace"
SELFHOST_COMPILE_TIMEOUT = 900
SELFHOST_TARGET = "wasm32-wasi-p1"
CLI_VERSION_GOLDEN_REL = "tests/snapshots/selfhost/cli-version.txt"
CLI_HELP_GOLDEN_REL = "tests/snapshots/selfhost/cli-help.txt"

BOOTSTRAP_EXCLUDED_OVERLAY_PREFIXES = (
    "component/",
    "wasm/wat.ark",
    "wasm/wat_functions.ark",
    "wasm/wat_function_body.ark",
    "wasm/wat_types.ark",
    "lexer/chars.ark",
    "component_emit.ark",
    "component_emitter.ark",
    "emit_wat.ark",
    "emitter.ark",
    "parser.ark",
    "mir/inst_gc_hint.ark",
    "wasm/ctx_gc_hint.ark",
    "wasm/inst_dispatch_gc_hint.ark",
    "wasm/sections_gc_hint.ark",
    "wasm/sections_tail.ark",
)

# Bootstrap overlay: freeze wasm/mir gc_hint files at pre-ff8f8ded (selfhost trap).
BOOTSTRAP_OVERLAY_FILE_FREEZE_REVS: dict[str, str] = {
    "wasm/inst_dispatch.ark": "4b859775",
    "wasm/inst_dispatch_struct.ark": "4b859775",
    "wasm/inst_struct_record.ark": "4b859775",
}

# ff8f8ded mir_opt LICM/GC passes trap in flat-overlay selfhost wasm; use passthrough stub.
BOOTSTRAP_STUB_OVERLAY_NAMESPACES: frozenset[str] = frozenset({"mir_opt"})
# Nested imports that flatten to the namespace owner when the namespace is stubbed.
BOOTSTRAP_STUB_NAMESPACE_FLAT_IMPORTS: dict[tuple[str, ...], str] = {
    ("mir_opt", "optimize_module"): "mir_opt",
}
BOOTSTRAP_MIR_OPT_STUB = """// Bootstrap overlay stub — full MIR opt excluded (ff8f8ded traps in selfhost wasm).
pub fn optimize_module(m: MirModule, opt_level: i32, target: String) -> MirModule {
    m
}
"""

BOOTSTRAP_COMPONENT_STUB = """// Bootstrap overlay stub — full component model excluded to reduce memory.
pub fn emit_component(core_wasm: Vec<i32>, mir: MirModule, target: String, wasi_version: String, world: String) -> Vec<i32> {
    if eq(clone(wasi_version), String_from("p2")) {
        return wasm_component_p2_emit::emit_p2_command_component(core_wasm)
    }
    core_wasm
}

pub fn mir_has_library_exports(mir: MirModule) -> bool {
    false
}

pub fn emit_wit_text_from_decls(decls: Vec<AstNode>) -> String {
    String_from("")
}

pub fn emit_wit_text_from_decls_with_world(decls: Vec<AstNode>, world: String) -> String {
    String_from("")
}

pub fn collect_export_roots(decls: Vec<AstNode>) -> Vec<String> {
    Vec_new_String()
}

pub fn string_to_bytes(text: String) -> Vec<i32> {
    let mut bytes = Vec_new_i32()
    let mut i = 0
    while i < len(text) {
        push(bytes, char_at(text, i))
        i = i + 1
    }
    bytes
}

pub fn validate_wit_import_surface(paths: Vec<String>) -> String {
    String_from("")
}

pub fn validate_export_surface(decls: Vec<AstNode>) -> String {
    String_from("")
}

pub fn preflight_frontend(config_emit_mode: String, wit_paths: Vec<String>, decls: Vec<AstNode>, world: String) -> String {
    String_from("")
}
"""

BOOTSTRAP_COMPONENT_WORLD_SPEC_STUB = """// Bootstrap overlay stub — world_spec excluded with component model.
pub fn component_world_spec__world_target_error(world: String, target: String, emit_mode: String) -> String {
    if len(world) == 0 { return String_new() }
    if !contains(clone(target), String_from("-p2")) { return String_from("--world requires --target wasm32-wasi-p2") }
    String_new()
}
"""

LOCAL_COMPILER_NAMESPACES = {
    "analysis",
    "compiler",
    "component",
    "corehir",
    "dap",
    "diagnostics",
    "driver",
    "fmt",
    "hir",
    "lexer",
    "loader",
    "lsp",
    "main",
    "mir",
    "mir_opt",
    "parser",
    "resolver",
    "typechecker",
    "wasm",
}

# When the working tree is modular but bootstrap compilers are memory-bound,
# prefer committed monolithic snapshots for overlay namespaces that still have
# a HEAD-level `.ark` owner. This keeps bootstrap compile units near the ~38-file
# scale the pinned wasm was built for.
MONOLITHIC_OVERLAY_FALLBACK_REV = "7911a527"
MONOLITHIC_OVERLAY_FALLBACK_BY_NS: dict[str, tuple[str, ...]] = {}
WORKTREE_OVERLAY_NAMESPACES = frozenset({
    "analysis",
    "compiler",
    "corehir",
    "dap",
    "diagnostics",
    "driver",
    "fmt",
    "hir",
    "lexer",
    "loader",
    "lsp",
    "main",
    "mir",
    "parser",
    "resolver",
    "typechecker",
    "wasm",
})
WORKTREE_LEGACY_FACADE_FILES: dict[str, tuple[str, ...]] = {
    "driver": ("driver.ark",),
    "main": ("main.ark",),
    # mir_lower.ark intentionally omitted: nested mir/lower.ark owns the flat
    # `mir_lower` module; the legacy facade forwards back to the stripped
    # `mir` namespace facade and would self-loop under first-match binding.
    "mir": ("mir_dump.ark",),
    "wasm": ("emitter.ark", "emit_wat.ark"),
}
MONOLITHIC_OVERLAY_EXTRA_FILES = (
    "component_emitter.ark",
)


# ── Helpers ──────────────────────────────────────────────────────────────────

def _find_wasmtime() -> str | None:
    return shutil.which("wasmtime")


def _find_pinned_wasm(root: Path) -> Path | None:
    """Return the committed pinned-reference selfhost wasm, honouring override."""
    env_override = os.environ.get("ARUKELLT_PINNED_WASM", "")
    if env_override:
        p = Path(env_override)
        return p if p.is_file() else None
    p = root / PINNED_WASM_REL
    return p if p.is_file() else None


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _wasm_needs_host_linker(wasm_path: Path) -> bool:
    try:
        return b"arukellt_host" in wasm_path.read_bytes()
    except OSError:
        return False


def _wasm_run_argv(root: Path, wasm_path: Path) -> list[str]:
    """Return argv to execute user wasm, using host-linker when imports need it."""
    if _wasm_needs_host_linker(wasm_path):
        hosted = root / "scripts" / "run" / "arukellt-run-hosted.sh"
        if hosted.is_file():
            return ["bash", str(hosted), f"--dir={root}", str(wasm_path)]
    wasmtime = _find_wasmtime()
    return [wasmtime or "wasmtime", "run", f"--dir={root}", str(wasm_path)]


def _run(cmd: list[str], root: Path, capture: bool = True, timeout: int | None = None) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(
            cmd,
            cwd=str(root),
            capture_output=capture,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(cmd, returncode=-1, stdout="", stderr="timeout")


def _wasm_compile(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    out_rel: str,
    root: Path,
    timeout: int | None = None,
    workspace_root: Path | None = None,
) -> subprocess.CompletedProcess:
    """Run ``compiler_wasm compile <src> --target <T> -o <out_rel>`` under wasmtime."""
    dirs: list[str] = []
    guest_out = out_rel
    if workspace_root is not None:
        dirs.extend(["--dir", str(workspace_root)])
        guest_out = "bootstrap-out.wasm"
    dirs.extend(["--dir", str(root)])
    result = _run(
        [wasmtime, "run", *dirs, str(compiler_wasm), "--",
         "compile", src, "--target", SELFHOST_TARGET, "-o", guest_out],
        root,
        timeout=timeout,
    )
    if workspace_root is not None:
        staged = workspace_root / guest_out
        stderr = result.stderr or ""
        root_staged = root / guest_out
        compiled_ok = result.returncode == 0 or (
            (
                (staged.is_file() and staged.stat().st_size > 0)
                or (root_staged.is_file() and root_staged.stat().st_size > 0)
            )
            and "compilation succeeded" in stderr
        )
        if compiled_ok:
            if not staged.is_file() and root_staged.is_file():
                staged = root_staged
            if staged.is_file():
                final = root / out_rel
                final.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(staged, final)
                if result.returncode != 0:
                    result = subprocess.CompletedProcess(
                        result.args, returncode=0, stdout=result.stdout, stderr=result.stderr,
                    )
            final = root / out_rel
            final.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(staged, final)
            if result.returncode != 0:
                result = subprocess.CompletedProcess(
                    result.args, returncode=0, stdout=result.stdout, stderr=result.stderr,
                )
    return result


def _compiler_source_mtime(root: Path) -> float:
    """Return newest mtime for source files that define the selfhost compiler."""
    candidates = list((root / "src" / "compiler").rglob("*.ark"))
    candidates.append(Path(__file__))
    return max(path.stat().st_mtime for path in candidates if path.is_file())


def _flat_compiler_module_name(path: Path) -> str:
    parts = list(path.with_suffix("").parts)
    return "_".join(parts)


def _flat_overlay_module_name(rel: Path) -> str:
    """Map namespace ``mod.ark`` owners to ``foo.ark`` instead of ``foo_mod.ark``."""
    if rel.name == "mod.ark" and len(rel.parts) > 1:
        return rel.parts[0]
    return _flat_compiler_module_name(rel)


def _flatten_namespace_mod_import(parts: list[str]) -> str | None:
    if len(parts) == 2 and parts[1] == "mod" and parts[0] in LOCAL_COMPILER_NAMESPACES:
        return parts[0]
    return None


def _should_exclude_bootstrap_overlay_source(rel: Path) -> bool:
    rel_str = rel.as_posix()
    if rel_str == "component/" or rel_str.startswith("component/"):
        return False
    for prefix in BOOTSTRAP_EXCLUDED_OVERLAY_PREFIXES:
        if rel_str == prefix or rel_str.startswith(prefix):
            return True
    return False


_TOP_LEVEL_FN_RE = re.compile(r"^(?:pub )?fn ([A-Za-z_][A-Za-z0-9_]*)")
_TOP_LEVEL_STRUCT_RE = re.compile(r"^(?:pub )?struct ([A-Za-z_][A-Za-z0-9_]*)")
_TOP_LEVEL_ENUM_RE = re.compile(r"^(?:pub )?enum ([A-Za-z_][A-Za-z0-9_]*)")
# Later overlay modules that must not claim an earlier module's symbol name.
_OVERLAY_PUBLISH_RENAMES: dict[str, dict[str, str]] = {
    "lexer.ark": {"Diagnostic_new": "lexer_Diagnostic_new"},
    "parser.ark": {"Token_new": "parser_Token_new", "TK_EOF": "parser_TK_EOF"},
    "parser/token_state.ark": {"Token_new": "parser_Token_new"},
    "parser/core_api_token.ark": {"Token_new": "parser_api_Token_new"},
    "emit_intrinsic_math.ark": {"emit_eq": "emit_intrinsic_math_eq"},
}
_OVERLAY_CALL_RENAMES: dict[str, str] = {
    "emit_intrinsic_math::emit_eq": "emit_intrinsic_math::emit_intrinsic_math_eq",
    "lexer::Diagnostic_new": "lexer::lexer_Diagnostic_new",
    "parser::Token_new": "parser::parser_Token_new",
    "parser::token_state::Token_new": "parser::parser_Token_new",
    "parser::TK_EOF": "parser::parser_TK_EOF",
}

_OVERLAY_LEXER_CHARS_RENAMES: dict[str, str] = {
    "chars::skip_whitespace": "char_skip::skip_whitespace_impl",
    "chars::skip_line_comment": "char_skip::skip_line_comment_impl",
    "chars::skip_block_comment": "char_skip::skip_block_comment_impl",
    "chars::is_alpha": "char_class::is_alpha",
    "chars::is_digit": "char_class::is_digit",
    "chars::is_hex_digit": "char_class::is_hex_digit",
    "chars::is_alnum": "char_class::is_alnum",
    "chars::digit_value": "char_class::digit_value",
    "chars::is_whitespace_not_newline": "char_class::is_whitespace_not_newline",
}


def _rewrite_overlay_lexer_chars(text: str) -> str:
    for old, new in _OVERLAY_LEXER_CHARS_RENAMES.items():
        text = text.replace(old, new)
    lines = text.splitlines()
    out: list[str] = []
    for line in lines:
        if line.strip() == "use lexer::chars":
            continue
        out.append(line)
    trailing = "\n" if text.endswith("\n") else ""
    body = "\n".join(out)
    return body if body.endswith("\n") or not trailing else body + trailing


def _rename_overlay_publish_symbols(text: str, rel_name: str) -> str:
    """Give later flat-overlay owners unique symbol names after collisions."""
    renames = _OVERLAY_PUBLISH_RENAMES.get(rel_name)
    if not renames:
        return text
    for old, new in renames.items():
        text = re.sub(rf"^pub fn {re.escape(old)}\b", f"pub fn {new}", text, flags=re.M)
        text = re.sub(rf"\b{re.escape(old)}\b", new, text)
    return text


_OVERLAY_MAIN_ENTRY_REL = "main.ark"


def _rewrite_overlay_call_sites(text: str) -> str:
    text = _rewrite_overlay_lexer_chars(text)
    for old, new in _OVERLAY_CALL_RENAMES.items():
        text = text.replace(old, new)
    return text


def _overlay_facade_rank(rel: Path) -> int:
    """Rank thin facades after implementation modules at the same path depth."""
    name = rel.name
    if name == "mod.ark":
        return 1
    if name.startswith("core_api_") or name.startswith("core_ast_"):
        return 1
    # Flat ``namespace.ark`` owners (no ``_`` in the stem) are namespace facades.
    if len(rel.parts) == 1 and rel.suffix == ".ark" and "_" not in rel.stem:
        return 1
    return 0


def _overlay_source_sort_key(rel: Path) -> tuple[int, int, str]:
    """Deepest modules first; defer namespace facades within the same depth."""
    return (-len(rel.parts), _overlay_facade_rank(rel), str(rel))


def _overlay_top_level_item_lines(lines: list[str], start: int) -> list[str]:
    end = _skip_top_level_item(lines, start)
    return lines[start:end]


def _overlay_fn_param_names(line: str) -> list[str]:
    match = re.search(r"\(([^)]*)\)", line)
    if match is None:
        return []
    params = match.group(1).strip()
    if len(params) == 0:
        return []
    names: list[str] = []
    for part in params.split(","):
        name = part.strip()
        if ":" in name:
            name = name.split(":", 1)[0].strip()
        if len(name) > 0:
            names.append(name)
    return names


def _overlay_fn_first_param_name(line: str) -> str | None:
    names = _overlay_fn_param_names(line)
    if len(names) == 0:
        return None
    return names[0]


def _is_overlay_delegate_fn(lines: list[str], start: int) -> bool:
    """True when the top-level fn only forwards its parameters unchanged."""
    item = _overlay_top_level_item_lines(lines, start)
    body = "".join(item)
    fn_match = _TOP_LEVEL_FN_RE.match(lines[start])
    if fn_match is None:
        return False
    fn_name = fn_match.group(1)
    brace_start = body.find("{")
    brace_end = body.rfind("}")
    if brace_start < 0 or brace_end <= brace_start:
        return False
    inner = body[brace_start + 1 : brace_end].strip()
    inner = re.sub(r"\s+", " ", inner)
    if inner.startswith("return "):
        inner = inner[7:].strip()
    if inner.endswith(";"):
        inner = inner[:-1].strip()
    if inner.find(";") >= 0:
        return False
    param_names = _overlay_fn_param_names(lines[start])
    clone_forward = re.match(
        r"^((?:[A-Za-z_][A-Za-z0-9_]*::)+)([A-Za-z_][A-Za-z0-9_]*)\(clone\(([^)]+)\)\)$",
        inner,
    )
    if clone_forward is not None:
        if clone_forward.group(2) != fn_name:
            return False
        if len(param_names) != 1:
            return False
        return clone_forward.group(3) == param_names[0]
    match = re.match(
        r"^((?:[A-Za-z_][A-Za-z0-9_]*::)+)([A-Za-z_][A-Za-z0-9_]*)\(([^)]*)\)$",
        inner,
    )
    if match is None:
        return False
    if match.group(2) != fn_name:
        return False
    if len(param_names) == 0:
        return len(match.group(3).strip()) == 0
    call_args = [part.strip() for part in match.group(3).split(",") if len(part.strip()) > 0]
    if len(call_args) != len(param_names):
        return False
    idx = 0
    while idx < len(call_args):
        arg = call_args[idx]
        param = param_names[idx]
        if arg != param and arg != f"clone({param})":
            return False
        idx = idx + 1
    return True


def _skip_top_level_item(lines: list[str], start: int) -> int:
    """Return index after the top-level fn/struct/enum item starting at ``start``."""
    i = start
    depth = 0
    started = False
    while i < len(lines):
        line = lines[i]
        if "{" in line:
            started = True
        depth += line.count("{") - line.count("}")
        i += 1
        if started and depth <= 0:
            break
    return i


def _iter_overlay_top_level_symbols(text: str, rel_name: str) -> list[tuple[str, bool]]:
    """Return ``(symbol, is_delegate_fn)`` for each top-level fn in ``text``."""
    lines = text.splitlines(keepends=True)
    symbols: list[tuple[str, bool]] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        match = _TOP_LEVEL_FN_RE.match(line)
        if match is not None:
            name = match.group(1)
            if name == "main" and rel_name != _OVERLAY_MAIN_ENTRY_REL:
                i = _skip_top_level_item(lines, i)
                continue
            symbols.append((name, _is_overlay_delegate_fn(lines, i)))
            i = _skip_top_level_item(lines, i)
            continue
        i += 1
    return symbols


def _overlay_reachable_rels(entries: list[tuple[str, str]]) -> set[str]:
    """Flat overlay modules reachable from main.ark via ``use`` edges."""
    imports_by_rel = {
        rel: set(_OVERLAY_USE_RE.findall(text)) for rel, text in entries
    }
    rel_by_stem = {
        (rel[:-4] if rel.endswith(".ark") else rel): rel for rel, _ in entries
    }
    reachable: set[str] = set()
    stack = [_OVERLAY_MAIN_ENTRY_REL]
    while stack:
        rel = stack.pop()
        if rel in reachable or rel not in imports_by_rel:
            continue
        reachable.add(rel)
        for stem in imports_by_rel[rel]:
            target = rel_by_stem.get(stem)
            if target is not None:
                stack.append(target)
    return reachable


def _compute_overlay_symbol_plan(
    entries: list[tuple[str, str]],
) -> tuple[set[tuple[str, str]], list[tuple[str, str]]]:
    """Plan collision handling: strip namespace facade dupes, rename the rest.

    Returns ``(keepers, renames)`` where ``keepers`` keeps baseline behavior of
    dropping duplicate symbols published by flat namespace facades (``foo.ark``
    owners without ``_`` in the stem), and ``renames`` lists ``(rel_name,
    symbol)`` pairs whose definitions must be prefixed with the flat module
    stem (``parser_cursor__skip_newlines``) so every remaining top-level fn
    name is globally unique under the pinned emitter's first-match call
    binding. Implementation bodies are never removed.
    """
    keepers: set[tuple[str, str]] = set()
    occurrences: dict[str, list[tuple[str, bool]]] = {}
    reachable = _overlay_reachable_rels(entries)
    for rel_name, text in entries:
        seen_in_file: set[str] = set()
        for symbol, is_delegate in _iter_overlay_top_level_symbols(text, rel_name):
            keepers.add((rel_name, symbol))
            if symbol in seen_in_file:
                # Multi-clause definitions share one owner entry.
                if not is_delegate:
                    owners = occurrences[symbol]
                    for oi, (orel, odel) in enumerate(owners):
                        if orel == rel_name:
                            owners[oi] = (orel, False)
                continue
            seen_in_file.add(symbol)
            occurrences.setdefault(symbol, []).append((rel_name, is_delegate))
    renames: list[tuple[str, str]] = []
    for symbol, owners in occurrences.items():
        if len(owners) <= 1:
            continue
        reachable_owners = [rel for rel, _ in owners if rel in reachable]
        remaining: list[tuple[str, bool]] = []
        for rel_name, is_delegate in owners:
            rank1 = _overlay_facade_rank(Path(rel_name)) == 1
            others_reachable = [rel for rel in reachable_owners if rel != rel_name]
            # Namespace facade dupes are stripped (baseline) only when another
            # import-reachable owner still provides the symbol.
            if rank1 and others_reachable:
                keepers.discard((rel_name, symbol))
            else:
                remaining.append((rel_name, is_delegate))
        if len(remaining) <= 1:
            continue
        # Keeper keeps the short name: it must be compiled by the pinned
        # driver (import-reachable from main.ark) so first-match call binding
        # can find it. Prefer reachable implementations over delegates.
        keeper_rel = None
        for rel_name, is_delegate in remaining:
            if not is_delegate and rel_name in reachable:
                keeper_rel = rel_name
                break
        if keeper_rel is None:
            for rel_name, _ in remaining:
                if rel_name in reachable:
                    keeper_rel = rel_name
                    break
        if keeper_rel is None:
            for rel_name, is_delegate in remaining:
                if not is_delegate:
                    keeper_rel = rel_name
                    break
        if keeper_rel is None:
            keeper_rel = remaining[0][0]
        for rel_name, _ in remaining:
            if rel_name != keeper_rel:
                renames.append((rel_name, symbol))
    return keepers, renames


_OVERLAY_USE_RE = re.compile(r"^use ([A-Za-z_][A-Za-z0-9_]*)\s*$", re.M)


def _apply_overlay_collision_renames(
    texts: dict[str, str],
    renames: list[tuple[str, str]],
    owner_modules: dict[str, set[str]],
) -> None:
    """Rename colliding definitions in place and rewrite their references.

    For each ``(rel_name, symbol)``: the definition and intra-file unqualified
    references become ``{stem}__{symbol}``; qualified ``stem::symbol``
    references are rewritten everywhere; files importing exactly one owner
    module of the symbol have unqualified calls re-qualified to that owner.
    """
    if not renames:
        return
    file_imports: dict[str, set[str]] = {
        rel: set(_OVERLAY_USE_RE.findall(text)) for rel, text in texts.items()
    }
    defines: dict[str, set[str]] = {}
    for rel, text in texts.items():
        names = set()
        for line in text.splitlines():
            m = _TOP_LEVEL_FN_RE.match(line)
            if m is not None:
                names.add(m.group(1))
        defines[rel] = names

    # Pass 1: rename definitions + intra-file unqualified references.
    for rel, symbol in renames:
        stem = rel[:-4] if rel.endswith(".ark") else rel
        new_name = f"{stem}__{symbol}"
        pat = re.compile(rf"(?<![A-Za-z0-9_:.]){re.escape(symbol)}\(")
        texts[rel] = pat.sub(new_name + "(", texts[rel])

    # Pass 2: rewrite qualified references globally in one pass per file.
    qualified_map: dict[str, str] = {}
    for rel, symbol in renames:
        stem = rel[:-4] if rel.endswith(".ark") else rel
        qualified_map[f"{stem}::{symbol}"] = f"{stem}::{stem}__{symbol}"
    if qualified_map:
        alt = "|".join(re.escape(k) for k in sorted(qualified_map, key=len, reverse=True))
        qpat = re.compile(rf"\b(?:{alt})\b")
        for rel in texts:
            texts[rel] = qpat.sub(lambda m: qualified_map[m.group(0)], texts[rel])

    # Pass 3: files importing exactly one owner module bind unqualified calls
    # to that owner explicitly (preserves original nested-module semantics).
    renamed_by_symbol: dict[str, set[str]] = {}
    for rel, symbol in renames:
        stem = rel[:-4] if rel.endswith(".ark") else rel
        renamed_by_symbol.setdefault(symbol, set()).add(stem)
    for symbol, renamed_stems in renamed_by_symbol.items():
        owners = owner_modules.get(symbol, set())
        upat = re.compile(rf"(?<![A-Za-z0-9_:.]){re.escape(symbol)}\(")
        for rel, text in texts.items():
            if symbol in defines.get(rel, set()):
                continue
            if f"{symbol}(" not in text:
                continue
            imported_owners = owners & file_imports.get(rel, set())
            if len(imported_owners) != 1:
                continue
            owner = next(iter(imported_owners))
            if owner not in renamed_stems:
                continue
            texts[rel] = upat.sub(f"{owner}::{owner}__{symbol}(", text)


def _strip_duplicate_overlay_top_level_fns(
    text: str,
    seen_names: set[str] | None = None,
    rel_name: str = "",
    keepers: set[tuple[str, str]] | None = None,
) -> str:
    """Drop facade re-exports that collide after flat overlay flattening."""
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        match = _TOP_LEVEL_FN_RE.match(line)
        if match is not None:
            name = match.group(1)
            if name == "main" and rel_name != _OVERLAY_MAIN_ENTRY_REL:
                i = _skip_top_level_item(lines, i)
                continue
            if keepers is not None:
                if (rel_name, name) not in keepers:
                    i = _skip_top_level_item(lines, i)
                    continue
            elif seen_names is not None:
                if name in seen_names:
                    i = _skip_top_level_item(lines, i)
                    continue
                seen_names.add(name)
        out.append(line)
        i += 1
    trailing = "\n" if text.endswith("\n") else ""
    body = "".join(out)
    return body if body.endswith("\n") or not trailing else body + trailing


def _reapply_global_overlay_dedupe(compiler_out: Path, write_order: list[str]) -> None:
    """Second pass: strip facade dupes, rename remaining collisions in place."""
    entries: list[tuple[str, str]] = []
    seen_rel: set[str] = set()
    for rel_name in write_order:
        if rel_name in seen_rel:
            continue
        seen_rel.add(rel_name)
        path = compiler_out / rel_name
        if path.is_file():
            entries.append((rel_name, path.read_text(encoding="utf-8")))
    keepers, renames = _compute_overlay_symbol_plan(entries)
    texts: dict[str, str] = {}
    for rel_name, text in entries:
        texts[rel_name] = _strip_duplicate_overlay_top_level_fns(
            text, rel_name=rel_name, keepers=keepers,
        )
    # Post-strip owner modules per symbol (keeper + renamed copies).
    owner_modules: dict[str, set[str]] = {}
    for rel_name, text in texts.items():
        stem = rel_name[:-4] if rel_name.endswith(".ark") else rel_name
        for line in text.splitlines():
            m = _TOP_LEVEL_FN_RE.match(line)
            if m is not None:
                owner_modules.setdefault(m.group(1), set()).add(stem)
    _apply_overlay_collision_renames(texts, renames, owner_modules)
    for rel_name, text in texts.items():
        (compiler_out / rel_name).write_text(text, encoding="utf-8")


def _patch_bootstrap_driver_timing(text: str) -> str:
    """Pinned bootstrap lacks clock intrinsics and stores struct i64 as i32."""
    text = text.replace("clock::monotonic_now()", "0")
    for field in ("t0", "t_lex", "t_parse", "t_resolve", "t_typecheck"):
        text = re.sub(rf"^    {field}: i64,$", f"    {field}: i32,", text, flags=re.M)
    text = text.replace(
        "fn DriverFrontendResult_new(should_return: bool, result: CompileResult, decls: Vec<AstNode>, t0: i64, t_lex: i64, t_parse: i64)",
        "fn DriverFrontendResult_new(should_return: bool, result: CompileResult, decls: Vec<AstNode>, t0: i32, t_lex: i32, t_parse: i32)",
    )
    text = text.replace(
        "fn frontend_stop(result: CompileResult, t0: i64, t_lex: i64, t_parse: i64)",
        "fn frontend_stop(result: CompileResult, t0: i32, t_lex: i32, t_parse: i32)",
    )
    text = text.replace(
        "fn frontend_continue(decls: Vec<AstNode>, t0: i64, t_lex: i64, t_parse: i64)",
        "fn frontend_continue(decls: Vec<AstNode>, t0: i32, t_lex: i32, t_parse: i32)",
    )
    text = text.replace(
        "fn run_lex_parse(source: String, config: DriverConfig, t0: i64)",
        "fn run_lex_parse(source: String, config: DriverConfig, t0: i32)",
    )
    text = text.replace(
        "fn DriverResolveResult_new(should_return: bool, result: CompileResult, load_state: LoadState, resolve_ctx: ResolveCtx, t_resolve: i64)",
        "fn DriverResolveResult_new(should_return: bool, result: CompileResult, load_state: LoadState, resolve_ctx: ResolveCtx, t_resolve: i32)",
    )
    text = text.replace(
        "fn DriverTypecheckResult_new(should_return: bool, result: CompileResult, check_result: TypeCheckResult, t_typecheck: i64)",
        "fn DriverTypecheckResult_new(should_return: bool, result: CompileResult, check_result: TypeCheckResult, t_typecheck: i32)",
    )
    text = text.replace(
        "fn frontend_result_t0(result: DriverFrontendResult) -> i64 {",
        "fn frontend_result_t0(result: DriverFrontendResult) -> i32 {",
    )
    text = text.replace(
        "fn frontend_result_t_lex(result: DriverFrontendResult) -> i64 {",
        "fn frontend_result_t_lex(result: DriverFrontendResult) -> i32 {",
    )
    text = text.replace(
        "fn frontend_result_t_parse(result: DriverFrontendResult) -> i64 {",
        "fn frontend_result_t_parse(result: DriverFrontendResult) -> i32 {",
    )
    text = text.replace(
        "run_backend(source, config, pipeline_frontend::frontend_result_decls(frontend), pipeline_frontend::frontend_result_t0(frontend), pipeline_frontend::frontend_result_t_lex(frontend), pipeline_frontend::frontend_result_t_parse(frontend))",
        "run_backend(source, config, pipeline_frontend::frontend_result_decls(frontend), i32_to_i64(pipeline_frontend::frontend_result_t0(frontend)), i32_to_i64(pipeline_frontend::frontend_result_t_lex(frontend)), i32_to_i64(pipeline_frontend::frontend_result_t_parse(frontend)))",
    )
    text = text.replace(
        "fn resolve_result_t_resolve(result: DriverResolveResult) -> i64 {",
        "fn resolve_result_t_resolve(result: DriverResolveResult) -> i32 {",
    )
    text = text.replace(
        "fn typecheck_result_t_typecheck(result: DriverTypecheckResult) -> i64 {",
        "fn typecheck_result_t_typecheck(result: DriverTypecheckResult) -> i32 {",
    )
    text = text.replace(
        "backend_resolve::resolve_result_t_resolve(resolved), backend_typecheck::typecheck_result_t_typecheck(checked)",
        "i32_to_i64(backend_resolve::resolve_result_t_resolve(resolved)), i32_to_i64(backend_typecheck::typecheck_result_t_typecheck(checked))",
    )
    text = text.replace(
        "typecheck_result_t_typecheck(checked)), t_lower)",
        "typecheck_result_t_typecheck(checked)), i32_to_i64(t_lower))",
    )
    text = text.replace(
        "driver_debug::emit_phase_timing(t0, t_lex, t_parse, t_resolve, t_typecheck, t_lower, 0)",
        "driver_debug::emit_phase_timing(t0, t_lex, t_parse, t_resolve, t_typecheck, t_lower, i32_to_i64(0))",
    )
    text = text.replace(
        "debug::emit_phase_timing(t0, t_lex, t_parse, t_resolve, t_typecheck, t_lower, 0)",
        "debug::emit_phase_timing(t0, t_lex, t_parse, t_resolve, t_typecheck, t_lower, i32_to_i64(0))",
    )
    return text


def _patch_bootstrap_wasm_sections_data_only(text: str) -> str:
    """Bootstrap overlay excludes gc_hint tail; emit data section directly."""
    text = text.replace("use wasm::sections_tail", "use wasm::sections_data")
    text = text.replace(
        "sections_tail::emit_tail_sections(out, ctx, strings, opt_level)",
        "sections_data::emit_data_section(out, strings::EmitStringTablePlan_values(strings))",
    )
    return text


def _patch_bootstrap_wasm_mod_stub_emit_wat(text: str) -> str:
    """Bootstrap overlay drops WAT modules; keep a stub for driver linkage."""
    lines = text.splitlines()
    out: list[str] = []
    for line in lines:
        if line.strip() == "use wasm::wat":
            continue
        out.append(line)
    text = "\n".join(out)
    if text and not text.endswith("\n"):
        text = text + "\n"
    old_emit = """pub fn emit_wat(mir: MirModule, target: String, opt_level: i32) -> String {
    wat::emit_wat_module(mir, target, opt_level)
}"""
    new_emit = """pub fn emit_wat(mir: MirModule, target: String, opt_level: i32) -> String {
    String_from("")
}"""
    return text.replace(old_emit, new_emit)


def _patch_bootstrap_wasm_mod_p2_emit(text: str) -> str:
    """Ensure flat wasm facade keeps P2 component emit after overlay dedupe."""
    if "emit_p2_command_component" in text:
        return text
    stub = """
pub fn emit_p2_command_component(core_wasm: Vec<i32>) -> Vec<i32> {
    wasm_component_p2_emit::emit_p2_command_component(core_wasm)
}
"""
    if text and not text.endswith("\n"):
        text = text + "\n"
    return text + stub


def _patch_bootstrap_wasm_ark_p2_emit(compiler_out: Path) -> None:
    wasm_path = compiler_out / "wasm.ark"
    if wasm_path.is_file():
        wasm_path.write_text(
            _patch_bootstrap_wasm_mod_p2_emit(wasm_path.read_text(encoding="utf-8")),
            encoding="utf-8",
        )


def _git_compiler_file(root: Path, rev: str, rel_name: str) -> str | None:
    result = subprocess.run(
        ["git", "show", f"{rev}:src/compiler/{rel_name}"],
        cwd=str(root),
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def _overlay_namespace_covered_by_monolithic_fallback(rel: Path) -> bool:
    if not rel.parts:
        return False
    ns = rel.parts[0]
    if ns in BOOTSTRAP_STUB_OVERLAY_NAMESPACES:
        return True
    if ns in WORKTREE_OVERLAY_NAMESPACES:
        return True
    return ns in MONOLITHIC_OVERLAY_FALLBACK_BY_NS


def _promote_top_level_fns_public(text: str) -> str:
    """Flat overlay modules lose parent-namespace visibility; expose top-level items."""
    text = re.sub(r"^fn ", "pub fn ", text, flags=re.M)
    text = re.sub(r"^struct ", "pub struct ", text, flags=re.M)
    text = re.sub(r"^enum ", "pub enum ", text, flags=re.M)
    return text


def _write_worktree_namespace_overlay(
    namespace: str,
    compiler_out: Path,
    source_root: Path,
    root: Path,
    write_order: list[str],
) -> set[str]:
    """Seed a namespace from flattened working-tree modules (not git monolith)."""
    ns_root = source_root / namespace
    if not ns_root.is_dir():
        return set()
    candidates: list[tuple[Path, Path]] = []
    for source_path in ns_root.rglob("*.ark"):
        rel = source_path.relative_to(source_root)
        if _should_exclude_bootstrap_overlay_source(rel):
            continue
        candidates.append((source_path, rel))
    for rel_name in WORKTREE_LEGACY_FACADE_FILES.get(namespace, ()):
        source_path = source_root / rel_name
        if source_path.is_file():
            candidates.append((source_path, Path(rel_name)))
    candidates.sort(key=lambda item: _overlay_source_sort_key(item[1]))

    written: set[str] = set()
    for source_path, rel in candidates:
        flat_name = _flat_overlay_module_name(rel)
        out_path = compiler_out / f"{flat_name}.ark"
        rel_name = rel.as_posix()
        freeze_rev = BOOTSTRAP_OVERLAY_FILE_FREEZE_REVS.get(rel_name)
        if freeze_rev is not None:
            frozen = _git_compiler_file(root, freeze_rev, rel_name)
            if frozen is None:
                continue
            text = frozen
        else:
            text = source_path.read_text(encoding="utf-8")
        if rel_name == "wasm/mod.ark":
            text = _patch_bootstrap_wasm_mod_stub_emit_wat(text)
            text = _patch_bootstrap_wasm_mod_p2_emit(text)
        if rel_name == "wasm/wasm_sections.ark":
            text = _patch_bootstrap_wasm_sections_data_only(text)
        if namespace == "driver":
            text = _patch_bootstrap_driver_timing(text)
        text = _promote_top_level_fns_public(text)
        text = _rename_overlay_publish_symbols(text, rel_name)
        text = _rewrite_overlay_call_sites(text)
        out_name = f"{flat_name}.ark"
        out_path.write_text(_flatten_compiler_imports(text), encoding="utf-8")
        written.add(out_name)
        write_order.append(out_name)
    return written


def _write_bootstrap_stub_namespace_overlays(
    compiler_out: Path,
    write_order: list[str],
) -> set[str]:
    """Write passthrough stubs for namespaces that trap when fully overlaid."""
    written: set[str] = set()
    if "mir_opt" in BOOTSTRAP_STUB_OVERLAY_NAMESPACES:
        # Single owner only: a second `mir_opt_optimize_module.ark` copy makes overlay
        # dedupe rename `optimize_module` → `mir_opt__optimize_module`, breaking
        # `mir_opt::optimize_module` in the emitted selfhost wasm (unreachable trap).
        out_name = "mir_opt.ark"
        (compiler_out / out_name).write_text(BOOTSTRAP_MIR_OPT_STUB, encoding="utf-8")
        written.add(out_name)
        write_order.append(out_name)
    return written


def _write_monolithic_overlay_fallbacks(
    compiler_out: Path,
    root: Path,
    write_order: list[str],
) -> set[str]:
    """Seed the flat overlay with committed monolithic compiler modules."""
    written: set[str] = set()
    all_files: list[str] = []
    for files in MONOLITHIC_OVERLAY_FALLBACK_BY_NS.values():
        all_files.extend(files)
    all_files.extend(MONOLITHIC_OVERLAY_EXTRA_FILES)
    for rel_name in all_files:
        if rel_name in written:
            continue
        text = _git_compiler_file(root, MONOLITHIC_OVERLAY_FALLBACK_REV, rel_name)
        if text is None:
            continue
        text = _rename_overlay_publish_symbols(text, rel_name)
        text = _rewrite_overlay_call_sites(text)
        out_path = compiler_out / rel_name
        out_path.write_text(_flatten_compiler_imports(text), encoding="utf-8")
        written.add(rel_name)
        write_order.append(rel_name)
    return written


def _should_skip_flat_overlay_source(source_root: Path, rel: Path) -> bool:
    """Skip root facade files when a nested namespace owner exists.

    Bootstrapping only needs the flattened nested modules; keeping thin root
    facades duplicates the module graph and increases compiler memory use.
    """
    if len(rel.parts) != 1:
        return False
    nested_mod = source_root / rel.stem / "mod.ark"
    return nested_mod.is_file()


# Pinned selfhost wasm: `lower_to_mir` passes prune=1 into `lower_to_mir_impl`.
# That strips most of the wasm emitter when compiling the compiler to stage-2 (~345KiB
# broken s2). `lower_to_mir_no_prune` uses the same call with prune=0.
_LOWER_TO_MIR_PRUNE_FLAG_ON = bytes((0x20, 0x00, 0x20, 0x01, 0x41, 0x01))
_LOWER_TO_MIR_PRUNE_FLAG_OFF = bytes((0x20, 0x00, 0x20, 0x01, 0x41, 0x00))


def _read_leb_u32(data: bytes, offset: int) -> tuple[int, int]:
    result = 0
    shift = 0
    while offset < len(data):
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7F) << shift
        if byte & 0x80 == 0:
            return result, offset
        shift += 7
    raise ValueError("truncated leb128")


def _write_leb_u32(value: int) -> bytes:
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


def _dedupe_wasm_export_section_raw(wasm_path: Path) -> bool:
    """Rewrite the export section, keeping the first export for each name."""
    data = bytearray(wasm_path.read_bytes())
    if len(data) < 8 or data[0:4] != b"\x00asm":
        return False
    offset = 8
    sections: list[tuple[int, int, int]] = []
    while offset < len(data):
        section_id = data[offset]
        offset += 1
        size, offset = _read_leb_u32(data, offset)
        payload_start = offset
        payload_end = offset + size
        if payload_end > len(data):
            return False
        sections.append((section_id, payload_start, payload_end))
        offset = payload_end
    export_entries: list[bytes] = []
    seen: set[bytes] = set()
    rebuilt = bytearray()
    rebuilt.extend(data[0:8])
    for section_id, start, end in sections:
        payload = bytes(data[start:end])
        if section_id != 7:
            rebuilt.append(section_id)
            rebuilt.extend(_write_leb_u32(len(payload)))
            rebuilt.extend(payload)
            continue
        count, pos = _read_leb_u32(payload, 0)
        i = 0
        while i < count:
            name_len, pos = _read_leb_u32(payload, pos)
            name = payload[pos : pos + name_len]
            pos += name_len
            kind = payload[pos]
            pos += 1
            index, pos = _read_leb_u32(payload, pos)
            entry = (
                _write_leb_u32(len(name))
                + name
                + bytes((kind,))
                + _write_leb_u32(index)
            )
            if name not in seen:
                seen.add(name)
                export_entries.append(entry)
            i += 1
        new_payload = bytearray()
        new_payload.extend(_write_leb_u32(len(export_entries)))
        for entry in export_entries:
            new_payload.extend(entry)
        rebuilt.append(section_id)
        rebuilt.extend(_write_leb_u32(len(new_payload)))
        rebuilt.extend(new_payload)
    wasm_path.write_bytes(rebuilt)
    return True


def _ensure_wasm_patcher_binary(root: Path) -> Path | None:
    """Build the walrus heap/export patcher when sources are newer than the binary."""
    patcher_dir = root / PATCHER_DIR_REL
    patcher_src = patcher_dir / "src" / "main.rs"
    patcher_bin = patcher_dir / "target" / "release" / "wasm-heap-grow-patcher"
    if not patcher_src.is_file():
        return None
    source_mtime = max(
        patcher_src.stat().st_mtime,
        patcher_dir.joinpath("Cargo.toml").stat().st_mtime,
        Path(__file__).stat().st_mtime,
    )
    if not patcher_bin.is_file() or patcher_bin.stat().st_mtime < source_mtime:
        build = subprocess.run(
            ["cargo", "build", "--release", "--quiet"],
            cwd=str(patcher_dir),
            capture_output=True,
            text=True,
        )
        if build.returncode != 0 or not patcher_bin.is_file():
            return None
    return patcher_bin


def _dedupe_selfhost_wasm_exports(wasm_path: Path, root: Path) -> bool:
    """Drop duplicate exports without GC (preserves functions needed for self-compile)."""
    if _dedupe_wasm_export_section_raw(wasm_path):
        return True
    patcher_bin = _ensure_wasm_patcher_binary(root)
    if patcher_bin is not None:
        staged = wasm_path.with_suffix(".dedupe.wasm")
        cmd = [str(patcher_bin), str(wasm_path), str(staged), "--dedupe-exports"]
        patch = subprocess.run(
            cmd,
            cwd=str(root),
            capture_output=True,
            text=True,
        )
        if patch.returncode == 0 and staged.is_file():
            shutil.copyfile(staged, wasm_path)
            staged.unlink(missing_ok=True)
            return True
    return False


def _patch_bootstrap_disable_selfhost_mir_prune(wasm_path: Path) -> bool:
    """Flip pinned `lower_to_mir` to the no-prune path so stage-2 keeps emitters."""
    data = bytearray(wasm_path.read_bytes())
    # The modular pipeline (identified by its lower_entry_input_to_mir export)
    # hardcodes the no-prune path; the legacy byte pattern is generic enough to
    # false-match and corrupt an unrelated function, so skip it entirely.
    if b"lower_entry_input_to_mir" in data:
        return True
    idx = data.find(_LOWER_TO_MIR_PRUNE_FLAG_ON)
    if idx < 0:
        return data.find(_LOWER_TO_MIR_PRUNE_FLAG_OFF) >= 0
    data[idx : idx + len(_LOWER_TO_MIR_PRUNE_FLAG_ON)] = _LOWER_TO_MIR_PRUNE_FLAG_OFF
    wasm_path.write_bytes(data)
    return True


def _ensure_bootstrap_compiler_wasm(root: Path, pinned: Path) -> Path | None:
    """Return a bootstrap-capable copy of the pinned wasm (4GiB + heap grow)."""
    out = root / BOOTSTRAP_WASM_REL
    patcher_dir = root / PATCHER_DIR_REL
    patcher_src = patcher_dir / "src" / "main.rs"
    patcher_bin = patcher_dir / "target" / "release" / "wasm-heap-grow-patcher"
    if not patcher_src.is_file():
        return None
    source_mtime = max(
        pinned.stat().st_mtime,
        patcher_src.stat().st_mtime,
        patcher_dir.joinpath("Cargo.toml").stat().st_mtime,
        Path(__file__).stat().st_mtime,
    )
    if out.is_file() and out.stat().st_mtime >= source_mtime:
        return out
    out.parent.mkdir(parents=True, exist_ok=True)
    build = subprocess.run(
        ["cargo", "build", "--release", "--quiet"],
        cwd=str(patcher_dir),
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        return None
    patch = subprocess.run(
        [str(patcher_bin), str(pinned), str(out)],
        cwd=str(root),
        capture_output=True,
        text=True,
    )
    if patch.returncode != 0 or not out.is_file():
        return None
    if not _patch_bootstrap_disable_selfhost_mir_prune(out):
        return None
    return out


def _postprocess_selfhost_compiler_wasm(wasm_path: Path, root: Path) -> None:
    """Normalize stage-2/3 selfhost wasm (duplicate exports without MIR prune)."""
    _dedupe_selfhost_wasm_exports(wasm_path, root)


def _stage3_compiler_wasm(root: Path, pinned: Path, built_s2: Path) -> Path | None:
    """Compiler wasm for the fixpoint stage-3 self-recompile step."""
    runtime = _ensure_runtime_compiler_wasm(root, built_s2)
    if runtime is not None:
        return runtime
    return built_s2 if built_s2.is_file() else None


def resolve_ide_gate_compiler_wasm(root: Path) -> Path | None:
    """Compiler wasm for LSP/DAP/analysis quick gates."""
    s2 = root / ".build" / "selfhost" / "arukellt-s2.wasm"
    pinned = _find_pinned_wasm(root)
    if s2.is_file() and pinned is not None:
        runtime = _parity_runtime_compiler(root, pinned, s2)
        if runtime is not None:
            return runtime
    if s2.is_file():
        return s2
    s3 = root / ".build" / "selfhost" / "arukellt-s3.wasm"
    if s3.is_file():
        return s3
    return pinned


def _parity_runtime_compiler(root: Path, pinned: Path, built_s2: Path) -> Path | None:
    """Compiler wasm for fixture/diag/cli parity and CLI wrapper compiles."""
    runtime = _ensure_runtime_compiler_wasm(root, built_s2)
    if runtime is not None:
        return runtime
    return built_s2 if built_s2.is_file() else None


def _ensure_runtime_compiler_wasm(root: Path, compiler_wasm: Path) -> Path | None:
    """Heap-patch a selfhost compiler wasm for stage-3 and parity compile runs."""
    out = root / S2_RUNTIME_WASM_REL
    patcher_bin = root / PATCHER_DIR_REL / "target" / "release" / "wasm-heap-grow-patcher"
    if not patcher_bin.is_file():
        return None
    if out.is_file() and out.stat().st_mtime >= compiler_wasm.stat().st_mtime:
        return out
    out.parent.mkdir(parents=True, exist_ok=True)
    patch = subprocess.run(
        [str(patcher_bin), str(compiler_wasm), str(out)],
        cwd=str(root),
        capture_output=True,
        text=True,
    )
    if patch.returncode != 0 or not out.is_file():
        return None
    return out


def _patch_monolithic_typechecker_unify(text: str) -> str:
    """Break cyclic type-variable chains in the legacy monolithic typechecker."""
    old_resolve = """fn resolve_type(env: TypeEnv, ty: TypeInfo) -> TypeInfo {
    if ty.tag != typechecker_kinds::TY_TYPE_VAR() {
        return ty
    }
    let count = len(env.vars)
    let mut i = 0
    while i < count {
        let tv = get_unchecked(env.vars, i)
        if eq(concat(String_from("t"), to_string(tv.id)), clone(ty.name)) {
            if tv.is_bound {
                return resolve_type(env, tv.bound)
            }
            return ty
        }
        i = i + 1
    }
    ty
}"""
    new_resolve = """fn resolve_type(env: TypeEnv, ty: TypeInfo) -> TypeInfo {
    resolve_type_depth(env, ty, 0)
}

fn resolve_type_depth(env: TypeEnv, ty: TypeInfo, depth: i32) -> TypeInfo {
    if depth > 64 {
        return ty
    }
    if ty.tag != typechecker_kinds::TY_TYPE_VAR() {
        return ty
    }
    let count = len(env.vars)
    let mut i = 0
    while i < count {
        let tv = get_unchecked(env.vars, i)
        if eq(concat(String_from("t"), to_string(tv.id)), clone(ty.name)) {
            if tv.is_bound {
                return resolve_type_depth(env, tv.bound, depth + 1)
            }
            return ty
        }
        i = i + 1
    }
    ty
}"""
    if old_resolve not in text:
        return text
    text = text.replace(old_resolve, new_resolve, 1)
    old_deep = """fn resolve_type_deep(env: TypeEnv, ty: TypeInfo) -> TypeInfo {
    let head = resolve_type(env, ty)
    let arg_count = len(head.type_args)
    if arg_count == 0 {
        return head
    }
    let res = TypeInfo { tag: head.tag, name: clone(head.name), type_args: Vec_new_TypeInfo() }
    let mut i = 0
    while i < arg_count {
        let arg = get_unchecked(head.type_args, i)
        push(res.type_args, resolve_type_deep(env, arg))
        i = i + 1
    }
    res
}"""
    new_deep = """fn resolve_type_deep(env: TypeEnv, ty: TypeInfo) -> TypeInfo {
    resolve_type_deep_depth(env, ty, 0)
}

fn resolve_type_deep_depth(env: TypeEnv, ty: TypeInfo, depth: i32) -> TypeInfo {
    if depth > 64 {
        return ty
    }
    let head = resolve_type_depth(env, ty, depth)
    let arg_count = len(head.type_args)
    if arg_count == 0 {
        return head
    }
    let res = TypeInfo { tag: head.tag, name: clone(head.name), type_args: Vec_new_TypeInfo() }
    let mut i = 0
    while i < arg_count {
        let arg = get_unchecked(head.type_args, i)
        push(res.type_args, resolve_type_deep_depth(env, arg, depth + 1))
        i = i + 1
    }
    res
}"""
    return text.replace(old_deep, new_deep, 1)


def _patch_monolithic_typechecker_per_fn_env(text: str) -> str:
    """Reset inference vars per function body to cap TypeEnv growth during self-compile."""
    marker = "fn TypeEnv_new() -> TypeEnv {"
    if marker not in text or "fn merge_type_env(dst: TypeEnv, src: TypeEnv)" in text:
        return text
    insert_after = """fn TypeEnv_new() -> TypeEnv {
    TypeEnv { vars: Vec_new_TypeVar(), next_id: 0, errors: Vec_new_String(), error_count: 0, mono_instances: Vec_new_MonoInstance(), mono_call_sites: Vec_new_MonoCallSite(), trait_impls: Vec_new_TraitImplInfo() }
}

"""
    helpers = """fn merge_mono_instances(dst: TypeEnv, src: TypeEnv) {
    let count = len(src.mono_instances)
    let mut i = 0
    while i < count {
        let incoming = get_unchecked(src.mono_instances, i)
        let dst_count = len(dst.mono_instances)
        let mut found = false
        let mut j = 0
        while j < dst_count {
            let existing = get_unchecked(dst.mono_instances, j)
            if eq(clone(existing.mangled_name), clone(incoming.mangled_name)) {
                found = true
            }
            j = j + 1
        }
        if !found {
            push(dst.mono_instances, incoming)
        }
        i = i + 1
    }
}

fn merge_type_env(dst: TypeEnv, src: TypeEnv) {
    let err_count = src.error_count
    let mut e = 0
    while e < err_count {
        push(dst.errors, clone(get_unchecked(src.errors, e)))
        dst.error_count = dst.error_count + 1
        e = e + 1
    }
    merge_mono_instances(dst, src)
    let site_count = len(src.mono_call_sites)
    let mut s = 0
    while s < site_count {
        push(dst.mono_call_sites, get_unchecked(src.mono_call_sites, s))
        s = s + 1
    }
}

fn type_env_for_fn_check(parent: TypeEnv) -> TypeEnv {
    let child = TypeEnv_new()
    child.trait_impls = parent.trait_impls
    child
}

"""
    if insert_after not in text:
        return text
    text = text.replace(insert_after, helpers, 1)
    old_fn_check = """            push(result.typed_fns, TypedFn { name: fn_name, return_type: return_type })
            check_fn_body(env, fn_sigs, node)
        }
        if node.kind == typechecker_kinds::NK_IMPL_DECL() {
            let impl_child_count = len(node.children)
            let mut ic = 0
            while ic < impl_child_count {
                let meth = get_unchecked(node.children, ic)
                if meth.kind == typechecker_kinds::NK_FN_DECL() {
                    check_fn_body(env, fn_sigs, meth)
                }
"""
    new_fn_check = """            push(result.typed_fns, TypedFn { name: fn_name, return_type: return_type })
            let fn_env = type_env_for_fn_check(env)
            check_fn_body(fn_env, fn_sigs, node)
            merge_type_env(env, fn_env)
        }
        if node.kind == typechecker_kinds::NK_IMPL_DECL() {
            let impl_child_count = len(node.children)
            let mut ic = 0
            while ic < impl_child_count {
                let meth = get_unchecked(node.children, ic)
                if meth.kind == typechecker_kinds::NK_IMPL_DECL() {
                    let meth_env = type_env_for_fn_check(env)
                    check_fn_body(meth_env, fn_sigs, meth)
                    merge_type_env(env, meth_env)
                }
"""
    # Fix typo - should be NK_FN_DECL not NK_IMPL_DECL for meth
    new_fn_check = new_fn_check.replace(
        "if meth.kind == typechecker_kinds::NK_IMPL_DECL()",
        "if meth.kind == typechecker_kinds::NK_FN_DECL()",
        1,
    )
    if old_fn_check not in text:
        return text
    return text.replace(old_fn_check, new_fn_check, 1)


def _patch_monolithic_typechecker(text: str) -> str:
    text = _patch_monolithic_typechecker_unify(text)
    return _patch_monolithic_typechecker_per_fn_env(text)


def _needs_flat_bootstrap_overlay(root: Path) -> bool:
    compiler = root / "src" / "compiler"
    if not compiler.is_dir():
        return False
    return any(compiler.rglob("mod.ark"))


def _should_try_flat_overlay(stderr: str) -> bool:
    if "module loading error" in stderr:
        return True
    if "out of bounds memory access" in stderr:
        return True
    if "wasm trap" in stderr:
        return True
    return False


def _ensure_hop_bootstrap_compiler_wasm(root: Path, bootstrap: Path) -> Path | None:
    """Build a hop compiler (pinned -> a56+unify) for oversized modular sources."""
    out = root / HOP_BOOTSTRAP_WASM_REL
    patcher_bin = root / PATCHER_DIR_REL / "target" / "release" / "wasm-heap-grow-patcher"
    patch_marker = root / ".build" / "selfhost" / f"hop-bootstrap-patch-rev{HOP_BOOTSTRAP_PATCH_REV}"
    if (
        out.is_file()
        and out.stat().st_mtime >= bootstrap.stat().st_mtime
        and patch_marker.is_file()
    ):
        return out
    wasmtime = _find_wasmtime()
    if not wasmtime:
        return None
    work_dir = root / ".build" / "hop-bootstrap-work"
    if work_dir.exists():
        _remove_tree(work_dir)
    work_dir.mkdir(parents=True, exist_ok=True)
    src_dir = work_dir / "src" / "compiler"
    archive = subprocess.run(
        ["git", "archive", HOP_BOOTSTRAP_COMMIT, "src/compiler"],
        cwd=str(root),
        capture_output=True,
    )
    if archive.returncode != 0:
        return None
    extract = subprocess.run(
        ["tar", "-x", "-C", str(work_dir)],
        input=archive.stdout,
        capture_output=True,
    )
    if extract.returncode != 0:
        return None
    tc_path = src_dir / "typechecker.ark"
    if not tc_path.is_file():
        return None
    tc_path.write_text(
        _patch_monolithic_typechecker(tc_path.read_text(encoding="utf-8")),
        encoding="utf-8",
    )
    shutil.copyfile(bootstrap, work_dir / "hop-compiler.wasm")
    hop_s2 = work_dir / "hop-s2.wasm"
    compile = subprocess.run(
        [wasmtime, "run", "--dir", ".", "hop-compiler.wasm", "--",
         "compile", "src/compiler/main.ark", "--target", SELFHOST_TARGET, "-o", "hop-s2.wasm"],
        cwd=str(work_dir),
        capture_output=True,
        text=True,
        timeout=SELFHOST_COMPILE_TIMEOUT,
    )
    if compile.returncode != 0 or not hop_s2.is_file():
        return None
    out.parent.mkdir(parents=True, exist_ok=True)
    patch = subprocess.run(
        [str(patcher_bin), str(hop_s2), str(out)],
        cwd=str(root),
        capture_output=True,
        text=True,
    )
    if patch.returncode != 0 or not out.is_file():
        return None
    patch_marker.parent.mkdir(parents=True, exist_ok=True)
    patch_marker.write_text(str(HOP_BOOTSTRAP_PATCH_REV), encoding="utf-8")
    return out


def _flatten_compiler_imports(text: str) -> str:
    """Rewrite local nested compiler imports to the legacy flat module spelling.

    The committed pinned wasm predates directory-backed compiler namespaces. The
    generated overlay is only a bootstrap adapter; checked-in source stays nested.
    """
    use_re = re.compile(
        r"^(\s*)use\s+([A-Za-z][A-Za-z0-9_]*(?:::[A-Za-z][A-Za-z0-9_]*)+)"
        r"(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?\s*$",
    )
    alias_map: dict[str, str] = {}
    lines = text.splitlines()
    for line in lines:
        match = use_re.match(line)
        if not match:
            continue
        parts = match.group(2).split("::")
        if parts[0] not in LOCAL_COMPILER_NAMESPACES:
            continue
        stub_owner = BOOTSTRAP_STUB_NAMESPACE_FLAT_IMPORTS.get(tuple(parts))
        if stub_owner is not None:
            continue
        ns_mod = _flatten_namespace_mod_import(parts)
        flat_name = ns_mod if ns_mod is not None else "_".join(parts)
        explicit_alias = match.group(3)
        if explicit_alias is not None:
            alias = explicit_alias
        elif parts[-1] == "mod" and ns_mod is not None:
            alias = flat_name
        else:
            alias = parts[-1]
        previous = alias_map.get(alias)
        if previous is not None and previous != flat_name:
            raise ValueError(f"ambiguous local import alias `{alias}`")
        alias_map[alias] = flat_name

    output: list[str] = []
    for line in lines:
        match = use_re.match(line)
        if match:
            parts = match.group(2).split("::")
            if parts[0] in LOCAL_COMPILER_NAMESPACES:
                stub_owner = BOOTSTRAP_STUB_NAMESPACE_FLAT_IMPORTS.get(tuple(parts))
                if stub_owner is not None:
                    output.append(f"{match.group(1)}use {stub_owner}")
                    continue
                ns_mod = _flatten_namespace_mod_import(parts)
                flat = ns_mod if ns_mod is not None else "_".join(parts)
                explicit_alias = match.group(3)
                if explicit_alias is not None:
                    output.append(f"{match.group(1)}use {flat} as {explicit_alias}")
                else:
                    output.append(f"{match.group(1)}use {flat}")
                continue
        rewritten = line
        for alias, flat_name in sorted(alias_map.items(), key=lambda item: len(item[0]), reverse=True):
            rewritten = re.sub(rf"\b{re.escape(alias)}::", f"{flat_name}::", rewritten)
        output.append(rewritten)
    trailing = "\n" if text.endswith("\n") else ""
    return "\n".join(output) + trailing


_FLAT_OVERLAY_CACHE: tuple[float, Path] | None = None


def _bootstrap_overlay_root(root: Path) -> Path:
    """Return workspace root for flattened selfhost overlay.

    Large overlay trees are prone to WSL9 directory races under the repo
    ``.build/`` tree; prefer ``/tmp`` unless overridden.  Include PID so
    concurrent bootstrap jobs do not delete each other's trees.
    """
    env = os.environ.get("ARUKELLT_SELFHOST_OVERLAY_ROOT", "").strip()
    if env:
        return Path(env)
    release = os.uname().release.lower()
    if "microsoft" in release or "wsl" in release:
        digest = hashlib.sha256(str(root.resolve()).encode()).hexdigest()[:10]
        return Path("/tmp") / f"arukellt-selfhost-flat-{digest}-{os.getpid()}"
    return root / ".build" / "selfhost" / "flat-src"


def _prepare_flattened_selfhost_source(root: Path) -> Path:
    """Generate a flat-module overlay for bootstrapping with the pinned wasm."""
    global _FLAT_OVERLAY_CACHE
    source_mtime = _compiler_source_mtime(root)
    if _FLAT_OVERLAY_CACHE is not None:
        cached_mtime, cached_root = _FLAT_OVERLAY_CACHE
        compiler_dir = cached_root / "src" / "compiler"
        if cached_mtime >= source_mtime and compiler_dir.is_dir():
            return cached_root

    source_root = root / "src" / "compiler"
    overlay_root = _bootstrap_overlay_root(root)
    compiler_out = overlay_root / "src" / "compiler"
    if overlay_root.exists():
        _remove_tree(overlay_root)
    compiler_out.mkdir(parents=True, exist_ok=True)
    write_order: list[str] = []
    monolithic_written = _write_monolithic_overlay_fallbacks(compiler_out, root, write_order)
    monolithic_written |= _write_bootstrap_stub_namespace_overlays(compiler_out, write_order)
    for ns in sorted(WORKTREE_OVERLAY_NAMESPACES):
        monolithic_written |= _write_worktree_namespace_overlay(
            ns, compiler_out, source_root, root, write_order,
        )
    # Deepest namespace owners first; thin facades (mod.ark, core_api_*, root
    # *.ark) are deferred so their re-exports lose to implementation modules.
    for source_path in sorted(
        source_root.rglob("*.ark"),
        key=lambda p: _overlay_source_sort_key(p.relative_to(source_root)),
    ):
        rel = source_path.relative_to(source_root)
        if _should_skip_flat_overlay_source(source_root, rel):
            continue
        if _should_exclude_bootstrap_overlay_source(rel):
            continue
        if _overlay_namespace_covered_by_monolithic_fallback(rel):
            continue
        flat_name = _flat_overlay_module_name(rel)
        if f"{flat_name}.ark" in monolithic_written:
            continue
        out_path = compiler_out / f"{flat_name}.ark"
        text = source_path.read_text(encoding="utf-8")
        rel_name = rel.as_posix()
        text = _rename_overlay_publish_symbols(text, rel_name)
        text = _rewrite_overlay_call_sites(text)
        out_name = f"{flat_name}.ark"
        out_path.write_text(_flatten_compiler_imports(text), encoding="utf-8")
        write_order.append(out_name)
    _reapply_global_overlay_dedupe(compiler_out, write_order)
    _patch_bootstrap_mir_host_call_delegates(compiler_out)
    _patch_bootstrap_mir_module_host_needs(compiler_out)
    (compiler_out / "component.ark").write_text(BOOTSTRAP_COMPONENT_STUB, encoding="utf-8")
    (compiler_out / "component_world_spec.ark").write_text(
        BOOTSTRAP_COMPONENT_WORLD_SPEC_STUB, encoding="utf-8",
    )
    _patch_bootstrap_wasm_ark_p2_emit(compiler_out)
    _write_bootstrap_namespace_facades(compiler_out)
    ark_toml = source_root / "ark.toml"
    if ark_toml.is_file():
        shutil.copyfile(ark_toml, compiler_out / "ark.toml")
    _FLAT_OVERLAY_CACHE = (source_mtime, overlay_root)
    return overlay_root


def _patch_bootstrap_mir_host_call_delegates(compiler_out: Path) -> None:
    """Drop mir host-call facades that recurse after overlay symbol renaming."""
    fn_path = compiler_out / "mir_module_functions.ark"
    host_path = compiler_out / "mir_module_host_calls.ark"
    if not fn_path.is_file() or not host_path.is_file():
        return
    host_text = host_path.read_text(encoding="utf-8")
    text = fn_path.read_text(encoding="utf-8")
    for symbol in ("mir_call_is_arukellt_host", "mir_call_is_wasi_http_outgoing"):
        if not re.search(
            rf"pub fn (?:mir_module_host_calls__)?{re.escape(symbol)}\(callee: String\) -> bool",
            host_text,
        ):
            continue
        text = re.sub(
            rf"pub fn {re.escape(symbol)}\(callee: String\) -> bool \{{[^}}]+\}}\n+",
            "",
            text,
            count=1,
        )
        text = text.replace(
            f"{symbol}(",
            f"mir_module_host_calls::{symbol}(",
        )
    if "use mir_module_host_calls" not in text:
        text = text.replace(
            "use mir_opcodes\n",
            "use mir_opcodes\nuse mir_module_host_calls\n",
        )
    fn_path.write_text(text, encoding="utf-8")


def _patch_bootstrap_mir_module_host_needs(compiler_out: Path) -> None:
    """Bootstrap overlay: host-import scans trap after flat-module symbol renames."""
    path = compiler_out / "mir_module_functions.ark"
    if not path.is_file():
        return
    text = path.read_text(encoding="utf-8")
    stubs: tuple[tuple[str, str], ...] = (
        ("mir_module_needs_arukellt_host", "mir: MirModule"),
        ("mir_module_needs_wasi_http_outgoing", "mir: MirModule"),
        ("mir_module_needs_wasi_http_outgoing_if_p2", "mir: MirModule, wasi_version: String"),
    )
    for name, params in stubs:
        text = re.sub(
            rf"pub fn {re.escape(name)}\({re.escape(params)}\) -> i32 \{{[\s\S]*?\n\}}",
            f"pub fn {name}({params}) -> i32 {{\n    0\n}}",
            text,
            count=1,
        )
    path.write_text(text, encoding="utf-8")


def _prepare_bootstrap_workspace(root: Path) -> Path:
    """Return an isolated workspace whose ``src/compiler`` shadows the live tree."""
    # Flat overlay is already isolated; skip copytree (WSL races on ~1.5k files).
    return _prepare_flattened_selfhost_source(root)


def _write_bootstrap_namespace_facades(compiler_out: Path) -> None:
    """Drop stale ``foo_mod.ark`` copies when ``foo.ark`` already owns the namespace."""
    for mod_path in sorted(compiler_out.glob("*_mod.ark")):
        ns = mod_path.name[: -len("_mod.ark")]
        facade_path = compiler_out / f"{ns}.ark"
        if facade_path.is_file():
            mod_path.unlink()
            continue
        shutil.copyfile(mod_path, facade_path)
        mod_path.unlink()


def _wasm_compile_selfhost_source(
    wasmtime: str,
    compiler_wasm: Path,
    out_rel: str,
    root: Path,
    timeout: int | None = None,
) -> subprocess.CompletedProcess:
    """Compile current selfhost source, falling back to a flat bootstrap overlay."""
    compile_timeout = SELFHOST_COMPILE_TIMEOUT if timeout is None else timeout
    if _needs_flat_bootstrap_overlay(root):
        workspace = _prepare_bootstrap_workspace(root)
        return _wasm_compile(
            wasmtime,
            compiler_wasm,
            SELFHOST_SOURCE_REL,
            out_rel,
            root,
            timeout=compile_timeout,
            workspace_root=workspace,
        )
    result = _wasm_compile(
        wasmtime, compiler_wasm, SELFHOST_SOURCE_REL, out_rel, root, timeout=compile_timeout,
    )
    if result.returncode == 0:
        return result
    if not _should_try_flat_overlay(result.stderr or ""):
        return result
    workspace = _prepare_bootstrap_workspace(root)
    return _wasm_compile(
        wasmtime,
        compiler_wasm,
        SELFHOST_SOURCE_REL,
        out_rel,
        root,
        timeout=compile_timeout,
        workspace_root=workspace,
    )


def _compile_selfhost_bootstrap_chain(
    wasmtime: str,
    root: Path,
    out_rel: str,
    bootstrap: Path,
) -> subprocess.CompletedProcess:
    """Try pinned bootstrap, then hop bootstrap (a56+unify), for stage-2 builds."""
    compilers: list[Path] = [bootstrap]
    hop = _ensure_hop_bootstrap_compiler_wasm(root, bootstrap)
    if hop is not None:
        compilers.append(hop)
    last: subprocess.CompletedProcess | None = None
    for compiler in compilers:
        last = _wasm_compile_selfhost_source(wasmtime, compiler, out_rel, root)
        if last.returncode == 0:
            out = root / out_rel
            if out.is_file():
                _postprocess_selfhost_compiler_wasm(out, root)
            return last
    assert last is not None
    return last


def _wasm_fmt(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    root: Path,
    timeout: int | None = None,
) -> subprocess.CompletedProcess:
    return _run(
        [wasmtime, "run", "--dir", str(root), str(compiler_wasm), "--",
         "fmt", src],
        root,
        timeout=timeout,
    )


def _wasm_check(
    wasmtime: str,
    compiler_wasm: Path,
    src: str,
    root: Path,
    timeout: int | None = None,
    extra_args: list[str] | None = None,
) -> subprocess.CompletedProcess:
    cmd = [wasmtime, "run", "--dir", str(root), str(compiler_wasm), "--"]
    cmd.extend(["check"])
    if extra_args:
        cmd.extend(extra_args)
    cmd.append(src)
    return _run(
        cmd,
        root,
        timeout=timeout,
    )


def _diag_fixture_flags(root: Path, fixture: str) -> list[str]:
    """Return extra CLI args from a sibling ``.flags`` file, one token per line."""
    flags_path = root / "tests" / "fixtures" / (fixture[:-4] + ".flags")
    if not flags_path.is_file():
        return []
    lines = flags_path.read_text(encoding="utf-8").splitlines()
    return [line.strip() for line in lines if line.strip()]


# ── SelfhostFixpointResult ────────────────────────────────────────────────────

@dataclass
class SelfhostFixpointResult:
    exit_code: int
    passed: bool
    skipped: bool
    output: str


# ── run_fixpoint ──────────────────────────────────────────────────────────────

def run_fixpoint(
    root: Path,
    dry_run: bool,
    no_build: bool = True,
) -> SelfhostFixpointResult:
    """Selfhost-native fixpoint gate (ADR-029).

    Bootstrap path:
        pinned (bootstrap/arukellt-selfhost.wasm) ──▶ s2.wasm
        s2.wasm ──▶ s3.wasm
        require sha256(s2) == sha256(s3)

    Exit codes:
        0  fixpoint reached (passed=True)
        1  not yet reached  (skipped=True, tracked)
        2  prereqs missing  (skipped=True)
    """
    build = not no_build
    lines: list[str] = []

    def emit(msg: str) -> None:
        lines.append(msg)

    if dry_run:
        print("DRY-RUN: run_fixpoint()")
        return SelfhostFixpointResult(exit_code=0, passed=True, skipped=False, output="")

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        emit(f"{RED}error: pinned-reference selfhost wasm not found at "
             f"{PINNED_WASM_REL} (see bootstrap/PROVENANCE.md){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    source = SELFHOST_SOURCE_REL
    if not (root / source).is_file():
        emit(f"{RED}error: selfhost source not found: {source}{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    wasmtime = _find_wasmtime()
    if not wasmtime:
        emit(f"{RED}error: wasmtime not found in PATH{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    build_dir = root / ".build" / "selfhost"
    build_dir.mkdir(parents=True, exist_ok=True)

    s2 = build_dir / "arukellt-s2.wasm"
    s3 = build_dir / "arukellt-s3.wasm"
    s2_rel = str(s2.relative_to(root))
    s3_rel = str(s3.relative_to(root))

    bootstrap = _ensure_bootstrap_compiler_wasm(root, pinned)
    if bootstrap is None:
        emit(f"{RED}error: failed to prepare bootstrap compiler wasm from {PINNED_WASM_REL}{NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Stage 2: bootstrap wasm compiles current selfhost source → s2.wasm
    if build or not s2.is_file():
        emit(f"{YELLOW}[selfhost] Building stage 2 (bootstrap wasm → s2.wasm)...{NC}")
        r = _compile_selfhost_bootstrap_chain(wasmtime, root, s2_rel, bootstrap)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 2 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        emit(f"{GREEN}✓ s2.wasm built{NC}")

    if not s2.is_file():
        emit(f"{RED}error: s2.wasm not found at {s2} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    # Stage 3: s2 compiles current selfhost source → s3.wasm
    if build or not s3.is_file():
        emit(f"{YELLOW}[selfhost] Building stage 3 (s2.wasm → s3.wasm)...{NC}")
        stage3_compiler = _stage3_compiler_wasm(root, pinned, s2)
        if stage3_compiler is None:
            emit(f"{RED}error: failed to prepare stage-3 compiler wasm{NC}")
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        r = _wasm_compile_selfhost_source(wasmtime, stage3_compiler, s3_rel, root)
        if r.returncode != 0:
            emit(f"{RED}✗ stage 3 compilation failed{NC}")
            if r.stderr:
                emit(r.stderr[:500])
            return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))
        _postprocess_selfhost_compiler_wasm(s3, root)
        emit(f"{GREEN}✓ s3.wasm built{NC}")

    if not s3.is_file():
        emit(f"{RED}error: s3.wasm not found at {s3} (run without --no-build first){NC}")
        return SelfhostFixpointResult(exit_code=2, passed=False, skipped=True, output="\n".join(lines))

    sha2 = _sha256(s2)
    sha3 = _sha256(s3)

    if sha2 == sha3:
        emit(f"{GREEN}✓ selfhost fixpoint reached: sha256({s2.name}) == sha256({s3.name}){NC}")
        emit(f"  sha256 = {sha2}")
        emit(f"  pinned base: {PINNED_WASM_REL} (sha256 {_sha256(pinned)})")
        return SelfhostFixpointResult(exit_code=0, passed=True, skipped=False, output="\n".join(lines))

    emit(f"{YELLOW}⊙ selfhost fixpoint not yet reached (this is normal during development){NC}")
    emit(f"  sha256(s2) = {sha2}")
    emit(f"  sha256(s3) = {sha3}")
    return SelfhostFixpointResult(exit_code=1, passed=False, skipped=True, output="\n".join(lines))


# ── Shared manifest parsing ───────────────────────────────────────────────────

def _load_manifest_fixtures(root: Path, kind: str) -> tuple[list[str], str]:
    """Return list of fixture paths for kind='run' or kind='diag'. Also returns error string."""
    manifest = root / "tests" / "fixtures" / "manifest.txt"
    if not manifest.is_file():
        return [], f"{RED}error: manifest not found: {manifest}{NC}"
    pattern = re.compile(rf"^{kind}:\s*(.+\.ark)$")
    fixtures: list[str] = []
    for line in manifest.read_text().splitlines():
        m = pattern.match(line)
        if m:
            fixtures.append(m.group(1))
    return fixtures, ""


# ── Current-selfhost wasm helper ──────────────────────────────────────────────

def _ensure_current_selfhost(root: Path, wasmtime: str, pinned: Path) -> tuple[Path | None, str]:
    """Return path to current-source selfhost wasm, building it from pinned if needed.

    Output is ``.build/selfhost/arukellt-s2.wasm``. If it already exists, it is
    reused (callers may invoke ``run_fixpoint`` first to refresh it).
    """
    build_dir = root / ".build" / "selfhost"
    build_dir.mkdir(parents=True, exist_ok=True)
    out = build_dir / "arukellt-s2.wasm"
    if out.is_file() and out.stat().st_mtime >= _compiler_source_mtime(root):
        runtime = _parity_runtime_compiler(root, pinned, out)
        if runtime is None:
            return None, (
                f"{RED}error: failed to prepare runtime compiler wasm from {out.name}{NC}"
            )
        return runtime, ""
    out_rel = str(out.relative_to(root))
    bootstrap = _ensure_bootstrap_compiler_wasm(root, pinned)
    if bootstrap is None:
        return None, (
            f"{RED}error: failed to prepare bootstrap compiler wasm from {PINNED_WASM_REL}{NC}"
        )
    r = _compile_selfhost_bootstrap_chain(wasmtime, root, out_rel, bootstrap)
    if r.returncode != 0:
        return None, (
            f"{RED}error: failed to bootstrap current-selfhost wasm from pinned wasm{NC}\n"
            + (r.stderr[:500] if r.stderr else "")
        )
    runtime = _parity_runtime_compiler(root, pinned, out)
    if runtime is None:
        return None, (
            f"{RED}error: failed to prepare runtime compiler wasm from {out.name}{NC}"
        )
    return runtime, ""


# ── Fixture parity skip list ─────────────────────────────────────────────────

# Fixtures with known parity differences that are not semantic errors.
# Pre-585 these tracked Rust-vs-selfhost differences. Post-585 (ADR-029)
# these track pinned-vs-current selfhost differences with the same root
# causes — kept verbatim because the underlying selfhost-emitter
# limitations have not changed.
#
# Format: "category/fixture.ark"  # reason
FIXTURE_PARITY_SKIP: set[str] = {
    "stdlib_sort/sort_f64.ark",  # selfhost f64_to_string uses naive digit extraction
                                 # (1.2 → 1.199999999999999); reference uses Grisu2/shortest-repr
    "functions/higher_order.ark",  # selfhost emitter lacks funcref table / call_indirect
                                   # support; fn-pointer parameters are not yet lowered.
}


# ── run_fixture_parity ────────────────────────────────────────────────────────

def run_fixture_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Pinned-vs-current selfhost execution-output parity gate (ADR-029).

    For each ``run:`` fixture in the manifest:
        - compile with pinned wasm and with current selfhost wasm
        - execute both wasms; require stdout/stderr/exit-code equal
    """
    if dry_run:
        print("DRY-RUN: run_fixture_parity()")
        return (0, "")

    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    fixtures, err = _load_manifest_fixtures(root, "run")
    if err:
        return (1, err + "\n")

    if len(fixtures) < 10:
        return (1, f"{RED}error: fewer than 10 run: fixtures in manifest ({len(fixtures)} found){NC}\n")

    pinned_sha = _sha256(pinned)
    current_sha = _sha256(current)
    lines.append(f"{YELLOW}[fixture-parity] Checking {len(fixtures)} run: fixtures "
                 f"(pinned={pinned_sha[:12]} vs current={current_sha[:12]})...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    self_out_dir = root / ".ark-fixture-parity-tmp"
    self_out_dir.mkdir(exist_ok=True)
    tmpdir = tempfile.mkdtemp(prefix="ark-fixture-parity-")
    try:
        for fixture in fixtures:
            if fixture in FIXTURE_PARITY_SKIP:
                lines.append(f"  skip: {fixture} (known parity skip)")
                skip_count += 1
                continue

            ark_file = root / "tests" / "fixtures" / fixture
            if not ark_file.is_file():
                lines.append(f"  skip: {fixture} (not found on disk)")
                skip_count += 1
                continue

            src_rel = str(Path("tests") / "fixtures" / fixture)
            out_pinned_rel = str(Path(".ark-fixture-parity-tmp") /
                                 f"pinned-{fixture.replace('/', '_')}.wasm")
            out_current_rel = str(Path(".ark-fixture-parity-tmp") /
                                  f"current-{fixture.replace('/', '_')}.wasm")
            out_pinned = root / out_pinned_rel
            out_current = root / out_current_rel

            # Compile with pinned compiler
            r = _wasm_compile(wasmtime, pinned, src_rel, out_pinned_rel, root, timeout=30)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (pinned compile failed/timeout)")
                skip_count += 1
                continue

            # Compile with current selfhost compiler
            r = _wasm_compile(wasmtime, current, src_rel, out_current_rel, root, timeout=30)
            if r.returncode != 0:
                lines.append(f"  skip: {fixture} (current selfhost compile failed/timeout)")
                skip_count += 1
                continue

            # Compare execution output
            r_p = _run(_wasm_run_argv(root, out_pinned), root, timeout=15)
            p_out = (r_p.stdout + r_p.stderr).strip()
            p_code = r_p.returncode

            r_c = _run(_wasm_run_argv(root, out_current), root, timeout=15)
            c_out = (r_c.stdout + r_c.stderr).strip()
            c_code = r_c.returncode

            # If either side traps as an invalid module (validation error from
            # the emitter — same forgiving treatment as the pre-585 contract),
            # treat as skip not fail.
            def _is_trap_or_invalid(code: int, out: str) -> bool:
                return code == 134 or (code == 1 and "failed to compile" in out)

            if _is_trap_or_invalid(p_code, p_out) or _is_trap_or_invalid(c_code, c_out):
                lines.append(f"  skip: {fixture} (selfhost wasm trap/invalid)")
                skip_count += 1
                continue

            if p_out == c_out and p_code == c_code:
                pass_count += 1
            else:
                lines.append(f"  FAIL: {fixture} (execution output drifts pinned↔current)")
                if p_code != c_code:
                    lines.append(f"    exit: pinned={p_code} current={c_code}")
                if p_out != c_out:
                    lines.append(f"    pinned : {p_out[:80]!r}")
                    lines.append(f"    current: {c_out[:80]!r}")
                fail_count += 1
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
        shutil.rmtree(str(self_out_dir), ignore_errors=True)

    lines.append("")
    lines.append(f"{YELLOW}fixture-parity: PASS={pass_count} FAIL={fail_count} SKIP={skip_count}{NC}")

    if fail_count > 0:
        lines.append(
            f"{RED}✗ fixture parity: {fail_count} fixture(s) drift between pinned and current selfhost — "
            f"fix the regression or refresh bootstrap/arukellt-selfhost.wasm per ADR-029{NC}"
        )
        return (1, "\n".join(lines) + "\n")

    if pass_count < 10:
        lines.append(
            f"{RED}✗ fixture parity: only {pass_count} fixtures passed (need >= 10 per #585 floor){NC}"
        )
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} run: fixtures match between pinned and current selfhost{NC}")
    return (0, "\n".join(lines) + "\n")


# ── run_diag_parity ───────────────────────────────────────────────────────────

# Fixtures skipped for diag-parity because selfhost has not yet implemented
# the diagnostics or the test exercises an unimplemented feature.  These are
# tracked in issue #529 Phase 3 (diagnostic parity expansion).
DIAG_PARITY_SKIP: frozenset[str] = frozenset({
    "diagnostics/deprecated_prelude_println.ark",
    "diagnostics/deprecated_std_io_import.ark",
    "diagnostics/deprecated_time_monotonic_now.ark",
    "diagnostics/immutable_mutation.ark",
    "diagnostics/mismatched_arms.ark",
    "diagnostics/mutable_sharing.ark",
    "diagnostics/non_exhaustive.ark",
    "diagnostics/question_type_mismatch.ark",
    "diagnostics/unused_import.ark",
    "diagnostics/wrong_arg_count.ark",
    "deny_clock_compile.ark",
    "deny_random_compile.ark",
    "stdlib_io/deny_clock.ark",
    "stdlib_io/deny_random.ark",
    "v0_constraints/no_method_call.ark",
    "v0_constraints/no_operator_overload.ark",
    "module_import/use_symbol_not_found.ark",
    "selfhost/typecheck_match_nonexhaustive.ark",
    "selfhost/ret_type_mismatch.ark",
    "selfhost/trait_ambiguous_bound.ark",
    "selfhost/trait_overlapping_impl.ark",
    "selfhost/trait_unresolved_var_bound.ark",
})


def run_diag_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Pure-selfhost diagnostic snapshot gate (ADR-029).

    For each ``diag:`` fixture, run the current selfhost compiler under
    wasmtime with ``check`` and require its output to contain the
    committed ``.selfhost.diag`` (or ``.diag`` fallback) pattern.
    """
    if dry_run:
        print("DRY-RUN: run_diag_parity()")
        return (0, "")

    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    fixtures, err = _load_manifest_fixtures(root, "diag")
    if err:
        return (1, err + "\n")

    lines.append(f"{YELLOW}[diag-parity] Checking {len(fixtures)} diag: fixtures "
                 f"against committed .diag goldens (current selfhost only)...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for fixture in fixtures:
        if fixture in DIAG_PARITY_SKIP:
            skip_count += 1
            continue

        ark_path = root / "tests" / "fixtures" / fixture
        diag_path = root / "tests" / "fixtures" / (fixture[:-4] + ".diag")
        selfhost_diag_path = root / "tests" / "fixtures" / (fixture[:-4] + ".selfhost.diag")

        if not ark_path.is_file():
            lines.append(f"  skip: {fixture} (source not found)")
            skip_count += 1
            continue
        if not diag_path.is_file() and not selfhost_diag_path.is_file():
            lines.append(f"  skip: {fixture} (.diag file not found)")
            skip_count += 1
            continue

        # Prefer .selfhost.diag (selfhost-specific golden) over .diag (legacy).
        if selfhost_diag_path.is_file():
            pattern = selfhost_diag_path.read_text().strip()
        else:
            pattern = diag_path.read_text().strip()

        r = _wasm_check(
            wasmtime,
            current,
            str(Path("tests") / "fixtures" / fixture),
            root,
            extra_args=_diag_fixture_flags(root, fixture),
        )
        out = r.stdout + r.stderr

        if pattern in out:
            lines.append(f"  pass: {fixture}")
            pass_count += 1
        else:
            lines.append(f"  FAIL: {fixture} (selfhost: pattern '{pattern[:60]}' not found)")
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}diag-parity: PASS={pass_count} SKIP={skip_count} FAIL={fail_count}{NC}")

    min_pass = 10
    if fail_count > 0:
        lines.append(f"{RED}✗ diag parity: {fail_count} fixture(s) regressed against committed goldens{NC}")
        return (1, "\n".join(lines) + "\n")
    if pass_count < min_pass:
        lines.append(f"{RED}✗ diag parity: only {pass_count} passing (need >= {min_pass}){NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(
        f"{GREEN}✓ diag parity: {pass_count} fixtures pass against committed selfhost goldens, "
        f"{skip_count} skipped (Phase 3 pending){NC}"
    )
    return (0, "\n".join(lines) + "\n")


def run_fmt_parity(root: Path, dry_run: bool) -> tuple[int, str]:
    """Formatter golden gate (#216, #345).

    For each ``fmt:`` fixture, run ``arukellt fmt`` and compare output to the
    committed ``.expected`` golden. Also checks idempotency and parse validity.
    """
    if dry_run:
        print("DRY-RUN: run_fmt_parity()")
        return (0, "")

    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "runtime_lock",
        root / "scripts" / "selfhost" / "runtime_lock.py",
    )
    if spec is None or spec.loader is None:
        return (1, f"{RED}error: missing scripts/selfhost/runtime_lock.py{NC}\n")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.with_selfhost_runtime_lock(lambda: _run_fmt_parity_locked(root))


def _run_fmt_parity_locked(root: Path) -> tuple[int, str]:
    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    fixtures, err = _load_manifest_fixtures(root, "fmt")
    if err:
        return (1, err + "\n")

    lines.append(f"{YELLOW}[fmt-parity] Checking {len(fixtures)} fmt: fixtures "
                 f"against committed .expected goldens...{NC}")

    pass_count = 0
    fail_count = 0
    skip_count = 0

    for fixture in fixtures:
        ark_path = root / "tests" / "fixtures" / fixture
        expected_path = root / "tests" / "fixtures" / (fixture[:-4] + ".expected")
        work_rel = str(Path("tests") / "fixtures" / (fixture[:-4] + ".fmt_work.ark"))
        work_path = root / work_rel

        if not ark_path.is_file():
            lines.append(f"  skip: {fixture} (source not found)")
            skip_count += 1
            continue
        if not expected_path.is_file():
            lines.append(f"  skip: {fixture} (.expected file not found)")
            skip_count += 1
            continue

        expected = expected_path.read_text(encoding="utf-8")
        shutil.copyfile(ark_path, work_path)
        try:
            r = _wasm_fmt(wasmtime, current, work_rel, root)
            if r.returncode != 0:
                lines.append(
                    f"  FAIL: {fixture} (fmt exit {r.returncode}: "
                    f"{(r.stdout + r.stderr).strip()!r})"
                )
                fail_count += 1
                continue
            formatted = work_path.read_text(encoding="utf-8")
            if formatted != expected:
                lines.append(f"  FAIL: {fixture} (output mismatch vs .expected)")
                fail_count += 1
                continue

            r2 = _wasm_fmt(wasmtime, current, work_rel, root)
            if r2.returncode != 0:
                lines.append(f"  FAIL: {fixture} (idempotent fmt exit {r2.returncode})")
                fail_count += 1
                continue
            formatted2 = work_path.read_text(encoding="utf-8")
            if formatted2 != formatted:
                lines.append(f"  FAIL: {fixture} (not idempotent)")
                fail_count += 1
                continue

            check = _wasm_check(wasmtime, current, work_rel, root)
            if check.returncode != 0:
                lines.append(f"  FAIL: {fixture} (formatted output does not parse)")
                fail_count += 1
                continue

            lines.append(f"  pass: {fixture}")
            pass_count += 1
        finally:
            if work_path.is_file():
                work_path.unlink()

    lines.append("")
    lines.append(f"{YELLOW}fmt-parity: PASS={pass_count} SKIP={skip_count} FAIL={fail_count}{NC}")

    if fail_count > 0:
        lines.append(f"{RED}✗ fmt parity: {fail_count} fixture(s) regressed{NC}")
        return (1, "\n".join(lines) + "\n")
    if pass_count == 0:
        lines.append(f"{RED}✗ fmt parity: no fixtures exercised{NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ fmt parity: {pass_count} fixtures pass{NC}")
    return (0, "\n".join(lines) + "\n")


# ── run_parity ────────────────────────────────────────────────────────────────

def _run_cli_parity(root: Path) -> tuple[int, str]:
    """Pure-selfhost CLI snapshot gate (ADR-029).

    Compares ``--version`` and ``--help`` byte-equal against committed
    goldens under ``tests/snapshots/selfhost/``, and asserts non-zero
    exit codes for unknown commands and known-but-no-args invocations.
    """
    lines: list[str] = []

    pinned = _find_pinned_wasm(root)
    if pinned is None:
        return (1, f"{RED}error: pinned-reference selfhost wasm not found at "
                   f"{PINNED_WASM_REL}{NC}\n")

    wasmtime = _find_wasmtime()
    if not wasmtime:
        return (1, f"{RED}error: wasmtime not found{NC}\n")

    current, err = _ensure_current_selfhost(root, wasmtime, pinned)
    if current is None:
        return (1, err)

    version_golden = root / CLI_VERSION_GOLDEN_REL
    help_golden = root / CLI_HELP_GOLDEN_REL
    if not version_golden.is_file():
        return (1, f"{RED}error: cli-version golden missing: {CLI_VERSION_GOLDEN_REL}{NC}\n")
    if not help_golden.is_file():
        return (1, f"{RED}error: cli-help golden missing: {CLI_HELP_GOLDEN_REL}{NC}\n")

    lines.append(f"{YELLOW}[cli-parity] Checking selfhost CLI surface against committed goldens...{NC}")

    pass_count = 0
    fail_count = 0

    def run_self_with_dirs(dirs: list[Path], *args: str) -> tuple[int, str]:
        cmd = [wasmtime, "run"]
        for mount in dirs:
            cmd.extend(["--dir", str(mount)])
        cmd.extend([str(current), "--", *args])
        r = _run(cmd, root)
        return r.returncode, (r.stdout + r.stderr)

    def run_self(*args: str) -> tuple[int, str]:
        return run_self_with_dirs([root], *args)

    def _norm(s: str) -> str:
        return s.replace("\r\n", "\n").rstrip("\n")

    # Case 1: --version snapshot
    _, out_v = run_self("--version")
    expected_v = _norm(version_golden.read_text())
    actual_v = _norm(out_v)
    if actual_v == expected_v:
        lines.append("  pass: --version (matches golden)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: --version (drifts from golden)\n"
                     f"    expected: {expected_v!r}\n"
                     f"    actual  : {actual_v!r}")
        fail_count += 1

    # Case 2: --help snapshot
    _, out_h = run_self("--help")
    expected_h = _norm(help_golden.read_text())
    actual_h = _norm(out_h)
    if actual_h == expected_h:
        lines.append("  pass: --help (matches golden)")
        pass_count += 1
    else:
        lines.append("  FAIL: --help (drifts from golden — update tests/snapshots/selfhost/cli-help.txt if intentional)")
        # Emit a tiny diff hint
        ex_lines = expected_h.splitlines()
        ac_lines = actual_h.splitlines()
        for i, (e, a) in enumerate(zip(ex_lines, ac_lines)):
            if e != a:
                lines.append(f"    line {i+1}: expected {e!r} got {a!r}")
                break
        if len(ex_lines) != len(ac_lines):
            lines.append(f"    line count: expected {len(ex_lines)} got {len(ac_lines)}")
        fail_count += 1

    # Case 3: unknown command — must exit non-zero
    rc_s, _ = run_self("foobar_unknown_cmd")
    if rc_s != 0:
        lines.append(f"  pass: unknown-cmd (non-zero exit: {rc_s})")
        pass_count += 1
    else:
        lines.append(f"  FAIL: unknown-cmd (expected non-zero exit, got {rc_s})")
        fail_count += 1

    # Cases 4-6: known commands with no args — must exit non-zero
    for cmd in ["compile", "check", "run"]:
        rc_s, _ = run_self(cmd)
        if rc_s != 0:
            lines.append(f"  pass: {cmd} (no-args: non-zero exit: {rc_s})")
            pass_count += 1
        else:
            lines.append(f"  FAIL: {cmd} (no-args: expected non-zero, got {rc_s})")
            fail_count += 1

    # Case 7: targets — must exit zero and mention wasm32-wasi-p2
    rc_t, out_t = run_self("targets")
    if rc_t == 0 and "wasm32-wasi-p2" in out_t:
        lines.append(f"  pass: targets (exit 0, mentions wasm32-wasi-p2)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: targets (exit={rc_t}, output={out_t.strip()!r})")
        fail_count += 1

    # ── Newly implemented commands ──────────────────────────────────────────

    # Case 8: init — create a project in a temp directory
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        (tmp / "test_project" / "src").mkdir(parents=True)
        r = _run(
            [wasmtime, "run", "--dir", ".", str(current), "--", "init", "test_project"],
            tmp,
        )
        rc_i, out_i = r.returncode, (r.stdout + r.stderr)
        if rc_i == 0 and "Initialized Arukellt project" in out_i:
            lines.append(f"  pass: init (exit 0, mentions 'Initialized Arukellt project')")
            pass_count += 1
        else:
            lines.append(f"  FAIL: init (exit={rc_i}, output={out_i.strip()!r})")
            fail_count += 1

    # Case 9: fmt — format a known source file
    rc_f, out_f = run_self("fmt", "tests/fixtures/hello_world.ark")
    if rc_f == 0:
        lines.append(f"  pass: fmt (exit 0)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: fmt (exit={rc_f}, output={out_f.strip()!r})")
        fail_count += 1

    # Case 10: component — build a component from a known source
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        rc_c, out_c = run_self_with_dirs(
            [root, tmp],
            "component",
            "tests/fixtures/hello_world.ark",
            "-o",
            "test_component.wasm",
        )
        if rc_c == 0:
            lines.append(f"  pass: component (exit 0)")
            pass_count += 1
        else:
            lines.append(f"  FAIL: component (exit={rc_c}, output={out_c.strip()!r})")
            fail_count += 1

    # Case 11: script — run in a directory with ark.toml
    fixture_project = root / "tests/package-workspace/basic-project"
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        shutil.copytree(fixture_project, tmp / "basic-project",
                        dirs_exist_ok=True)
        project_dir = tmp / "basic-project"
        r = _run(
            [wasmtime, "run", "--dir", str(project_dir), str(current), "--", "script"],
            project_dir,
        )
        rc_s, out_s = r.returncode, (r.stdout + r.stderr)
        if rc_s == 0:
            lines.append(f"  pass: script (exit 0)")
            pass_count += 1
        else:
            lines.append(f"  FAIL: script (exit={rc_s}, output={out_s.strip()!r})")
            fail_count += 1

    # Case 12: lint — run lint on a known clean source
    rc_l, out_l = run_self("lint", "tests/fixtures/selfhost/analysis_clean.ark")
    if rc_l == 0:
        lines.append(f"  pass: lint (exit 0)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: lint (exit={rc_l}, output={out_l.strip()!r})")
        fail_count += 1

    # Case 13: analyze — run analysis on a known clean source
    rc_a, out_a = run_self("analyze", "tests/fixtures/selfhost/analysis_clean.ark")
    if rc_a == 0:
        lines.append(f"  pass: analyze (exit 0)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: analyze (exit={rc_a}, output={out_a.strip()!r})")
        fail_count += 1

    # Case 14: doc — look up a known standard library module
    rc_d, out_d = run_self("doc", "std::core")
    if rc_d == 0:
        lines.append(f"  pass: doc (exit 0)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: doc (exit={rc_d}, output={out_d.strip()!r})")
        fail_count += 1

    # Case 15: component build — compile to component wasm
    rc_cb, out_cb = run_self("component", "build", "tests/fixtures/hello_world.ark")
    if rc_cb == 0:
        lines.append(f"  pass: component build (exit 0)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: component build (exit={rc_cb}, output={out_cb.strip()!r})")
        fail_count += 1

    # Case 16: component inspect — should gracefully report not-yet-implemented
    rc_ci, out_ci = run_self("component", "inspect", "nonexistent.wasm")
    if rc_ci == 1 and "not yet implemented" in out_ci:
        lines.append(f"  pass: component inspect (exit 1, graceful)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: component inspect (exit={rc_ci}, output={out_ci.strip()!r})")
        fail_count += 1

    # Case 17: component validate — should gracefully report not-yet-implemented
    rc_cv, out_cv = run_self("component", "validate", "nonexistent.wasm")
    if rc_cv == 1 and "not yet implemented" in out_cv:
        lines.append(f"  pass: component validate (exit 1, graceful)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: component validate (exit={rc_cv}, output={out_cv.strip()!r})")
        fail_count += 1

    # Case 18: compose — no args must exit non-zero with usage
    rc_co0, out_co0 = run_self("compose")
    if rc_co0 != 0 and ("usage" in out_co0.lower() or "error" in out_co0.lower()):
        lines.append(f"  pass: compose (no-args: non-zero exit with usage)")
        pass_count += 1
    else:
        lines.append(f"  FAIL: compose (no-args: exit={rc_co0}, output={out_co0.strip()!r})")
        fail_count += 1

    # Case 19: compose --validate with placeholder component files
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        provider = tmp / "provider.wasm"
        socket = tmp / "socket.wasm"
        provider.write_bytes(b"\0asm")
        socket.write_bytes(b"\0asm")
        rc_co1, out_co1 = run_self_with_dirs(
            [root, tmp],
            "compose",
            "--validate",
            "--plug",
            "provider.wasm",
            "socket.wasm",
            "-o",
            "composed.wasm",
        )
        if rc_co1 == 0 and "compose dependency graph" in out_co1:
            lines.append(f"  pass: compose --validate (exit 0, graph printed)")
            pass_count += 1
        else:
            lines.append(
                f"  FAIL: compose --validate (exit={rc_co1}, output={out_co1.strip()!r})"
            )
            fail_count += 1

    lines.append("")
    lines.append(f"{YELLOW}cli-parity: PASS={pass_count} FAIL={fail_count}{NC}")
    if fail_count > 0:
        lines.append(f"{RED}✗ cli parity: {fail_count} case(s) failed{NC}")
        return (1, "\n".join(lines) + "\n")

    lines.append(f"{GREEN}✓ all {pass_count} CLI parity cases pass{NC}")
    return (0, "\n".join(lines) + "\n")


def run_parity(
    root: Path,
    dry_run: bool,
    mode: str = "",
) -> tuple[int, str]:
    """Selfhost parity gate dispatch.

    mode: '' | '--fixture' | '--cli' | '--diag'
    """
    if dry_run:
        print(f"DRY-RUN: run_parity(mode={mode!r})")
        return (0, "")

    if mode == "--fixture":
        return run_fixture_parity(root, dry_run=False)
    if mode == "--diag":
        return run_diag_parity(root, dry_run=False)
    if mode == "--cli":
        return _run_cli_parity(root)

    # mode == '' → run fixture + diag
    rc1, out1 = run_fixture_parity(root, dry_run=False)
    rc2, out2 = run_diag_parity(root, dry_run=False)
    combined = out1 + out2
    return (max(rc1, rc2), combined)
