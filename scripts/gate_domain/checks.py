"""Gate domain check runners — pure Python, no shell script delegation."""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def _exec(cmd: list[str], cwd: Path, dry_run: bool) -> tuple[int, str]:
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "")
    result = subprocess.run(
        cmd, cwd=str(cwd), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
    )
    return (result.returncode, result.stdout)


def _run(cmd: list[str], cwd: Path, env: dict | None = None) -> tuple[int, str]:
    """Run a command, returning (returncode, combined output)."""
    result = subprocess.run(
        cmd,
        cwd=str(cwd),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=env,
    )
    return (result.returncode, result.stdout)


# ── pre-commit ────────────────────────────────────────────────────────────────

def run_pre_commit(root: Path, dry_run: bool) -> tuple[int, str]:
    """Fast pre-commit hook: calls manager.py verify --quick."""
    cmd = [sys.executable, "scripts/manager.py", "verify", "--quick"]
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "")
    out_lines: list[str] = []
    out_lines.append("Running pre-commit checks (quick harness)...\n")
    rc, out = _run(cmd, root)
    out_lines.append(out)
    if rc != 0:
        out_lines.append("verify-harness quick check failed.\n")
        return (rc, "".join(out_lines))
    out_lines.append("All pre-commit checks passed!\n")
    return (0, "".join(out_lines))


# ── pre-push ──────────────────────────────────────────────────────────────────

def _git_changed_files(root: Path) -> str:
    """Return newline-separated list of changed files relative to upstream."""
    # Try to find upstream ref
    r = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        cwd=str(root), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
    )
    if r.returncode == 0 and r.stdout.strip():
        upstream = r.stdout.strip()
        base_r = subprocess.run(
            ["git", "merge-base", "HEAD", upstream],
            cwd=str(root), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        base = base_r.stdout.strip()
        changed_r = subprocess.run(
            ["git", "diff", "--name-only", f"{base}...HEAD"],
            cwd=str(root), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        return changed_r.stdout
    else:
        changed_r = subprocess.run(
            ["git", "diff", "--name-only", "HEAD~1...HEAD"],
            cwd=str(root), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        return changed_r.stdout


def _has_rust_changes(changed: str) -> bool:
    import re
    return bool(re.search(
        r'^(src/|tests/|benches/|examples/)',
        changed, re.MULTILINE
    ))


def _has_doc_changes(changed: str) -> bool:
    import re
    return bool(re.search(
        r'^(docs/|issues/|scripts/gen/generate-docs\.py|scripts/gen/generate-issue-index\.sh|std/manifest\.toml|README\.md)',
        changed, re.MULTILINE
    ))


def _has_extension_changes(changed: str) -> bool:
    import re
    return bool(re.search(r'^extensions/', changed, re.MULTILINE))


def run_pre_push(root: Path, dry_run: bool) -> tuple[int, str]:
    """Lightweight pre-push gate: quick verification, docs freshness, extension syntax."""
    cmd_repr = ["<pre-push logic>"]
    if dry_run:
        print(f"DRY-RUN: {cmd_repr}")
        return (0, "")

    env = os.environ.copy()

    out_lines: list[str] = []

    def emit(msg: str) -> None:
        print(msg)
        out_lines.append(msg + "\n")

    emit("=== arukellt Pre-Push (lightweight gate) ===")

    changed = _git_changed_files(root)

    rust_changed = _has_rust_changes(changed) or not changed.strip()
    doc_changed = _has_doc_changes(changed) or rust_changed

    # 1. Selfhost checks
    if rust_changed:
        emit("\n── Selfhost quick checks ──")
        cmd = [sys.executable, "scripts/manager.py", "verify", "--quick"]
        rc, out = _run(cmd, root, env=env)
        out_lines.append(out)
        if rc != 0:
            emit(f"FAILED: {' '.join(cmd)}")
            return (rc, "".join(out_lines))
    else:
        emit("\n⊙ No compiler changes — skipping quick verification")

    # 2. Docs freshness
    if doc_changed:
        emit("\n── Docs freshness ──")
        for cmd in [
            [sys.executable, "scripts/gen/generate-docs.py", "--check"],
            ["bash", "scripts/gen/generate-issue-index.sh"],
        ]:
            rc, out = _run(cmd, root)
            out_lines.append(out)
            if rc != 0:
                emit(f"FAILED: {' '.join(cmd)}")
                return (rc, "".join(out_lines))
        rc, out = _run(["git", "diff", "--exit-code", "--", "docs/", "issues/", "README.md"], root)
        out_lines.append(out)
        if rc != 0:
            emit("FAILED: docs/issues/README.md have uncommitted changes after generation")
            return (rc, "".join(out_lines))
    else:
        emit("\n⊙ No doc changes — skipping docs freshness")

    # 3. Extension syntax check
    if _has_extension_changes(changed):
        emit("\n── Extension syntax check ──")
        ext_js = root / "extensions/arukellt-all-in-one/src/extension.js"
        if ext_js.exists():
            rc, out = _run(["node", "--check", str(ext_js)], root)
            out_lines.append(out)
            if rc != 0:
                emit("FAILED: node --check extension.js")
                return (rc, "".join(out_lines))
        pkg_json = root / "extensions/arukellt-all-in-one/package.json"
        if pkg_json.exists():
            import json
            try:
                json.loads(pkg_json.read_text())
                emit("  ✓ package.json valid JSON")
            except json.JSONDecodeError as e:
                emit(f"  ✗ package.json invalid JSON: {e}")
                return (1, "".join(out_lines))

    emit("\n=== Pre-push passed ===")
    emit("Fixture tests run in CI. For full local checks: scripts/gate_domain/checks.py run_local")
    return (0, "".join(out_lines))


# ── reproducible build ────────────────────────────────────────────────────────

def run_repro(
    root: Path,
    dry_run: bool,
    fixture: str = "",
    target: str = "",
    verbose: bool = False,
) -> tuple[int, str]:
    """Assert that compiling the same source twice produces identical .wasm output."""
    cmd: list[str] = ["<repro logic>"]
    opts: list[str] = []
    if fixture:
        opts += ["--fixture", fixture]
    if target:
        opts += ["--target", target]
    if verbose:
        opts.append("--verbose")
    if dry_run:
        print(f"DRY-RUN: {cmd + opts}")
        return (0, "")

    fixture_path = fixture or "tests/fixtures/hello/hello.ark"
    target_str = target or "wasm32"

    out_lines: list[str] = []

    def emit(msg: str) -> None:
        print(msg)
        out_lines.append(msg + "\n")

    # Locate compiler binary
    arukellt_bin = os.environ.get("ARUKELLT_BIN", "")
    if not arukellt_bin:
        for candidate in ["./target/debug/arukellt", "./target/release/arukellt", "scripts/run/arukellt-selfhost.sh"]:
            if (root / candidate).is_file() and os.access(root / candidate, os.X_OK):
                arukellt_bin = candidate
                break

    if not arukellt_bin or not os.access(root / arukellt_bin, os.X_OK):
        emit("✗ reproducible build: compiler binary not found (build first or set ARUKELLT_BIN)")
        return (2, "".join(out_lines))

    if not (root / fixture_path).is_file():
        emit(f"✗ reproducible build: fixture not found: {fixture_path}")
        return (1, "".join(out_lines))

    emit(f"[reproducible-build] Compiling '{fixture_path}' twice (target: {target_str})...")

    with tempfile.TemporaryDirectory(prefix="ark-repro-") as tmpdir:
        out1 = os.path.join(tmpdir, "build1.wasm")
        out2 = os.path.join(tmpdir, "build2.wasm")

        # First compilation
        rc1, _ = _run(
            [arukellt_bin, "compile", fixture_path, "--target", target_str, "-o", out1],
            root
        )
        if rc1 != 0:
            emit("✗ reproducible build: first compilation failed")
            return (1, "".join(out_lines))

        # Second compilation
        rc2, _ = _run(
            [arukellt_bin, "compile", fixture_path, "--target", target_str, "-o", out2],
            root
        )
        if rc2 != 0:
            emit("✗ reproducible build: second compilation failed")
            return (1, "".join(out_lines))

        def sha256file(path: str) -> str:
            h = hashlib.sha256()
            with open(path, "rb") as f:
                for chunk in iter(lambda: f.read(65536), b""):
                    h.update(chunk)
            return h.hexdigest()

        sha1 = sha256file(out1)
        sha2 = sha256file(out2)

        if verbose:
            emit(f"  build1 sha256: {sha1}")
            emit(f"  build2 sha256: {sha2}")

        if sha1 == sha2:
            size = os.path.getsize(out1)
            emit(f"✓ reproducible build: both outputs identical (sha256={sha1}, {size} bytes)")
            return (0, "".join(out_lines))

        emit("✗ reproducible build: outputs differ!")
        out_lines.append(f"  build1 sha256: {sha1}\n")
        out_lines.append(f"  build2 sha256: {sha2}\n")

        # WAT diff if available
        if shutil.which("wasm2wat"):
            wat1 = os.path.join(tmpdir, "build1.wat")
            wat2 = os.path.join(tmpdir, "build2.wat")
            subprocess.run(["wasm2wat", out1, "-o", wat1], cwd=str(root),
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            subprocess.run(["wasm2wat", out2, "-o", wat2], cwd=str(root),
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if os.path.isfile(wat1) and os.path.isfile(wat2):
                diff_r = subprocess.run(
                    ["diff", wat1, wat2], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
                )
                diff_lines = diff_r.stdout.splitlines()[:40]
                out_lines.append("  WAT diff (first 40 lines):\n")
                out_lines.extend(l + "\n" for l in diff_lines)
        else:
            out_lines.append("  (install wasm2wat / wabt for WAT-level diff)\n")

        # Binary diff summary via cmp
        cmp_r = subprocess.run(
            ["cmp", "-l", out1, out2],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        cmp_lines = cmp_r.stdout.splitlines()[:20]
        out_lines.append("  Binary diff summary:\n")
        out_lines.extend(l + "\n" for l in cmp_lines)

        return (1, "".join(out_lines))


# ── ci-full-local ─────────────────────────────────────────────────────────────

def run_local(root: Path, dry_run: bool, skip_ext: bool = False) -> tuple[int, str]:
    """Full local CI gate — equivalent to complete GitHub Actions pipeline."""
    cmd: list[str] = ["<ci-full-local logic>"]
    if skip_ext:
        cmd.append("--skip-ext")
    if dry_run:
        print(f"DRY-RUN: {cmd}")
        return (0, "")

    env = os.environ.copy()
    env["RUSTFLAGS"] = "-D warnings"
    env["CARGO_TERM_COLOR"] = "always"

    out_lines: list[str] = []
    passed = 0
    failed = 0
    skipped = 0

    def emit(msg: str) -> None:
        print(msg)
        out_lines.append(msg + "\n")

    def step(n: int, name: str) -> None:
        emit(f"\n══ Layer {n}: {name} ══")

    def ok(name: str) -> None:
        nonlocal passed
        emit(f"  ✓ {name}")
        passed += 1

    def skip_step(name: str) -> None:
        nonlocal skipped
        emit(f"  ⊙ {name} (skipped)")
        skipped += 1

    def fail_step(
        name: str,
        *,
        category: str = "",
        command: str = "",
        primary_path: str = "",
    ) -> None:
        nonlocal failed
        emit(f"  ✗ {name}")
        if category:
            emit(f"    category: {category}")
        if command:
            emit(f"    command: {command}")
        if primary_path:
            emit(f"    primary path: {primary_path}")
        failed += 1

    emit("=== arukellt Full Local CI ===")

    # ── 1. Quick manager verification ──
    step(1, "Quick verification")
    rc, out = _run([sys.executable, "scripts/manager.py", "verify", "--quick"], root, env=env)
    out_lines.append(out)
    if rc != 0:
        fail_step("Quick verification")
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    ok("Quick checks")

    # ── 2. Docs ──
    step(2, "Docs (freshness + consistency)")
    rc, out = _run([sys.executable, "scripts/manager.py", "verify", "--docs"], root)
    out_lines.append(out)
    if rc != 0:
        fail_step("Docs: manager.py verify --docs")
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    rc, out = _run([sys.executable, "scripts/check/check-docs-consistency.py"], root)
    out_lines.append(out)
    if rc != 0:
        fail_step("Docs: check-docs-consistency.py")
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    ok("Docs checks")

    # ── 3. T3 Fixtures ──
    step(3, "Fixtures T3 (wasm32-gc)")
    env3 = env.copy()
    env3["ARUKELLT_TARGET"] = "wasm32-gc"
    rc, out = _run([sys.executable, "scripts/manager.py", "verify", "--fixtures"], root, env=env3)
    out_lines.append(out)
    if rc != 0:
        fail_step(
            "T3 fixtures",
            category="fixture",
            command="python3 scripts/manager.py verify --fixtures",
            primary_path="tests/fixtures/manifest.txt",
        )
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    ok("T3 fixtures")

    # ── 4. T1 Fixtures (non-blocking) ──
    step(4, "Fixtures T1 (wasm32, non-blocking)")
    env4 = env.copy()
    env4["ARUKELLT_TARGET"] = "wasm32"
    rc, out = _run([sys.executable, "scripts/manager.py", "verify", "--fixtures"], root, env=env4)
    out_lines.append(out)
    if rc == 0:
        ok("T1 fixtures")
    else:
        fail_step(
            "T1 fixtures (non-blocking — recorded but not fatal)",
            category="fixture",
            command="python3 scripts/manager.py verify --fixtures",
            primary_path="tests/fixtures/manifest.txt",
        )

    # ── 5. Selfhost wrapper setup ──
    step(5, "Selfhost wrapper setup")
    rc, out = _run(
        ["bash", "-c", "mkdir -p target/release && cp scripts/run/arukellt-selfhost.sh target/release/arukellt && chmod +x target/release/arukellt"],
        root,
    )
    out_lines.append(out)
    if rc != 0:
        fail_step("Selfhost wrapper setup")
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    ok("Selfhost wrapper ready")

    # ── 6. Integration & Packaging ──
    step(6, "Integration & Packaging")
    smoke = root / "scripts/run/smoke-test-binary.sh"
    if smoke.is_file() and os.access(smoke, os.X_OK):
        rc, out = _run(["bash", str(smoke), "./target/release/arukellt"], root)
        out_lines.append(out)
        if rc != 0:
            fail_step("Smoke test")
            emit("✗ Full CI failed")
            return (rc, "".join(out_lines))
    pkg_ws = root / "scripts/run/test-package-workspace.sh"
    if pkg_ws.is_file() and os.access(pkg_ws, os.X_OK):
        rc, out = _run(["bash", str(pkg_ws)], root)
        out_lines.append(out)
        if rc != 0:
            fail_step(
                "Package workspace",
                category="package-workspace",
                command="bash scripts/run/test-package-workspace.sh",
                primary_path="tests/package-workspace/",
            )
            emit("✗ Full CI failed")
            return (rc, "".join(out_lines))
    ok("Integration & packaging")

    # ── 7. Determinism ──
    step(7, "Determinism")
    hello_ark = root / "docs/examples/hello.ark"
    if hello_ark.is_file():
        with tempfile.NamedTemporaryFile(delete=False) as ta, \
             tempfile.NamedTemporaryFile(delete=False) as tb:
            tmp_a, tmp_b = ta.name, tb.name
        try:
            bin_path = "./target/release/arukellt"
            for tgt, label in [("wasm32-gc", "T3"), ("wasm32", "T1")]:
                _run([bin_path, "compile", "--target", tgt, "--output", tmp_a, str(hello_ark)], root)
                _run([bin_path, "compile", "--target", tgt, "--output", tmp_b, str(hello_ark)], root)
                rc, out = _run(["diff", tmp_a, tmp_b], root)
                out_lines.append(out)
                if rc == 0:
                    ok(f"{label} deterministic")
                else:
                    fail_step(f"{label} deterministic")
        finally:
            for f in [tmp_a, tmp_b]:
                try:
                    os.unlink(f)
                except OSError:
                    pass
    else:
        skip_step("Determinism (hello.ark not found)")

    # ── 8. Selfhost Stage 0 ──
    step(8, "Selfhost Stage 0")
    rc, out = _run(["bash", "scripts/run/verify-bootstrap.sh", "--stage1-only"], root)
    out_lines.append(out)
    if rc != 0:
        fail_step(
            "Selfhost stage 0",
            category="bootstrap",
            command="bash scripts/run/verify-bootstrap.sh --stage1-only",
            primary_path="src/compiler/main.ark",
        )
        emit("✗ Full CI failed")
        return (rc, "".join(out_lines))
    ok("Selfhost stage 0")

    # ── 9. Component Interop + Size + WAT ──
    step(9, "Component interop + size + WAT")
    for args in [["--component"], ["--size", "--wat"]]:
        rc, out = _run([sys.executable, "scripts/manager.py", "verify"] + args, root)
        out_lines.append(out)
        if rc != 0:
            category = "component-interop" if "--component" in args else "target-contract"
            primary_path = "tests/component-interop/" if "--component" in args else "scripts/run/wat-roundtrip.sh"
            fail_step(
                f"Component/size/WAT: {' '.join(args)}",
                category=category,
                command=f"python3 scripts/manager.py verify {' '.join(args)}",
                primary_path=primary_path,
            )
            emit("✗ Full CI failed")
            return (rc, "".join(out_lines))
    ok("Component + size + WAT")

    # ── 10. Extension Tests ──
    step(10, "VS Code Extension Tests")
    if skip_ext:
        skip_step("Extension tests (--skip-ext)")
    elif shutil.which("npm") and shutil.which("xvfb-run"):
        ext_dir = str(root / "extensions/arukellt-all-in-one")
        rc1, out1 = _run(["npm", "ci", "--quiet"], Path(ext_dir))
        out_lines.append(out1)
        rc2, out2 = _run(["xvfb-run", "-a", "npm", "test"], Path(ext_dir))
        out_lines.append(out2)
        rc3, out3 = _run(["xvfb-run", "-a", "npm", "run", "test:vsix-live"], Path(ext_dir))
        out_lines.append(out3)
        if rc1 == 0 and rc2 == 0 and rc3 == 0:
            ok("Extension tests")
        else:
            fail_step(
                "Extension tests",
                category="editor-tooling",
                command="xvfb-run -a npm test && xvfb-run -a npm run test:vsix-live",
                primary_path="extensions/arukellt-all-in-one/",
            )
            emit("✗ Full CI failed")
            return (max(rc1, rc2, rc3), "".join(out_lines))
    else:
        skip_step("Extension tests (npm or xvfb-run not found)")

    # ── Summary ──
    emit("\n══════════════════════════════════════════")
    emit(f"  Passed: {passed}  Failed: {failed}  Skipped: {skipped}")
    if failed > 0:
        emit("✗ Full CI failed")
        return (1, "".join(out_lines))
    emit("✓ Full CI passed")
    return (0, "".join(out_lines))
