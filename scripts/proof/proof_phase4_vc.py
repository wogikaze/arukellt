"""Phase 4 invariant-based VC rendering for annotated while loops."""
from __future__ import annotations
from typing import Any
from proof import smtlib_v1 as legacy
from proof.loop_semantics import has_loops
from proof.verified_core_typed import validate_typed_document

UnsupportedVerifiedCore = legacy.UnsupportedVerifiedCore


def _modified_user_locals(function, blocks, body_target, header):
    storage = {int(local["id"]): str(local["storage"]) for local in function["locals"]}
    modified = set(); stack = [body_target]; seen = set()
    while stack:
        block_id = stack.pop()
        if block_id == header or block_id in seen: continue
        seen.add(block_id); block = blocks[block_id]
        for instruction in block["instructions"]:
            local_id = int(instruction["dest_local_id"])
            if storage.get(local_id) != "temporary": modified.add(local_id)
        term = block["terminator"]
        if term["kind"] == "goto":
            target = int(term["target"])
            if target != header: stack.append(target)
        elif term["kind"] == "branch":
            for target in (int(term["then_target"]), int(term["else_target"])):
                if target != header: stack.append(target)
    return modified


def _function_vcs(function, functions, sorts, function_index):
    headers = [block for block in function["body"]["blocks"] if "loop" in block]
    if not headers: return legacy._function_vcs(function, functions, sorts, function_index)
    if len(headers) != 1: raise UnsupportedVerifiedCore(f"$.functions[{function_index}].body: phase 4 supports one annotated while per function")
    path = f"$.functions[{function_index}]"; prefix = legacy._symbol(f"f{function['id']}_{function['name']}"); lines = []
    initial_env = {}; parameter_names = {parameter["name"] for parameter in function["signature"]["parameters"]}
    for local in function["locals"]:
        if local["storage"] == "parameter" and local["name"] in parameter_names:
            symbol = legacy._symbol(f"{prefix}_arg_{local['id']}_{local['name']}")
            initial_env[int(local["id"])] = symbol; lines.append(f"(declare-const {symbol} {sorts[int(local['type_id'])]})")
    placeholder = legacy._symbol(f"{prefix}_result")
    requires = [legacy._contract_expression(contract["expression"], initial_env, sorts, placeholder, f"{path}.contracts[{index}].expression") for index, contract in enumerate(function["contracts"]) if contract["kind"] == "requires"]
    ensures = [(index, contract) for index, contract in enumerate(function["contracts"]) if contract["kind"] == "ensures"]
    if not ensures: raise UnsupportedVerifiedCore(f"{path}.contracts: at least one ensures is required")
    blocks = {int(block["id"]): block for block in function["body"]["blocks"]}; header_block = headers[0]; header = int(header_block["id"]); header_term = header_block["terminator"]
    body_target = int(header_term["then_target"]); exit_target = int(header_term["else_target"]); call_counter = [0]

    def execute(block, env, facts, assumptions, label):
        local_env = dict(env); local_facts = list(facts)
        for instruction_index, instruction in enumerate(block["instructions"]):
            instruction_path = f"{label}.instructions[{instruction_index}]"; current_assumptions = assumptions + local_facts
            if instruction["op"] == "call":
                callee = functions[int(instruction["callee_id"])]
                args = [legacy._typed_value(value, local_env, sorts, f"{instruction_path}.arguments[{index}]") for index, value in enumerate(instruction["arguments"])]
                result_symbol = legacy._symbol(f"{prefix}_b{block['id']}_i{instruction_index}_call{call_counter[0]}"); call_counter[0] += 1
                lines.append(f"(declare-const {result_symbol} {sorts[int(instruction['type_id'])]})"); callee_env = legacy._callee_env(callee, args)
                for contract_index, contract in enumerate(callee["contracts"]):
                    if contract["kind"] == "requires":
                        claim = legacy._contract_expression(contract["expression"], callee_env, sorts, result_symbol, f"{instruction_path}.callee.requires[{contract_index}]")
                        legacy._obligation(lines, current_assumptions, claim, f"{prefix}.{label}.i{instruction_index}.callee-requires[{contract_index}]")
                    elif contract["kind"] == "ensures":
                        local_facts.append(legacy._contract_expression(contract["expression"], callee_env, sorts, result_symbol, f"{instruction_path}.callee.ensures[{contract_index}]"))
                local_env[int(instruction["dest_local_id"])] = result_symbol; continue
            term, side_conditions = legacy._instruction_term(instruction, local_env, sorts, instruction_path)
            for side_index, side_condition in enumerate(side_conditions): legacy._obligation(lines, current_assumptions, side_condition, f"{prefix}.{label}.i{instruction_index}.side[{side_index}]")
            local_env[int(instruction["dest_local_id"])] = term
        return local_env, local_facts

    def visit_exit(block_id, env, assumptions, facts, trace):
        if block_id == header or block_id in trace: raise UnsupportedVerifiedCore(f"{path}.body: loop exit path re-enters cycle")
        block = blocks[block_id]; local_env, local_facts = execute(block, env, facts, assumptions, f"exit-b{block_id}"); term = block["terminator"]; current = assumptions + local_facts
        if term["kind"] == "return":
            if "value" not in term: raise UnsupportedVerifiedCore(f"{path}.body.blocks[id={block_id}]: value return required")
            returned = legacy._typed_value(term["value"], local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.value")
            for contract_index, contract in ensures:
                claim = legacy._contract_expression(contract["expression"], local_env, sorts, returned, f"{path}.contracts[{contract_index}].expression")
                legacy._obligation(lines, current, claim, f"{prefix}.loop-exit.ensures[{contract_index}]")
            return
        if term["kind"] == "goto":
            target = int(term["target"]); target_block = blocks[target]; next_env = dict(local_env)
            values = [legacy._typed_value(value, local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.arguments[{index}]") for index, value in enumerate(term["arguments"])]
            for parameter, rendered in zip(target_block["parameters"], values): next_env[int(parameter["local_id"])] = rendered
            visit_exit(target, next_env, assumptions, local_facts, trace + (block_id,)); return
        if term["kind"] == "branch":
            condition = legacy._typed_value(term["condition"], local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.condition")
            for side, cond in (("then", condition), ("else", f"(not {condition})")):
                target = int(term[f"{side}_target"]); target_block = blocks[target]; next_env = dict(local_env)
                values = [legacy._typed_value(value, local_env, sorts, f"{path}.body.blocks[id={block_id}].terminator.{side}_arguments[{index}]") for index, value in enumerate(term[f"{side}_arguments"])]
                for parameter, rendered in zip(target_block["parameters"], values): next_env[int(parameter["local_id"])] = rendered
                visit_exit(target, next_env, assumptions + [cond], local_facts, trace + (block_id,))
            return
        raise UnsupportedVerifiedCore(f"{path}.body.blocks[id={block_id}]: unsupported exit terminator")

    def enter_loop(env, entry_assumptions, entry_facts):
        metadata = header_block["loop"]
        init_invariants = [legacy._contract_expression(expression, env, sorts, placeholder, f"{path}.body.blocks[id={header}].loop.invariants[{index}]") for index, expression in enumerate(metadata["invariants"])]
        for index, claim in enumerate(init_invariants): legacy._obligation(lines, entry_assumptions + entry_facts, claim, f"{prefix}.loop-init[{index}]")
        modified = _modified_user_locals(function, blocks, body_target, header); loop_env = dict(env); local_types = {int(local["id"]): int(local["type_id"]) for local in function["locals"]}
        for local_id in sorted(modified):
            symbol = legacy._symbol(f"{prefix}_loop_h{header}_local{local_id}"); lines.append(f"(declare-const {symbol} {sorts[local_types[local_id]]})"); loop_env[local_id] = symbol
        invariants = [legacy._contract_expression(expression, loop_env, sorts, placeholder, f"{path}.body.blocks[id={header}].loop.invariants[{index}]") for index, expression in enumerate(metadata["invariants"])]
        old_variant = legacy._contract_expression(metadata["decreases"], loop_env, sorts, placeholder, f"{path}.body.blocks[id={header}].loop.decreases")
        header_env, header_facts = execute(header_block, loop_env, [], invariants, f"loop-h{header}")
        condition = legacy._typed_value(header_term["condition"], header_env, sorts, f"{path}.body.blocks[id={header}].terminator.condition")
        legacy._obligation(lines, invariants + [condition] + header_facts, f"(>= {old_variant} 0)", f"{prefix}.loop-decreases-nonnegative")
        body_block = blocks[body_target]; body_env, body_facts = execute(body_block, header_env, header_facts, invariants + [condition], f"loop-body-b{body_target}"); body_term = body_block["terminator"]
        if body_term["kind"] != "goto" or int(body_term["target"]) != header: raise UnsupportedVerifiedCore(f"{path}.body: phase 4 SMT lowering requires one straight-line body backedge")
        next_env = dict(body_env); values = [legacy._typed_value(value, body_env, sorts, f"{path}.body.blocks[id={body_target}].terminator.arguments[{index}]") for index, value in enumerate(body_term["arguments"])]
        for parameter, rendered in zip(header_block["parameters"], values): next_env[int(parameter["local_id"])] = rendered
        preserve_assumptions = invariants + [condition] + body_facts
        for index, expression in enumerate(metadata["invariants"]):
            claim = legacy._contract_expression(expression, next_env, sorts, placeholder, f"{path}.body.blocks[id={header}].loop.invariants[{index}]")
            legacy._obligation(lines, preserve_assumptions, claim, f"{prefix}.loop-preserve[{index}]")
        new_variant = legacy._contract_expression(metadata["decreases"], next_env, sorts, placeholder, f"{path}.body.blocks[id={header}].loop.decreases")
        legacy._obligation(lines, preserve_assumptions, f"(< {new_variant} {old_variant})", f"{prefix}.loop-decreases-strict")
        visit_exit(exit_target, header_env, invariants + [f"(not {condition})"], header_facts, ())

    current = int(function["body"]["entry_block"]); env = dict(initial_env); facts = []; trace = set()
    while current != header:
        if current in trace: raise UnsupportedVerifiedCore(f"{path}.body: cycle before annotated loop")
        trace.add(current); block = blocks[current]; env, facts = execute(block, env, facts, requires, f"pre-b{current}"); term = block["terminator"]
        if term["kind"] != "goto": raise UnsupportedVerifiedCore(f"{path}.body: phase 4 requires straight-line preheader")
        target = int(term["target"]); target_block = blocks[target]; values = [legacy._typed_value(value, env, sorts, f"{path}.body.blocks[id={current}].terminator.arguments[{index}]") for index, value in enumerate(term["arguments"])]
        for parameter, rendered in zip(target_block["parameters"], values): env[int(parameter["local_id"])] = rendered
        current = target
    enter_loop(env, requires, facts); return lines


def generate_smtlib(value: Any) -> str:
    document = validate_typed_document(value)
    if not has_loops(document): return legacy.generate_smtlib(document)
    profile = document["target_profile"]
    if profile["integer_model"] != "mathematical" or profile["overflow"] != "checked" or profile["floating_point"] != "unsupported": raise UnsupportedVerifiedCore("$.target_profile: phase 4 requires mathematical/checked/no-float profile")
    kinds = {int(entry["id"]): str(entry["kind"]) for entry in document["types"]}; sorts = {type_id: legacy._sort(kind, f"$.types[id={type_id}]") for type_id, kind in kinds.items() if kind != "unit"}; functions = {int(function["id"]): function for function in document["functions"]}
    lines = ["(set-logic QF_NIA)", "; generated from arukellt-verified-core v1 proof phase 4"]
    for function_index, function in enumerate(document["functions"]): lines.extend(_function_vcs(function, functions, sorts, function_index))
    if not any(line == "(check-sat)" for line in lines): raise UnsupportedVerifiedCore("$.functions: no proof obligations generated")
    lines.append("(exit)"); return "\n".join(lines) + "\n"

__all__ = ["UnsupportedVerifiedCore", "generate_smtlib"]
