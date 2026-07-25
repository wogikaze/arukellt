#!/usr/bin/env python3
"""Public host launcher for `arukellt run --target native-cpp` (ADR-050).

Owns: temporary C generation via selfhost compile, clang link, process exec.
Does not own: MIR→C lowering (selfhost compiler) or the internal executor lane.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from native.toolchain import ClangToolchain, resolve_clang  # noqa: E402

WRAPPER = REPO_ROOT / "scripts" / "run" / "arukellt-selfhost.sh"
RUNTIME_C = REPO_ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.c"
RUNTIME_H = REPO_ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.h"
RUNTIME_ABI_VERSION = 1
INTERNAL_COMPILE_ENV = "ARUKELLT_NATIVE_CPP_INTERNAL_COMPILE"
KEEP_TEMP_ENV = "ARUKELLT_NATIVE_CPP_KEEP_TEMP"
VERBOSE_ENV = "ARUKELLT_NATIVE_CPP_VERBOSE"


def _build_root() -> Path:
    """Prefer ARUKELLT_BUILD_DIR when it stays inside the repo (WASI --dir=repo)."""
    override = os.environ.get("ARUKELLT_BUILD_DIR", "").strip()
    if override:
        path = Path(override).resolve()
        try:
            path.relative_to(REPO_ROOT.resolve())
            return path
        except ValueError:
            pass
    return REPO_ROOT / ".build"


def _cache_dir() -> Path:
    return _build_root() / "native-cpp" / "run-cache"


def _repo_relative(path: Path) -> str:
    return str(path.resolve().relative_to(REPO_ROOT.resolve()))


def _is_verbose() -> bool:
    return os.environ.get(VERBOSE_ENV, "").strip() in {"1", "true", "TRUE", "yes", "YES"}


def _keep_temp() -> bool:
    return os.environ.get(KEEP_TEMP_ENV, "").strip() in {"1", "true", "TRUE", "yes", "YES"}


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def parse_run_argv(argv: list[str]) -> tuple[list[str], list[str]]:
    """Split `run ...` argv into (compiler_args_including_run, program_args)."""
    if not argv or argv[0] != "run":
        raise ValueError("native-cpp-runner expects argv to start with `run`")
    compiler_args = ["run"]
    program_args: list[str] = []
    i = 1
    while i < len(argv):
        if argv[i] == "--":
            program_args = argv[i + 1 :]
            break
        compiler_args.append(argv[i])
        i += 1
    return compiler_args, program_args


def extract_target(compiler_args: list[str]) -> str | None:
    i = 0
    while i < len(compiler_args):
        arg = compiler_args[i]
        if arg == "--target" and i + 1 < len(compiler_args):
            return compiler_args[i + 1]
        if arg.startswith("--target="):
            return arg.split("=", 1)[1]
        i += 1
    return None


def extract_input_file(compiler_args: list[str]) -> str | None:
    """First non-option argument after `run` (before `--`)."""
    i = 1
    while i < len(compiler_args):
        arg = compiler_args[i]
        if arg in {"--target", "--wasi-version", "--opt-level", "--emit", "--wit", "--world",
                   "--dump-phases", "--cache-dir", "--allow", "--deny", "-o", "--output",
                   "--template"}:
            i += 2
            continue
        if arg.startswith("-"):
            i += 1
            continue
        return arg
    return None


def extract_opt_level(compiler_args: list[str]) -> int:
    i = 0
    while i < len(compiler_args):
        if compiler_args[i] == "--opt-level" and i + 1 < len(compiler_args):
            try:
                return int(compiler_args[i + 1])
            except ValueError:
                return 0
        i += 1
    return 0


def clang_opt_flag(opt_level: int) -> str:
    if opt_level <= 0:
        return "-O0"
    if opt_level == 1:
        return "-O1"
    return "-O2"


def _runtime_pair_ok(directory: Path) -> bool:
    return (directory / "ark_native_runtime.c").is_file() and (
        directory / "ark_native_runtime.h"
    ).is_file()


def resolve_runtime_dir() -> tuple[Path | None, str]:
    """Discovery order (ADR-050 / Phase 7):

    1. `ARUKELLT_NATIVE_RUNTIME_DIR`
    2. launcher-adjacent installed prefix: `<prefix>/lib/arukellt/native-cpp`
       where `<prefix>/bin` contains this runner or `arukellt`
    3. source-tree runtime under the repository checkout
    """
    override = os.environ.get("ARUKELLT_NATIVE_RUNTIME_DIR", "").strip()
    if override:
        path = Path(override)
        if _runtime_pair_ok(path):
            return path, ""
        return None, f"runtime diagnostic: ARUKELLT_NATIVE_RUNTIME_DIR={override} is incomplete"

    here = Path(__file__).resolve()
    # Installed layout: <prefix>/bin/... → <prefix>/lib/arukellt/native-cpp
    for anchor in (here.parent, here.parent.parent, here.parent.parent.parent):
        if anchor.name == "bin":
            installed = anchor.parent / "lib" / "arukellt" / "native-cpp"
            if _runtime_pair_ok(installed):
                return installed, ""
        candidate = anchor / "lib" / "arukellt" / "native-cpp"
        if _runtime_pair_ok(candidate):
            return candidate, ""

    if _runtime_pair_ok(RUNTIME_C.parent):
        return RUNTIME_C.parent, ""
    return None, (
        "runtime diagnostic: native-cpp runtime not found; set ARUKELLT_NATIVE_RUNTIME_DIR "
        "or install to <prefix>/lib/arukellt/native-cpp "
        f"(source tree expected {RUNTIME_C} and {RUNTIME_H})"
    )


def compile_to_c(source: Path, output_c: Path, compiler_args: list[str]) -> int:
    """Invoke selfhost compile for native-cpp C emission. Returns exit code."""
    source_arg = _repo_relative(source) if source.is_absolute() else str(source)
    cmd = [
        str(WRAPPER),
        "compile",
        source_arg,
        "--target",
        "native-cpp",
        "--emit",
        "c",
        "-o",
        _repo_relative(output_c),
    ]
    # Forward selected compiler options from the public run command.
    i = 1
    while i < len(compiler_args):
        arg = compiler_args[i]
        if arg in {"--opt-level", "--cache-dir", "--dump-phases", "--allow", "--deny"}:
            if i + 1 < len(compiler_args):
                cmd.extend([arg, compiler_args[i + 1]])
                i += 2
                continue
        if arg in {"--show-timing", "--json"} or arg.startswith("--dump"):
            cmd.append(arg)
        i += 1

    env = os.environ.copy()
    env[INTERNAL_COMPILE_ENV] = "1"
    if _is_verbose():
        print("+", " ".join(cmd), file=sys.stderr)
    result = subprocess.run(cmd, cwd=str(REPO_ROOT), env=env, check=False)
    return result.returncode


def public_clang_command(
    clang: ClangToolchain,
    *,
    generated_c: Path,
    runtime_dir: Path,
    output_exe: Path,
    opt_level: int,
) -> list[str]:
    return [
        clang.path,
        "-std=c99",
        clang_opt_flag(opt_level),
        "-DNDEBUG",
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        f"-I{runtime_dir}",
        str(generated_c),
        str(runtime_dir / "ark_native_runtime.c"),
        "-lm",
        "-o",
        str(output_exe),
    ]


def cache_key(
    *,
    generated_c: Path,
    runtime_dir: Path,
    clang: ClangToolchain,
    opt_level: int,
) -> str:
    payload = "\n".join(
        [
            f"abi={RUNTIME_ABI_VERSION}",
            f"c={_sha256_file(generated_c)}",
            f"runtime_c={_sha256_file(runtime_dir / 'ark_native_runtime.c')}",
            f"runtime_h={_sha256_file(runtime_dir / 'ark_native_runtime.h')}",
            f"clang={clang.identity}",
            f"opt={clang_opt_flag(opt_level)}",
            "flags=-std=c99 -DNDEBUG -Wall -Wextra -Wpedantic -lm",
            f"host={os.uname().sysname}-{os.uname().machine}",
        ]
    )
    return _sha256_text(payload)


def map_child_exit(returncode: int) -> int:
    if returncode < 0:
        return 128 + (-returncode)
    return returncode


def main(argv: list[str]) -> int:
    try:
        compiler_args, program_args = parse_run_argv(argv)
    except ValueError as exc:
        print(f"native-cpp-runner: {exc}", file=sys.stderr)
        return 2

    target = extract_target(compiler_args)
    if target != "native-cpp":
        print(
            f"native-cpp-runner: refusing target `{target or '<default>'}`; "
            "expected --target native-cpp",
            file=sys.stderr,
        )
        return 2

    source_arg = extract_input_file(compiler_args)
    if not source_arg:
        print("native-cpp-runner: missing input .ark file", file=sys.stderr)
        return 2
    source = Path(source_arg)
    if not source.is_file():
        # Allow repo-relative paths from cwd.
        candidate = Path.cwd() / source
        if candidate.is_file():
            source = candidate
        else:
            print(f"native-cpp-runner: input not found: {source_arg}", file=sys.stderr)
            return 2

    clang, clang_diag = resolve_clang()
    if clang is None:
        print(clang_diag, file=sys.stderr)
        return 127

    runtime_dir, runtime_diag = resolve_runtime_dir()
    if runtime_dir is None:
        print(runtime_diag, file=sys.stderr)
        return 1

    opt_level = extract_opt_level(compiler_args)
    cache_root = _cache_dir()
    cache_root.mkdir(parents=True, exist_ok=True)

    temp_dir = Path(tempfile.mkdtemp(prefix="arukellt-native-cpp-", dir=str(cache_root)))
    generated_c = temp_dir / "program.c"
    temp_exe = temp_dir / "program"

    try:
        compile_rc = compile_to_c(source, generated_c, compiler_args)
        if compile_rc != 0:
            return compile_rc
        if not generated_c.is_file():
            print("native-cpp-runner: compile succeeded but C output is missing", file=sys.stderr)
            return 1

        key = cache_key(
            generated_c=generated_c,
            runtime_dir=runtime_dir,
            clang=clang,
            opt_level=opt_level,
        )
        cached_exe = cache_root / f"{key}.exe"
        key_sidecar = cache_root / f"{key}.key"
        cache_hit = (
            cached_exe.is_file()
            and os.access(cached_exe, os.X_OK)
            and key_sidecar.is_file()
            and key_sidecar.read_text(encoding="utf-8").strip() == key
        )
        if _is_verbose():
            print(f"native-cpp-runner: cache {'hit' if cache_hit else 'miss'} key={key[:12]}", file=sys.stderr)

        if not cache_hit:
            clang_cmd = public_clang_command(
                clang,
                generated_c=generated_c,
                runtime_dir=runtime_dir,
                output_exe=temp_exe,
                opt_level=opt_level,
            )
            if _is_verbose():
                print("+", " ".join(clang_cmd), file=sys.stderr)
            clang_result = subprocess.run(clang_cmd, cwd=str(REPO_ROOT), check=False)
            if clang_result.returncode != 0:
                print(
                    f"native-cpp-runner: clang failed (exit {clang_result.returncode}); "
                    f"generated C: {generated_c}",
                    file=sys.stderr,
                )
                if _keep_temp():
                    print(f"native-cpp-runner: kept temp dir {temp_dir}", file=sys.stderr)
                    # Prevent finally from deleting when keep is set — handled below.
                return clang_result.returncode
            if not temp_exe.is_file():
                print("native-cpp-runner: clang produced no executable", file=sys.stderr)
                return 1
            os.chmod(temp_exe, 0o755)
            staging = cache_root / f".{key}.staging"
            shutil.copy2(temp_exe, staging)
            os.replace(staging, cached_exe)
            key_sidecar.write_text(key + "\n", encoding="utf-8")
            os.chmod(cached_exe, 0o755)

        env = os.environ.copy()
        # Public run defaults to GC on; explicit override still wins.
        if "ARUKELLT_NATIVE_GC" not in env:
            env["ARUKELLT_NATIVE_GC"] = "1"

        if _keep_temp():
            print(f"native-cpp-runner: kept temp dir {temp_dir}", file=sys.stderr)
            print(f"native-cpp-runner: executable {cached_exe}", file=sys.stderr)
            result = subprocess.run(
                [str(cached_exe), *program_args],
                cwd=str(Path.cwd()),
                env=env,
                check=False,
            )
            return map_child_exit(result.returncode)

        # Normal path: drop temp artifacts, then replace this process so signals
        # and exit codes match a direct native executable launch.
        shutil.rmtree(temp_dir, ignore_errors=True)
        os.execve(str(cached_exe), [str(cached_exe), *program_args], env)
        return 1  # pragma: no cover — execve does not return
    finally:
        if not _keep_temp() and temp_dir.is_dir():
            # If execve succeeded this finally does not run in the child image;
            # if we returned early, clean up.
            shutil.rmtree(temp_dir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
