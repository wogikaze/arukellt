#[derive(Clone, Debug, PartialEq)]
pub struct LowModule {
    pub functions: Vec<LowFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LowFunction {
    pub name: String,
    pub instructions: Vec<LowInstruction>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LowInstruction {
    Param(String),
    ConstInt(i64),
    ConstBool(bool),
    ConstString(String),
    Load(String),
    Add,
    Subtract,
    Multiply,
    Divide,
    Greater,
    Equal,
    IfStart,
    ElseStart,
    EndIf,
    Call { callee: String, argc: usize },
    Return,
    Error,
}

impl LowInstruction {
    #[must_use]
    pub fn op_name(&self) -> &'static str {
        match self {
            Self::Param(_) => "param",
            Self::ConstInt(_) => "const_int",
            Self::ConstBool(_) => "const_bool",
            Self::ConstString(_) => "const_string",
            Self::Load(_) => "load",
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Greater => "gt",
            Self::Equal => "eq",
            Self::IfStart => "if",
            Self::ElseStart => "else",
            Self::EndIf => "end_if",
            Self::Call { .. } => "call",
            Self::Return => "return",
            Self::Error => "error",
        }
    }
}
