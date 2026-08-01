#!/usr/bin/env python3
"""Check the structured typed-contract compiler boundary."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise ValueError(f"missing {label}: {needle}")


def reject(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise ValueError(f"forbidden {label}: {needle}")


def main() -> int:
    tokens = read("src/compiler/lexer/tokens.ark")
    keywords = read("src/compiler/lexer/keywords_decl.ark")
    parser = read("src/compiler/parser/proof_contracts.ark")
    fn_sig = read("src/compiler/parser/fn_sig_decl.ark")
    checker = read("src/compiler/typechecker/proof_contracts.ark")
    body_checker = read("src/compiler/typechecker/body.ark")
    body_table = read("src/compiler/corehir/body_table.ark")
    body_roots = read("src/compiler/corehir/body_roots.ark")
    renderer = read("src/compiler/driver/typed_corehir_contract_render.ark")
    entry = read("src/compiler/driver/typed_corehir_render.ark")

    require(tokens, "fn TK_PROOF()", "proof token")
    require(keywords, 'eq(word, "proof")', "proof keyword")
    require(parser, "AstNode_push_type_ann(fn_node, contract)", "contract AST retention")
    require(parser, "expr::parse_expr(p)", "structured expression parser")
    require(fn_sig, "parse_optional_proof_block_into_node(p, node)", "function-header integration")

    require(checker, "infer::infer_expr", "contract type inference")
    require(checker, 'scope::scope_define(contract_scope, "result"', "ensures result binding")
    require(checker, "proof contract must have type bool", "bool contract gate")
    require(body_checker, "proof_contracts::check_proof_contracts", "body typecheck integration")

    require(body_table, "contract_roots: Vec<i32>", "contract root storage")
    require(body_table, "contract_kinds: Vec<String>", "contract kind storage")
    require(body_roots, "corehir_build_expr(table, expression)", "typed CoreHIR contract lowering")
    require(renderer, '"{\\\"kind\\\":"', "contract JSON rendering")
    require(renderer, '"expression_id"', "contract expression identity")
    require(renderer, "tccr_collect_reachable", "contract reachability integration")
    require(entry, "typed_corehir_contract_render::render_function", "contract renderer entrypoint")

    for text, label in (
        (parser, "parser"),
        (checker, "typechecker"),
        (body_roots, "CoreHIR retention"),
        (renderer, "TypedCoreHIR renderer"),
    ):
        reject(text, "S-expression", f"{label} string proof model")
        reject(text, "sexpr", f"{label} S-expression proof model")

    print("typed-contract-source: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as exc:
        print(f"typed-contract-source: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
