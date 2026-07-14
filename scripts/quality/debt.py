"""High-confidence classification for small Ark forwarding functions."""

from __future__ import annotations

import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from .metrics import FN_START_RE, sanitize_ark_lines
from .structure import compiler_import_graph


CALL_RE = re.compile(
    r"^(?:return\s+)?([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"
    r"\s*\((.*)\)\s*;?$"
)
FIELD_RE = re.compile(r"^(?:return\s+)?[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*\s*;?$")
RECORD_RE = re.compile(r"^(?:return\s+)?[A-Za-z_][A-Za-z0-9_]*\s*\{")


@dataclass(frozen=True)
class WrapperClassification:
    symbol: str
    line: int
    category: str
    target: str | None


@dataclass(frozen=True)
class WrapperDebtInventory:
    categories: dict[str, int]
    unjustified_pure_forwarders: tuple[str, ...]
    wrapper_only_single_function_files: tuple[str, ...]


def _split_top_level(text: str) -> list[str]:
    pieces: list[str] = []
    current: list[str] = []
    depth = 0
    for char in text:
        if char in "([{<":
            depth += 1
        elif char in ")]}>":
            depth = max(0, depth - 1)
        if char == "," and depth == 0:
            pieces.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    if current or text.strip():
        pieces.append("".join(current).strip())
    return [piece for piece in pieces if piece]


def _parameter_names(signature: str) -> list[str]:
    start = signature.find("(")
    end = signature.rfind(")")
    if start < 0 or end <= start:
        return []
    return [
        piece.split(":", 1)[0].removeprefix("mut ").strip()
        for piece in _split_top_level(signature[start + 1 : end])
    ]


def _is_boundary(path: str, is_public: bool) -> bool:
    filename = path.rsplit("/", 1)[-1]
    stem = filename.removesuffix(".ark")
    if is_public or filename == "mod.ark" or path.count("/") == 2:
        return True
    return any(token in stem for token in ("facade", "adapter", "entry", "query"))


def _is_record_contract(path: str, symbol: str, statements: list[str]) -> bool:
    stem = path.rsplit("/", 1)[-1].removesuffix(".ark")
    if "record" in stem or symbol.endswith("_new") or "_new_" in symbol:
        return True
    return len(statements) == 1 and bool(
        FIELD_RE.match(statements[0]) or RECORD_RE.match(statements[0])
    )


def classify_wrappers(path: str, text: str) -> tuple[WrapperClassification, ...]:
    """Classify only functions that look like small wrappers with high confidence."""
    lines = sanitize_ark_lines(text)
    findings: list[WrapperClassification] = []
    index = 0
    while index < len(lines):
        match = FN_START_RE.match(lines[index])
        if not match:
            index += 1
            continue
        start = index
        signature_lines = [lines[index].strip()]
        while "{" not in " ".join(signature_lines) and index + 1 < len(lines):
            index += 1
            signature_lines.append(lines[index].strip())
        signature = " ".join(signature_lines)
        depth = 0
        opened = False
        end = index
        while end < len(lines):
            depth += lines[end].count("{") - lines[end].count("}")
            opened = opened or "{" in lines[end]
            if opened and depth <= 0:
                break
            end += 1
        body = "\n".join(lines[start : end + 1])
        body = body[body.find("{") + 1 : body.rfind("}")]
        statements = [line.strip().rstrip(";") for line in body.splitlines() if line.strip()]
        symbol = match.group(2)
        category = "ambiguous"
        target: str | None = None
        call = CALL_RE.match(statements[0]) if len(statements) == 1 else None
        if _is_record_contract(path, symbol, statements):
            category = "record_accessor_or_constructor"
        elif call:
            target = call.group(1)
            arguments = _split_top_level(call.group(2))
            if arguments == _parameter_names(signature):
                target_symbol = target.rsplit("::", 1)[-1]
                category = "pure_forwarder"
                if _is_boundary(path, bool(match.group(1))) or target_symbol != symbol:
                    category = "boundary_facade"
            else:
                category = "semantic_wrapper"
        elif len(statements) > 1 and len(statements) <= 5:
            category = "semantic_wrapper"
        findings.append(WrapperClassification(symbol, start + 1, category, target))
        index = max(index + 1, end + 1)
    return tuple(findings)


def collect_wrapper_debt(root: Path) -> WrapperDebtInventory:
    """Find only unused, one-function pure-forwarder files as removable debt."""
    graph = compiler_import_graph(root)
    fan_in: Counter[Path] = Counter()
    for dependencies in graph.values():
        fan_in.update(
            dependency.relative_to(root) if dependency.is_absolute() else dependency
            for dependency in dependencies
        )
    categories: Counter[str] = Counter()
    unjustified: list[str] = []
    wrapper_files: list[str] = []
    for path in sorted((root / "src/compiler").rglob("*.ark")):
        rel = str(path.relative_to(root))
        functions = classify_wrappers(rel, path.read_text(encoding="utf-8"))
        categories.update(item.category for item in functions)
        if (
            len(functions) == 1
            and functions[0].category == "pure_forwarder"
            and fan_in[path.relative_to(root)] == 0
        ):
            finding = f"{rel}:{functions[0].line}: {functions[0].symbol}"
            unjustified.append(finding)
            wrapper_files.append(rel)
    return WrapperDebtInventory(
        dict(sorted(categories.items())),
        tuple(unjustified),
        tuple(wrapper_files),
    )
