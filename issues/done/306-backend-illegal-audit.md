---
Status: done
Created: 2026-03-31
ID: 306
Track: corehir
Depends on: 281, 282, 283
Priority: 4
Orchestration class: implementation-ready
---

- 214/214 `run: "` fixtures compile with `--mir-select corehir` (0 failures)"
`is_backend_legal_operand` correctly rejects exactly 3 operands: IfExpr, LoopExpr, TryExpr.
- `crates/ark-mir/src/mir.rs: 419-424`
# Backend-illegal operand の棚卸し

## Summary

IfExpr / LoopExpr / TryExpr の 3 ノードは既知だが、CoreHIR path をデフォルトに昇格する前に、他に backend-illegal として残るノードがないか網羅的に監査する必要がある。昇格後に未発見ノードで壊れるリスクを潰す。

## Audit Results

### HIR ExprKind → MIR Operand mapping

| HIR ExprKind | MIR Operand | Backend Legal? | Notes |
|---|---|---|---|
| Const | ConstI32/I64/F32/F64/Bool/Char/String/U8/U16/U32/U64/I8/I16/Unit | ✅ | Direct constants |
| Local | Place | ✅ | Variable access |
| Global / QualifiedGlobal | Place / Call | ✅ | Name resolution |
| Call | Call / CallIndirect | ✅ | Function calls |
| BuiltinBinary | BinOp | ✅ | Binary operators |
| BuiltinUnary | UnaryOp | ✅ | Unary operators |
| **If** | **IfExpr** | **❌** | Backend-illegal (expression position) |
| Match | Switch/If chain | ✅ | Desugared to control flow |
| Block | inline stmts | ✅ | Body inlined |
| Tuple | ArrayInit/Aggregate | ✅ | Struct-like lowering |
| Array | ArrayInit | ✅ | Array construction |
| ArrayRepeat | ArrayInit | ✅ | Repeated elements |
| StructInit | StructInit | ✅ | Struct construction |
| FieldAccess | FieldAccess | ✅ | Field access |
| Index | IndexAccess | ✅ | Array indexing |
| Return | Return stmt | ✅ | Control flow |
| Break | Break stmt | ✅ | Loop control |
| Continue | Continue stmt | ✅ | Loop control |
| **Try** | **TryExpr** | **❌** | Backend-illegal (early return) |
| Assign | Assign stmt | ✅ | Assignment |
| **Loop** | **LoopExpr** | **❌** | Backend-illegal (expression position) |
| Closure | FnRef + captures | ✅ | Lowered to function ref |
| StringConcatMany | Call chain | ✅ | Lowered to concat calls |

### Fixture validation

- 214/214 `run:` fixtures compile with `--mir-select corehir` (0 failures)
- 334/337 `module-run/t3-run/t3-compile` fixtures compile with corehir (3 failures)
- 3 failures are W0004 backend validation (GC ref type mismatch), NOT backend-illegal operand issues

### Conclusion

`is_backend_legal_operand` correctly rejects exactly 3 operands: IfExpr, LoopExpr, TryExpr.
No additional backend-illegal nodes were found. These 3 correspond to #281, #282, #283 respectively.

## Acceptance

- [x] CoreHIR の全 HIR ノード種 (`crates/ark-hir/src/hir.rs`) を列挙し、MIR Operand への変換状態を表にする
- [x] `is_backend_legal_operand` が reject すべきノードが 3 種のみであることを確認、または追加すべきノードを特定
- [x] 全 fixture を `--mir-select corehir` で compile し、backend-illegal エラーの種類を収集
- [x] 結果を #284 の前提として記録

## References

- `crates/ark-hir/src/hir.rs`
- `crates/ark-mir/src/mir.rs:419-424`
- `crates/ark-mir/src/lower/mod.rs`