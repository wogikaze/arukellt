#!/usr/bin/env python3
"""Compile-time smoke for ark_gc_clear_root_slots compact clear helper."""

from __future__ import annotations

import os
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNTIME_C = ROOT / "src" / "compiler" / "native_c" / "runtime" / "ark_native_runtime.c"
RUNTIME_I = ROOT / "src" / "compiler" / "native_c" / "runtime"
CC = os.environ.get("ARUKELLT_CC", "clang-16")

SMOKE = r"""
#define _DEFAULT_SOURCE
#include "ark_native_runtime.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    char *argv[] = { "clear-slots-smoke", NULL };
    setenv("ARUKELLT_NATIVE_GC", "1", 1);
    ark_rt_init(1, argv);
    ark_object_header *a = NULL;
    ark_object_header *b = NULL;
    ark_object_header *c = NULL;
    a = (ark_object_header *)ark_rt_string_from_bytes((const uint8_t *)"a", 1);
    b = (ark_object_header *)ark_rt_string_from_bytes((const uint8_t *)"b", 1);
    c = (ark_object_header *)ark_rt_string_from_bytes((const uint8_t *)"c", 1);
    ark_gc_push_frame(3);
    ark_gc_set_root(0, &a);
    ark_gc_set_root(1, &b);
    ark_gc_set_root(2, &c);
    const size_t slots[] = { 0u, 2u };
    ark_gc_clear_root_slots(2, slots);
    assert(a == NULL);
    assert(b != NULL);
    assert(c == NULL);
    ark_gc_pop_frame();
    ark_rt_shutdown();
    puts("ok");
    return 0;
}
"""


def test_ark_gc_clear_root_slots_smoke() -> None:
    with tempfile.TemporaryDirectory(prefix="native-clear-slots-") as tmp:
        tmp_path = Path(tmp)
        src = tmp_path / "smoke.c"
        exe = tmp_path / "smoke"
        src.write_text(SMOKE, encoding="utf-8")
        compile_cmd = [
            CC,
            "-std=c99",
            "-Wall",
            "-Wextra",
            "-Wno-unused-function",
            "-O1",
            "-I",
            str(RUNTIME_I),
            str(src),
            str(RUNTIME_C),
            "-o",
            str(exe),
        ]
        built = subprocess.run(compile_cmd, cwd=ROOT, capture_output=True, text=True)
        assert built.returncode == 0, built.stderr[-2000:]
        ran = subprocess.run([str(exe)], cwd=ROOT, capture_output=True, text=True)
        assert ran.returncode == 0, ran.stderr[-1000:]
        assert "ok" in ran.stdout


if __name__ == "__main__":
    test_ark_gc_clear_root_slots_smoke()
    print("PASS")
