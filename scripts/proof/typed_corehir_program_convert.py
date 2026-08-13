"""TypedCoreHIR v1 -> VerifiedCore v1 program lowering for proof phases 1-3.

The source artifact stays expression-tree shaped. This converter lowers the
supported pure subset into explicit instructions and an acyclic CFG, and emits
direct call instructions only when the callee is another contracted function.
"""

from __future__ import annotations

import copy
from typing import Any

from proof.typed_corehir import validate_document as validate_typed_corehir
from proof.verified_core_typed import validate_typed_document

SOURCE_SCHEMA = "arukellt-typed-corehir"
SOURCE_VERSION = 1
CONVERTER = "arukellt-typed-corehir-converter-v3"


class ExplicitTypedCoreHirError(ValueError):
    pass


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ExplicitTypedCoreHirError(message)


def _verified_type(entry: dict[str, Any], path: str) -> dict[str, Any]:
    type_id = int(entry["id"])
    kind = str(entry["kind"])
    _require(kind in {"unit", "bool", "integer"}, f"{path}.kind: unsupported reachable proof type {kind!r}")
    representation = entry.get("representation")
    _require(isinstance(representation, dict), f"{path}.representation: expected object")
    rendered: dict[str, Any] = {
        "id": type_id,
        "kind": kind,
        "name": str(entry["name"]),
        "representation": {
            "wasm": copy.deepcopy(representation["wasm"]),
            "nullable": representation["nullable"],
            "size_bytes": representation["size_bytes"],
            "align_bytes": representation["align_bytes"],
        },
    }
    if kind == "integer":
        bits = entry.get("bits")
        signed = entry.get("signed")
        _require(type(bits) is int and bits in {32, 64}, f"{path}.bits: expected 32 or 64")
        _require(signed is True, f"{path}.signed: only signed integers are supported")
        expected = f"i{bits}"
        _require(entry.get("value_type") == expected, f"{path}.value_type: explicit integer representation mismatch")
        _require(representation.get("wasm") == [expected], f"{path}.representation.wasm: integer representation mismatch")
        rendered["bits"] = bits
        rendered["signed"] = True
    elif kind == "bool":
        _require(entry.get("value_type") == "i32" and representation.get("wasm") == ["i32"], f"{path}: bool representation mismatch")
    else:
        _require(entry.get("value_type") == "void" and representation.get("wasm") == [], f"{path}: unit representation mismatch")
    return rendered


_BINARY = {
    "+": "add", "-": "sub", "*": "mul", "/": "div", "%": "mod",
    "==": "eq", "!=": "ne", "<": "lt", "<=": "le", ">": "gt", ">=": "ge",
    "&&": "and", "||": "or",
}
_UNARY = {"-": "neg", "!": "not"}


def _expression_index(function: dict[str, Any], path: str) -> tuple[int, dict[int, dict[str, Any]]]:
    body = function.get("body")
    _require(isinstance(body, dict), f"{path}.body: expected object")
    root = body.get("root_expr_id")
    _require(type(root) is int and root >= 0, f"{path}.body.root_expr_id: invalid root")
    expressions = body.get("expressions")
    _require(isinstance(expressions, list), f"{path}.body.expressions: expected array")
    by_id: dict[int, dict[str, Any]] = {}
    for index, expression in enumerate(expressions):
        _require(isinstance(expression, dict), f"{path}.body.expressions[{index}]: expected object")
        expression_id = expression.get("id")
        _require(type(expression_id) is int and expression_id >= 0, f"{path}.body.expressions[{index}].id: invalid id")
        _require(expression_id not in by_id, f"{path}.body.expressions[{index}].id: duplicate id")
        by_id[expression_id] = expression
    _require(root in by_id, f"{path}.body.root_expr_id: unknown expression")
    return root, by_id


def _proof_expression(
    expression_id: int,
    expressions: dict[int, dict[str, Any]],
    local_ids: dict[str, int],
    result_name: str,
    next_id: list[int],
    used_types: set[int],
    stack: set[int],
    path: str,
) -> dict[str, Any]:
    _require(expression_id in expressions, f"{path}: unknown expression {expression_id}")
    _require(expression_id not in stack, f"{path}: expression cycle")
    source = expressions[expression_id]
    kind = str(source["kind"])
    type_id = int(source["type_id"])
    used_types.add(type_id)
    text = str(source.get("text", ""))
    children = [int(child) for child in source.get("children", [])]
    verified_id = next_id[0]
    next_id[0] += 1
    common = {"id": verified_id, "type_id": type_id}
    stack = {*stack, expression_id}
    if kind in {"ident", "path"}:
        if result_name and text == result_name:
            return {**common, "kind": "result"}
        _require(text in local_ids, f"{path}.text: unknown proof identifier {text!r}")
        return {**common, "kind": "local", "local_id": local_ids[text]}
    if kind == "int":
        return {**common, "kind": "constant", "value": int(source.get("int_value", 0))}
    if kind == "bool":
        lowered = text.lower()
        _require(lowered in {"true", "false"}, f"{path}.text: invalid bool literal")
        return {**common, "kind": "constant", "value": lowered == "true"}
    if kind == "binary":
        operator = _BINARY.get(text)
        _require(operator is not None and len(children) == 2, f"{path}: unsupported binary {text!r}")
        return {**common, "kind": operator, "operands": [
            _proof_expression(children[0], expressions, local_ids, result_name, next_id, used_types, stack, f"{path}.children[0]"),
            _proof_expression(children[1], expressions, local_ids, result_name, next_id, used_types, stack, f"{path}.children[1]"),
        ]}
    if kind == "unary":
        operator = _UNARY.get(text)
        _require(operator is not None and len(children) == 1, f"{path}: unsupported unary {text!r}")
        return {**common, "kind": operator, "operands": [
            _proof_expression(children[0], expressions, local_ids, result_name, next_id, used_types, stack, f"{path}.children[0]")
        ]}
    raise ExplicitTypedCoreHirError(f"{path}.kind: unsupported proof expression kind {kind!r}")


class _Lowerer:
    def __init__(
        self,
        function: dict[str, Any],
        expressions: dict[int, dict[str, Any]],
        contract_ids: set[int],
        callee_ids: dict[str, int | None],
        used_types: set[int],
        path: str,
    ) -> None:
        self.function = function
        self.expressions = expressions
        self.contract_ids = contract_ids
        self.callee_ids = callee_ids
        self.used_types = used_types
        self.path = path
        self.locals = [copy.deepcopy(local) for local in function["locals"]]
        self.local_ids = {str(local["name"]): int(local["id"]) for local in self.locals}
        self.local_types = {int(local["id"]): int(local["type_id"]) for local in self.locals}
        self.next_local = max(self.local_types, default=-1) + 1
        self.next_instruction = 0
        self.blocks: list[dict[str, Any]] = []
        self.new_block()

    def new_block(self) -> int:
        block_id = len(self.blocks)
        self.blocks.append({"id": block_id, "parameters": [], "instructions": [], "terminator": None})
        return block_id

    def add_local(self, name: str, type_id: int, storage: str) -> int:
        _require(name not in self.local_ids, f"{self.path}.locals: duplicate local name {name!r}")
        local_id = self.next_local
        self.next_local += 1
        self.local_ids[name] = local_id
        self.local_types[local_id] = type_id
        self.used_types.add(type_id)
        self.locals.append({"id": local_id, "name": name, "type_id": type_id, "storage": storage})
        return local_id

    def temp(self, type_id: int, prefix: str = "tmp") -> int:
        return self.add_local(f"${prefix}{self.next_local}", type_id, "temporary")

    def emit(self, block_id: int, instruction: dict[str, Any]) -> None:
        _require(self.blocks[block_id]["terminator"] is None, f"{self.path}.body: instruction after terminator")
        instruction["id"] = self.next_instruction
        self.next_instruction += 1
        self.blocks[block_id]["instructions"].append(instruction)

    def set_terminator(self, block_id: int, terminator: dict[str, Any]) -> None:
        _require(self.blocks[block_id]["terminator"] is None, f"{self.path}.body: duplicate terminator")
        self.blocks[block_id]["terminator"] = terminator

    def value_local(self, local_id: int) -> dict[str, Any]:
        return {"kind": "local", "type_id": self.local_types[local_id], "local_id": local_id}

    def _literal_or_local(self, expression: dict[str, Any], expr_path: str) -> dict[str, Any] | None:
        kind = str(expression["kind"])
        type_id = int(expression["type_id"])
        self.used_types.add(type_id)
        text = str(expression.get("text", ""))
        if kind in {"ident", "path"}:
            _require(text in self.local_ids, f"{expr_path}.text: unknown local {text!r}")
            return self.value_local(self.local_ids[text])
        if kind == "int":
            return {"kind": "constant", "type_id": type_id, "value": int(expression.get("int_value", 0))}
        if kind == "bool":
            lowered = text.lower()
            _require(lowered in {"true", "false"}, f"{expr_path}.text: invalid bool")
            return {"kind": "constant", "type_id": type_id, "value": lowered == "true"}
        return None

    def eval(self, expression_id: int, block_id: int, stack: set[int] | None = None) -> tuple[int, dict[str, Any] | None, bool]:
        stack = set() if stack is None else stack
        _require(expression_id in self.expressions, f"{self.path}.body: unknown expression {expression_id}")
        _require(expression_id not in stack, f"{self.path}.body: expression cycle at {expression_id}")
        stack = {*stack, expression_id}
        expression = self.expressions[expression_id]
        expr_path = f"{self.path}.body.expression[id={expression_id}]"
        simple = self._literal_or_local(expression, expr_path)
        if simple is not None:
            return block_id, simple, False
        kind = str(expression["kind"])
        type_id = int(expression["type_id"])
        self.used_types.add(type_id)
        children = [int(child) for child in expression.get("children", [])]

        if kind == "block":
            current = block_id
            last: dict[str, Any] | None = None
            active_children = [child for child in children if child not in self.contract_ids]
            for child_index, child in enumerate(active_children):
                current, last, terminated = self.eval(child, current, stack)
                if terminated:
                    _require(child_index == len(active_children) - 1, f"{expr_path}: statements follow terminal control flow")
                    return current, None, True
            return current, last, False

        if kind in {"binary", "unary"}:
            expected = 2 if kind == "binary" else 1
            _require(len(children) == expected, f"{expr_path}.children: wrong arity")
            current = block_id
            arguments: list[dict[str, Any]] = []
            for child in children:
                current, value, terminated = self.eval(child, current, stack)
                _require(not terminated and value is not None, f"{expr_path}: operand does not produce value")
                arguments.append(value)
            operator = (_BINARY if kind == "binary" else _UNARY).get(str(expression.get("text", "")))
            _require(operator is not None, f"{expr_path}: unsupported operator {expression.get('text')!r}")
            destination = self.temp(type_id)
            self.emit(current, {"op": kind, "operator": operator, "dest_local_id": destination, "type_id": type_id, "arguments": arguments})
            return current, self.value_local(destination), False

        if kind == "let":
            _require(len(children) == 1, f"{expr_path}: let requires initializer")
            current, value, terminated = self.eval(children[0], block_id, stack)
            _require(not terminated and value is not None, f"{expr_path}: initializer does not produce value")
            local_type = type_id if type_id != 0 else int(value["type_id"])
            local_id = self.add_local(str(expression.get("text", "")), local_type, "local")
            self.emit(current, {"op": "copy", "dest_local_id": local_id, "type_id": local_type, "arguments": [value]})
            return current, self.value_local(local_id), False

        if kind == "assign":
            _require(len(children) == 1, f"{expr_path}: assignment requires value")
            target_name = str(expression.get("text", ""))
            _require(target_name in self.local_ids, f"{expr_path}.text: unknown assignment target {target_name!r}")
            current, value, terminated = self.eval(children[0], block_id, stack)
            _require(not terminated and value is not None, f"{expr_path}: assignment value missing")
            destination = self.local_ids[target_name]
            self.emit(current, {"op": "copy", "dest_local_id": destination, "type_id": self.local_types[destination], "arguments": [value]})
            return current, self.value_local(destination), False

        if kind == "return":
            _require(len(children) <= 1, f"{expr_path}: return arity")
            if children:
                current, value, terminated = self.eval(children[0], block_id, stack)
                _require(not terminated and value is not None, f"{expr_path}: return value missing")
                self.set_terminator(current, {"kind": "return", "value": value})
                return current, None, True
            self.set_terminator(block_id, {"kind": "return"})
            return block_id, None, True

        if kind == "call":
            _require(len(children) >= 1, f"{expr_path}: call missing callee")
            callee_expr = self.expressions.get(children[0])
            _require(isinstance(callee_expr, dict) and callee_expr.get("kind") in {"ident", "path"}, f"{expr_path}: indirect calls are outside phase 3")
            callee_name = str(expression.get("text", "")) or str(callee_expr.get("text", ""))
            callee_id = self.callee_ids.get(callee_name)
            _require(callee_id is not None, f"{expr_path}.text: unknown or ambiguous contracted callee {callee_name!r}")
            current = block_id
            arguments: list[dict[str, Any]] = []
            for child in children[1:]:
                current, value, terminated = self.eval(child, current, stack)
                _require(not terminated and value is not None, f"{expr_path}: call argument missing")
                arguments.append(value)
            _require(type_id != 0, f"{expr_path}: unit-return direct calls are outside phase 3")
            destination = self.temp(type_id, "call")
            self.emit(current, {"op": "call", "callee_id": callee_id, "dest_local_id": destination, "type_id": type_id, "arguments": arguments})
            return current, self.value_local(destination), False

        if kind == "if":
            _require(len(children) in {2, 3}, f"{expr_path}: if requires condition/then[/else]")
            current, condition, terminated = self.eval(children[0], block_id, stack)
            _require(not terminated and condition is not None, f"{expr_path}: condition missing")
            then_block = self.new_block()
            else_block = self.new_block()
            self.set_terminator(current, {"kind": "branch", "condition": condition, "then_target": then_block, "else_target": else_block, "then_arguments": [], "else_arguments": []})
            then_end, then_value, then_term = self.eval(children[1], then_block, stack)
            if len(children) == 3:
                else_end, else_value, else_term = self.eval(children[2], else_block, stack)
            else:
                else_end, else_value, else_term = else_block, None, False
            continuing: list[tuple[int, dict[str, Any] | None]] = []
            if not then_term: continuing.append((then_end, then_value))
            if not else_term: continuing.append((else_end, else_value))
            if not continuing:
                return current, None, True
            join = self.new_block()
            result_value: dict[str, Any] | None = None
            if type_id != 0:
                _require(len(children) == 3, f"{expr_path}: value if requires else")
                result_local = self.temp(type_id, "phi")
                self.locals[-1]["storage"] = "block-parameter"
                self.blocks[join]["parameters"].append({"local_id": result_local, "type_id": type_id})
                result_value = self.value_local(result_local)
                for end, value in continuing:
                    _require(value is not None and int(value["type_id"]) == type_id, f"{expr_path}: branch result type mismatch")
                    self.set_terminator(end, {"kind": "goto", "target": join, "arguments": [value]})
            else:
                for end, _ in continuing:
                    self.set_terminator(end, {"kind": "goto", "target": join, "arguments": []})
            return join, result_value, False

        raise ExplicitTypedCoreHirError(f"{expr_path}.kind: unsupported executable expression {kind!r}")

    def finish(self, root_id: int, return_type: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
        end, value, terminated = self.eval(root_id, 0)
        if not terminated:
            if return_type == 0:
                self.set_terminator(end, {"kind": "return"})
            else:
                _require(value is not None and int(value["type_id"]) == return_type, f"{self.path}.body: final expression type does not match return type")
                self.set_terminator(end, {"kind": "return", "value": value})
        for block in self.blocks:
            _require(block["terminator"] is not None, f"{self.path}.body.blocks[{block['id']}]: unterminated block")
        return self.locals, {"entry_block": 0, "blocks": self.blocks}


def _callee_map(functions: list[dict[str, Any]]) -> dict[str, int | None]:
    result: dict[str, int | None] = {}
    for function in functions:
        function_id = int(function["id"])
        name = str(function["name"])
        aliases = {name, name.split("::")[-1]}
        for alias in aliases:
            if alias in result and result[alias] != function_id:
                result[alias] = None
            else:
                result[alias] = function_id
    return result


def convert_document(value: Any) -> dict[str, Any]:
    source = validate_typed_corehir(value)
    _require(source.get("schema") == SOURCE_SCHEMA and source.get("schema_version") == SOURCE_VERSION, "$: unsupported TypedCoreHIR schema")
    source_types = {int(entry["id"]): entry for entry in source["types"]}
    _require(0 in source_types and source_types[0].get("kind") == "unit", "$.types: type id 0 must be unit")
    contracted = [function for function in source["functions"] if function.get("contracts")]
    _require(contracted, "$.functions: no contracted functions")
    callee_ids = _callee_map(contracted)
    used_types: set[int] = {0}
    verified_functions: list[dict[str, Any]] = []

    for function_index, function in enumerate(contracted):
        path = f"$.functions[contracted={function_index}]"
        signature = copy.deepcopy(function["signature"])
        return_type = int(signature["return_type_id"])
        used_types.add(return_type)
        for parameter in signature["parameters"]: used_types.add(int(parameter["type_id"]))
        root_id, expressions = _expression_index(function, path)
        locals_rendered = [copy.deepcopy(local) for local in function["locals"]]
        local_ids = {str(local["name"]): int(local["id"]) for local in locals_rendered}
        for local in locals_rendered: used_types.add(int(local["type_id"]))
        contract_ids: set[int] = set()
        contracts: list[dict[str, Any]] = []
        next_proof_id = [0]
        for contract_index, contract in enumerate(function["contracts"]):
            contract_path = f"{path}.contracts[{contract_index}]"
            kind = str(contract["kind"])
            _require(kind in {"requires", "ensures"}, f"{contract_path}.kind: proof phases 1-3 support requires/ensures")
            expression_id = int(contract["expression_id"])
            contract_ids.add(expression_id)
            result_name = str(contract.get("result_name", "result" if kind == "ensures" else ""))
            rendered: dict[str, Any] = {"kind": kind, "expression": _proof_expression(expression_id, expressions, local_ids, result_name, next_proof_id, used_types, set(), f"{contract_path}.expression")}
            if result_name: rendered["result_name"] = result_name
            contracts.append(rendered)

        lowerer_source = copy.deepcopy(function)
        lowerer_source["locals"] = locals_rendered
        lowerer = _Lowerer(lowerer_source, expressions, contract_ids, callee_ids, used_types, path)
        final_locals, body = lowerer.finish(root_id, return_type)
        verified_functions.append({
            "id": int(function["id"]),
            "name": str(function["name"]),
            "signature": signature,
            "abi": copy.deepcopy(function["abi"]),
            "locals": final_locals,
            "contracts": contracts,
            "body": body,
        })

    missing = sorted(type_id for type_id in used_types if type_id not in source_types)
    _require(not missing, f"$.types: missing reachable TypeIds {missing}")
    result = {
        "schema": "arukellt-verified-core",
        "schema_version": 1,
        "generator": CONVERTER,
        "module": source["module"],
        "target_profile": copy.deepcopy(source["target_profile"]),
        "types": [_verified_type(source_types[type_id], f"$.types[id={type_id}]") for type_id in sorted(used_types)],
        "functions": verified_functions,
    }
    return validate_typed_document(result)


__all__ = ["CONVERTER", "ExplicitTypedCoreHirError", "SOURCE_SCHEMA", "SOURCE_VERSION", "convert_document"]
