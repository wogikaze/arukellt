mod high;
mod low;
mod lowering;
mod optimize;
mod wasm;

pub use high::{HighExpr, HighExprKind, HighFunction, HighMatchArm, HighModule, HighParam};
pub use low::{LowFunction, LowInstruction, LowModule};
pub use lowering::{lower_to_high_ir, lower_to_low_ir};
pub use optimize::optimize_high_module;
pub use wasm::{
    ParseOrZeroSpec, SuffixRecursionSpec, WasmFunction, WasmFunctionBody, WasmHelperUsage,
    WasmModule, lower_to_wasm_ir,
};
