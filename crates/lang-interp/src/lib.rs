mod json_bridge;

pub use json_bridge::{JsonBridgeError, value_from_json, value_to_json, values_from_json_str};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lang_core::Pattern;
use lang_ir::{HighExpr, HighExprKind, HighFunction, HighMatchArm, HighModule};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Unit,
    Int(i64),
    Bool(bool),
    String(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Variant {
        name: String,
        fields: Vec<Value>,
    },
    Function(String),
    Closure {
        param: String,
        body: HighExpr,
        env: HashMap<String, Value>,
    },
    IterUnfold {
        state: Box<Value>,
        func: Box<Value>,
    },
    Error,
}

#[derive(Debug, Error)]
pub enum InterpreterError {
    #[error("unknown function `{0}`")]
    UnknownFunction(String),
    #[error("arity mismatch for `{0}`")]
    ArityMismatch(String),
    #[error("type mismatch: {0}")]
    TypeMismatch(&'static str),
    #[error("non-exhaustive match")]
    NonExhaustiveMatch,
    #[error("io error: {0}")]
    Io(String),
}

pub struct Interpreter {
    module: HighModule,
    last_trace: Vec<String>,
    output: String,
    base_dir: Option<PathBuf>,
    stdin: String,
}

impl Interpreter {
    #[must_use]
    pub fn new(module: &HighModule) -> Self {
        Self::with_base_dir(module, None)
    }

    #[must_use]
    pub fn with_base_dir(module: &HighModule, base_dir: Option<PathBuf>) -> Self {
        Self {
            module: module.clone(),
            last_trace: Vec::new(),
            output: String::new(),
            base_dir,
            stdin: String::new(),
        }
    }

    #[must_use]
    pub fn with_io(
        module: &HighModule,
        base_dir: Option<PathBuf>,
        stdin: impl Into<String>,
    ) -> Self {
        Self {
            module: module.clone(),
            last_trace: Vec::new(),
            output: String::new(),
            base_dir,
            stdin: stdin.into(),
        }
    }

    pub fn set_stdin_text(&mut self, stdin: impl Into<String>) {
        self.stdin = stdin.into();
    }

    pub fn call_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        self.last_trace.clear();
        self.output.clear();
        let function = self
            .module
            .functions
            .iter()
            .find(|function| function.name == name)
            .cloned()
            .ok_or_else(|| InterpreterError::UnknownFunction(name.to_owned()))?;
        self.eval_function(&function, args)
    }

    #[must_use]
    pub fn last_trace(&self) -> &[String] {
        &self.last_trace
    }

    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }

    fn eval_function(
        &mut self,
        function: &HighFunction,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        if function.params.len() != args.len() {
            return Err(InterpreterError::ArityMismatch(function.name.clone()));
        }
        let env = function
            .params
            .iter()
            .zip(args)
            .map(|(param, value)| (param.name.clone(), value))
            .collect::<HashMap<_, _>>();
        self.last_trace.push(format!("call {}", function.name));
        self.eval_expr(&function.body, &env)
    }

    fn eval_expr(
        &mut self,
        expr: &HighExpr,
        env: &HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        match &expr.kind {
            HighExprKind::Int(value) => Ok(Value::Int(*value)),
            HighExprKind::Bool(value) => Ok(Value::Bool(*value)),
            HighExprKind::String(value) => Ok(Value::String(value.clone())),
            HighExprKind::Ident(name) => Ok(env
                .get(name)
                .cloned()
                .or_else(|| {
                    if self.has_named_function(name) || is_builtin(name) {
                        Some(Value::Function(name.clone()))
                    } else {
                        None
                    }
                })
                .unwrap_or(Value::Error)),
            HighExprKind::List(items) => Ok(Value::List(
                items
                    .iter()
                    .map(|item| self.eval_expr(item, env))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            HighExprKind::Tuple(items) => Ok(Value::Tuple(
                items
                    .iter()
                    .map(|item| self.eval_expr(item, env))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            HighExprKind::Lambda { param, body } => Ok(Value::Closure {
                param: param.clone(),
                body: (**body).clone(),
                env: env.clone(),
            }),
            HighExprKind::Let { name, value, body } => {
                let value = self.eval_expr(value, env)?;
                let mut scoped = env.clone();
                scoped.insert(name.clone(), value);
                self.eval_expr(body, &scoped)
            }
            HighExprKind::Binary { op, left, right } => {
                let left_value = self.eval_expr(left, env)?;
                let right_value = self.eval_expr(right, env)?;
                self.last_trace.push(format!("binary {:?}", op));
                match (op, left_value, right_value) {
                    (lang_core::BinaryOp::Add, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Int(left + right))
                    }
                    (lang_core::BinaryOp::Subtract, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Int(left - right))
                    }
                    (lang_core::BinaryOp::Multiply, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Int(left * right))
                    }
                    (lang_core::BinaryOp::Divide, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Int(left / right))
                    }
                    (lang_core::BinaryOp::Modulo, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Int(left % right))
                    }
                    (lang_core::BinaryOp::Greater, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Bool(left > right))
                    }
                    (lang_core::BinaryOp::Less, Value::Int(left), Value::Int(right)) => {
                        Ok(Value::Bool(left < right))
                    }
                    (lang_core::BinaryOp::Equal, left, right) => Ok(Value::Bool(left == right)),
                    (lang_core::BinaryOp::And, Value::Bool(left), Value::Bool(right)) => {
                        Ok(Value::Bool(left && right))
                    }
                    (lang_core::BinaryOp::Or, Value::Bool(left), Value::Bool(right)) => {
                        Ok(Value::Bool(left || right))
                    }
                    _ => Err(InterpreterError::TypeMismatch("binary operator operands")),
                }
            }
            HighExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = self.eval_expr(condition, env)?;
                self.last_trace.push("if".to_owned());
                match condition_value {
                    Value::Bool(true) => self.eval_expr(then_branch, env),
                    Value::Bool(false) => self.eval_expr(else_branch, env),
                    _ => Err(InterpreterError::TypeMismatch("if condition")),
                }
            }
            HighExprKind::Match { subject, arms } => {
                let subject_value = self.eval_expr(subject, env)?;
                self.last_trace.push("match".to_owned());
                self.eval_match_arms(&subject_value, arms, env)
            }
            HighExprKind::Construct { variant, args } => {
                let fields = args
                    .iter()
                    .map(|arg| self.eval_expr(arg, env))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Variant {
                    name: variant.clone(),
                    fields,
                })
            }
            HighExprKind::Call { callee, args } => {
                let values = args
                    .iter()
                    .map(|arg| self.eval_expr(arg, env))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_named(callee, values)
            }
            HighExprKind::Error => Ok(Value::Error),
        }
    }

    fn call_named(&mut self, callee: &str, args: Vec<Value>) -> Result<Value, InterpreterError> {
        if is_builtin(callee) {
            return self.call_builtin(callee, args);
        }
        let function = self
            .module
            .functions
            .iter()
            .find(|function| function.name == callee)
            .cloned()
            .ok_or_else(|| InterpreterError::UnknownFunction(callee.to_owned()))?;
        self.eval_function(&function, args)
    }

    fn call_callable(
        &mut self,
        callable: Value,
        args: Vec<Value>,
    ) -> Result<Value, InterpreterError> {
        match callable {
            Value::Function(name) => self.call_named(&name, args),
            Value::Closure { param, body, env } => {
                if args.len() != 1 {
                    return Err(InterpreterError::ArityMismatch("<lambda>".to_owned()));
                }
                let mut scoped = env;
                scoped.insert(param, args.into_iter().next().expect("lambda arg"));
                self.eval_expr(&body, &scoped)
            }
            other => Err(InterpreterError::TypeMismatch(match other {
                Value::Unit => "unit is not callable",
                Value::Int(_) => "int is not callable",
                Value::Bool(_) => "bool is not callable",
                Value::String(_) => "string is not callable",
                Value::List(_) => "list is not callable",
                Value::Tuple(_) => "tuple is not callable",
                Value::Variant { .. } => "variant is not callable",
                Value::IterUnfold { .. } => "iterator is not callable",
                Value::Error => "error is not callable",
                Value::Function(_) | Value::Closure { .. } => unreachable!(),
            })),
        }
    }

    fn call_builtin(&mut self, callee: &str, args: Vec<Value>) -> Result<Value, InterpreterError> {
        self.last_trace.push(format!("builtin {callee}"));
        match callee {
            "__apply" => {
                if args.is_empty() {
                    return Err(InterpreterError::ArityMismatch("__apply".to_owned()));
                }
                let mut args = args;
                let callable = args.remove(0);
                self.call_callable(callable, args)
            }
            "__index" => match args.as_slice() {
                [Value::Tuple(items), Value::Int(index)] => {
                    Ok(items.get(*index as usize).cloned().unwrap_or(Value::Error))
                }
                [Value::List(items), Value::Int(index)] => {
                    Ok(items.get(*index as usize).cloned().unwrap_or(Value::Error))
                }
                _ => Err(InterpreterError::TypeMismatch("index operation")),
            },
            "range_inclusive" => match args.as_slice() {
                [Value::Int(start), Value::Int(end)] => Ok(Value::List(
                    (*start..=*end).map(Value::Int).collect::<Vec<_>>(),
                )),
                _ => Err(InterpreterError::TypeMismatch("range bounds")),
            },
            "string" => match args.as_slice() {
                [value] => Ok(Value::String(render_value(value))),
                _ => Err(InterpreterError::ArityMismatch("string".to_owned())),
            },
            "map" => match args.as_slice() {
                [Value::List(items), callable] => {
                    let mut out = Vec::with_capacity(items.len());
                    for item in items {
                        out.push(self.call_callable(callable.clone(), vec![item.clone()])?);
                    }
                    Ok(Value::List(out))
                }
                _ => Err(InterpreterError::TypeMismatch("map(list, fn)")),
            },
            "filter" => match args.as_slice() {
                [Value::List(items), callable] => {
                    let mut out = Vec::new();
                    for item in items {
                        if matches!(
                            self.call_callable(callable.clone(), vec![item.clone()])?,
                            Value::Bool(true)
                        ) {
                            out.push(item.clone());
                        }
                    }
                    Ok(Value::List(out))
                }
                _ => Err(InterpreterError::TypeMismatch("filter(list, fn)")),
            },
            "sum" => match args.as_slice() {
                [Value::List(items)] => {
                    let mut sum = 0i64;
                    for item in items {
                        if let Value::Int(value) = item {
                            sum += value;
                        } else {
                            return Err(InterpreterError::TypeMismatch("sum(list<int>)"));
                        }
                    }
                    Ok(Value::Int(sum))
                }
                _ => Err(InterpreterError::TypeMismatch("sum(list<int>)")),
            },
            "join" => match args.as_slice() {
                [Value::List(items), Value::String(separator)] => Ok(Value::String(
                    items
                        .iter()
                        .map(render_value)
                        .collect::<Vec<_>>()
                        .join(separator),
                )),
                _ => Err(InterpreterError::TypeMismatch("join(list<string>, sep)")),
            },
            "take" => match args.as_slice() {
                [Value::IterUnfold { state, func }, Value::Int(limit)] => {
                    let mut items = Vec::new();
                    let mut state = (**state).clone();
                    let callable = (**func).clone();
                    for _ in 0..*limit {
                        match self.call_callable(callable.clone(), vec![state.clone()])? {
                            Value::Variant { name, fields }
                                if name == "Next" && fields.len() == 2 =>
                            {
                                items.push(fields[0].clone());
                                state = fields[1].clone();
                            }
                            Value::Variant { name, .. } if name == "Done" => break,
                            _ => return Err(InterpreterError::TypeMismatch("iter.unfold step")),
                        }
                    }
                    Ok(Value::List(items))
                }
                [Value::List(items), Value::Int(limit)] => Ok(Value::List(
                    items.iter().take(*limit as usize).cloned().collect(),
                )),
                _ => Err(InterpreterError::TypeMismatch("take(iter, n)")),
            },
            "iter.unfold" => match args.as_slice() {
                [seed, callable] => Ok(Value::IterUnfold {
                    state: Box::new(seed.clone()),
                    func: Box::new(callable.clone()),
                }),
                _ => Err(InterpreterError::ArityMismatch("iter.unfold".to_owned())),
            },
            "console.println" => match args.as_slice() {
                [value] => {
                    let rendered = render_value(value);
                    self.output.push_str(&rendered);
                    if !rendered.ends_with('\n') {
                        self.output.push('\n');
                    }
                    Ok(Value::Unit)
                }
                _ => Err(InterpreterError::ArityMismatch(
                    "console.println".to_owned(),
                )),
            },
            "fs.read_text" => match args.as_slice() {
                [Value::String(path)] => self.read_text(path),
                _ => Err(InterpreterError::TypeMismatch("fs.read_text(path)")),
            },
            "stdin.read_text" => match args.as_slice() {
                [] => Ok(Value::String(self.stdin.clone())),
                _ => Err(InterpreterError::ArityMismatch(
                    "stdin.read_text".to_owned(),
                )),
            },
            "Next" => Ok(Value::Variant {
                name: "Next".to_owned(),
                fields: args,
            }),
            "Done" => Ok(Value::Variant {
                name: "Done".to_owned(),
                fields: args,
            }),
            other => Err(InterpreterError::UnknownFunction(other.to_owned())),
        }
    }

    fn read_text(&self, path: &str) -> Result<Value, InterpreterError> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else if let Some(base_dir) = &self.base_dir {
            base_dir.join(path)
        } else {
            PathBuf::from(path)
        };

        match std::fs::read_to_string(&candidate) {
            Ok(text) => Ok(Value::Variant {
                name: "Ok".to_owned(),
                fields: vec![Value::String(text)],
            }),
            Err(error) => {
                let error_variant = if error.kind() == std::io::ErrorKind::NotFound {
                    "FileNotFound"
                } else if error.kind() == std::io::ErrorKind::PermissionDenied {
                    "PermissionDenied"
                } else {
                    "UnknownReadError"
                };
                Ok(Value::Variant {
                    name: "Err".to_owned(),
                    fields: vec![Value::Variant {
                        name: error_variant.to_owned(),
                        fields: Vec::new(),
                    }],
                })
            }
        }
    }

    fn eval_match_arms(
        &mut self,
        subject: &Value,
        arms: &[HighMatchArm],
        env: &HashMap<String, Value>,
    ) -> Result<Value, InterpreterError> {
        for arm in arms {
            if let Some(bindings) = self.match_pattern(subject, &arm.pattern) {
                let mut scoped_env = env.clone();
                for (name, value) in bindings {
                    scoped_env.insert(name, value);
                }
                return self.eval_expr(&arm.expr, &scoped_env);
            }
        }
        Err(InterpreterError::NonExhaustiveMatch)
    }

    fn match_pattern(&self, subject: &Value, pattern: &Pattern) -> Option<HashMap<String, Value>> {
        match pattern {
            Pattern::Wildcard => Some(HashMap::new()),
            Pattern::Variant { name, bindings } => match subject {
                Value::Variant {
                    name: actual,
                    fields,
                } if actual == name => {
                    let mut env = HashMap::new();
                    for (binding, value) in bindings.iter().zip(fields.iter()) {
                        env.insert(binding.clone(), value.clone());
                    }
                    Some(env)
                }
                _ => None,
            },
        }
    }

    fn has_named_function(&self, name: &str) -> bool {
        self.module
            .functions
            .iter()
            .any(|function| function.name == name)
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "__apply"
            | "__index"
            | "range_inclusive"
            | "string"
            | "map"
            | "filter"
            | "sum"
            | "join"
            | "take"
            | "iter.unfold"
            | "console.println"
            | "fs.read_text"
            | "stdin.read_text"
            | "Next"
            | "Done"
    )
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Unit => String::new(),
        Value::Int(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::String(text) => text.clone(),
        Value::List(items) => {
            let rendered = items
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rendered}]")
        }
        Value::Tuple(items) => {
            let rendered = items
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
        Value::Variant { name, fields } => {
            if fields.is_empty() {
                name.clone()
            } else {
                let rendered = fields
                    .iter()
                    .map(render_value)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({rendered})")
            }
        }
        Value::Function(name) => format!("<fn {name}>"),
        Value::Closure { .. } => "<lambda>".to_owned(),
        Value::IterUnfold { .. } => "<iter>".to_owned(),
        Value::Error => "<error>".to_owned(),
    }
}
