use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lang_core::{
    Diagnostic, DiagnosticLevel, Type, TypedFunction, TypedModule, compile_module,
    compile_module_partial,
};
use lang_interp::{Interpreter, Value};
use lang_ir::lower_to_high_ir;

const EVAL_FUNCTION: &str = "__repl_eval__";

pub fn run_stdio() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_repl(stdin.lock(), stdout.lock())
}

pub fn run_repl<R: BufRead, W: Write>(mut input: R, mut output: W) -> Result<()> {
    let mut session = ReplSession::new();
    let mut line = String::new();

    loop {
        write!(output, "{}", session.prompt())?;
        output.flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            session.flush_pending(&mut output)?;
            break;
        }

        let trimmed = line.trim_end_matches(['\n', '\r']);
        if session.handle_line(trimmed, &mut output)? {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
pub struct ReplSession {
    loaded_path: Option<PathBuf>,
    loaded_source: Option<String>,
    imports: Vec<String>,
    type_decls: Vec<String>,
    function_blocks: Vec<String>,
    let_bindings: Vec<String>,
    pending_block: Vec<String>,
}

impl ReplSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn prompt(&self) -> &'static str {
        if self.pending_block.is_empty() {
            "arkli> "
        } else {
            "...> "
        }
    }

    pub fn handle_line<W: Write>(&mut self, line: &str, output: &mut W) -> Result<bool> {
        let trimmed = line.trim();
        if trimmed.starts_with(':') {
            self.flush_pending(output)?;
            return self.handle_command(trimmed, output);
        }

        if trimmed.is_empty() {
            self.flush_pending(output)?;
            return Ok(false);
        }

        if !self.pending_block.is_empty() || starts_multiline_block(trimmed) {
            self.pending_block.push(line.to_owned());
            return Ok(false);
        }

        if trimmed.starts_with("let ") {
            self.add_let_binding(trimmed)?;
            return Ok(false);
        }

        self.eval_and_print(trimmed, output)
    }

    pub fn flush_pending<W: Write>(&mut self, output: &mut W) -> Result<()> {
        if self.pending_block.is_empty() {
            return Ok(());
        }
        let block = self.pending_block.join("\n");
        self.pending_block.clear();
        self.add_definition_block(&block, output)
    }

    fn handle_command<W: Write>(&mut self, command: &str, output: &mut W) -> Result<bool> {
        let (name, rest) = command
            .split_once(' ')
            .map_or((command, ""), |(name, rest)| (name, rest.trim()));
        match name {
            ":quit" | ":q" => Ok(true),
            ":load" => {
                if rest.is_empty() {
                    writeln!(output, "usage: :load <file>")?;
                } else {
                    self.load_file(Path::new(rest))?;
                    writeln!(output, "loaded {}", rest)?;
                }
                Ok(false)
            }
            ":reload" => {
                let Some(path) = self.loaded_path.clone() else {
                    writeln!(output, "no file loaded")?;
                    return Ok(false);
                };
                self.load_file(&path)?;
                writeln!(output, "reloaded {}", path.display())?;
                Ok(false)
            }
            ":type" => {
                if rest.is_empty() {
                    writeln!(output, "usage: :type <expr>")?;
                } else {
                    let ty = self.type_of(rest)?;
                    writeln!(output, "{ty}")?;
                }
                Ok(false)
            }
            other => {
                writeln!(output, "unknown command: {other}")?;
                Ok(false)
            }
        }
    }

    fn load_file(&mut self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        validate_source(&source)?;
        self.loaded_path = Some(path.to_path_buf());
        self.loaded_source = Some(source);
        Ok(())
    }

    fn add_definition_block<W: Write>(&mut self, block: &str, output: &mut W) -> Result<()> {
        validate_source(&self.compose_source_with_extra(block))?;
        match classify_block(block) {
            BlockKind::Import => self.imports.push(block.to_owned()),
            BlockKind::Type => self.type_decls.push(block.to_owned()),
            BlockKind::Function => self.function_blocks.push(block.to_owned()),
        }
        writeln!(output, "defined")?;
        Ok(())
    }

    fn add_let_binding(&mut self, line: &str) -> Result<()> {
        let mut trial = self.let_bindings.clone();
        trial.push(line.to_owned());
        let source = self.compose_eval_module(&trial, "0");
        let _ = self.compile_eval_module(&source)?;
        self.let_bindings = trial;
        Ok(())
    }

    fn eval_and_print<W: Write>(&self, expr: &str, output: &mut W) -> Result<bool> {
        let value = self.eval_expr(expr)?;
        let rendered = render_value(&value.value, &value.output);
        if !rendered.is_empty() {
            write!(output, "{rendered}")?;
        }
        Ok(false)
    }

    fn eval_expr(&self, expr: &str) -> Result<EvalOutcome> {
        let source = self.compose_eval_module(&self.let_bindings, expr);
        let typed = self.compile_eval_module(&source)?;
        let high = lower_to_high_ir(&typed);
        let base_dir = self
            .loaded_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf);
        let mut interpreter = Interpreter::with_base_dir(&high, base_dir);
        let value = interpreter.call_function(EVAL_FUNCTION, Vec::new())?;
        Ok(EvalOutcome {
            value,
            output: interpreter.output().to_owned(),
        })
    }

    fn type_of(&self, expr: &str) -> Result<Type> {
        let source = self.compose_eval_module(&self.let_bindings, expr);
        let typed = self.compile_eval_module(&source)?;
        self.eval_function(&typed)
            .map(|function| function.body.ty.clone())
            .context("repl eval function missing after compilation")
    }

    fn compose_eval_module(&self, let_bindings: &[String], expr: &str) -> String {
        let mut source = self.compose_base_source();
        if !source.is_empty() {
            source.push('\n');
        }
        source.push_str(&format!("fn {EVAL_FUNCTION}():\n"));
        for binding in let_bindings {
            source.push_str("  ");
            source.push_str(binding);
            source.push('\n');
        }
        for line in expr.lines() {
            source.push_str("  ");
            source.push_str(line);
            source.push('\n');
        }
        source
    }

    fn compose_source_with_extra(&self, extra: &str) -> String {
        let mut source = self.compose_base_source();
        if !source.is_empty() && !extra.is_empty() {
            source.push('\n');
        }
        source.push_str(extra);
        source
    }

    fn compose_base_source(&self) -> String {
        let mut sections = Vec::new();
        if let Some(source) = &self.loaded_source {
            sections.push(source.trim_end().to_owned());
        }
        sections.extend(self.imports.iter().cloned());
        sections.extend(self.type_decls.iter().cloned());
        sections.extend(self.function_blocks.iter().cloned());
        sections
            .into_iter()
            .filter(|section| !section.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn compile_eval_module(&self, source: &str) -> Result<TypedModule> {
        let result = compile_module_partial(source);
        let typed = result
            .module
            .context("typed module missing after compilation")?;
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.level == DiagnosticLevel::Error && diagnostic.code != "E_RETURN_MISMATCH"
            })
            .cloned()
            .collect::<Vec<_>>();
        if !diagnostics.is_empty() {
            bail!(format_diagnostics(&diagnostics));
        }
        self.eval_function(&typed)
            .context("repl eval function missing after compilation")?;
        Ok(typed)
    }

    fn eval_function<'a>(&self, typed: &'a TypedModule) -> Option<&'a TypedFunction> {
        typed
            .functions
            .iter()
            .find(|function| function.name == EVAL_FUNCTION)
    }
}

struct EvalOutcome {
    value: Value,
    output: String,
}

fn validate_source(source: &str) -> Result<()> {
    let result = compile_module(source);
    if result.error_count() > 0 {
        bail!(format_diagnostics(&result.diagnostics));
    }
    Ok(())
}

fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "[{:?}] {} {}: {}",
                diagnostic.stage, diagnostic.code, diagnostic.message, diagnostic.suggested_fix
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn starts_multiline_block(line: &str) -> bool {
    line.starts_with("fn ") || line.starts_with("type ") || line.starts_with("import ")
}

fn classify_block(block: &str) -> BlockKind {
    let first = block
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    if first.trim_start().starts_with("import ") {
        BlockKind::Import
    } else if first.trim_start().starts_with("type ") {
        BlockKind::Type
    } else {
        BlockKind::Function
    }
}

enum BlockKind {
    Import,
    Type,
    Function,
}

fn render_value(value: &Value, captured_output: &str) -> String {
    if !captured_output.is_empty() {
        return captured_output.to_owned();
    }
    match value {
        Value::Unit => String::new(),
        Value::Int(number) => format!("{number}\n"),
        Value::Bool(flag) => format!("{flag}\n"),
        Value::String(text) => format!("{text}\n"),
        Value::List(items) => {
            let rendered = items
                .iter()
                .map(render_inline_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rendered}]\n")
        }
        Value::Tuple(items) => {
            let rendered = items
                .iter()
                .map(render_inline_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})\n")
        }
        Value::Variant { name, fields } => {
            if fields.is_empty() {
                format!("{name}\n")
            } else {
                let rendered = fields
                    .iter()
                    .map(render_inline_value)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({rendered})\n")
            }
        }
        Value::Function(name) => format!("<fn {name}>\n"),
        Value::Closure { .. } => "<lambda>\n".to_owned(),
        Value::IterUnfold { .. } => "<iter>\n".to_owned(),
        Value::Error => "<error>\n".to_owned(),
    }
}

fn render_inline_value(value: &Value) -> String {
    match value {
        Value::Unit => String::new(),
        Value::Int(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::String(text) => text.clone(),
        Value::List(items) => {
            let rendered = items
                .iter()
                .map(render_inline_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rendered}]")
        }
        Value::Tuple(items) => {
            let rendered = items
                .iter()
                .map(render_inline_value)
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
                    .map(render_inline_value)
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ReplSession, run_repl};

    #[test]
    fn evaluates_expressions_and_let_bindings() {
        let input = "let base = 40\nbase + 2\n:q\n";
        let mut output = Vec::new();

        run_repl(Cursor::new(input), &mut output).expect("repl run");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("42\n"), "unexpected output: {rendered}");
    }

    #[test]
    fn loads_files_and_supports_type_queries() {
        let path = temp_file_path("arkli-load");
        fs::write(
            &path,
            "\
fn double(value: Int) -> Int:
  value * 2
",
        )
        .expect("write repl fixture");
        let input = format!(
            ":load {}\n:type double(21)\ndouble(21)\n:q\n",
            path.display()
        );
        let mut output = Vec::new();

        run_repl(Cursor::new(input), &mut output).expect("repl run");

        let rendered = String::from_utf8(output).expect("utf8 output");
        assert!(rendered.contains("loaded"), "unexpected output: {rendered}");
        assert!(rendered.contains("Int"), "unexpected output: {rendered}");
        assert!(rendered.contains("42\n"), "unexpected output: {rendered}");
    }

    #[test]
    fn reloads_loaded_files() {
        let path = temp_file_path("arkli-reload");
        fs::write(
            &path,
            "\
fn value() -> Int:
  1
",
        )
        .expect("write initial fixture");

        let mut session = ReplSession::new();
        session.load_file(&path).expect("load initial file");
        let first = session.eval_expr("value()").expect("initial eval");
        assert_eq!(super::render_inline_value(&first.value), "1");

        fs::write(
            &path,
            "\
fn value() -> Int:
  2
",
        )
        .expect("write updated fixture");
        let mut output = Vec::new();
        assert!(
            !session
                .handle_line(":reload", &mut output)
                .expect("reload command")
        );
        let second = session.eval_expr("value()").expect("reloaded eval");
        assert_eq!(super::render_inline_value(&second.value), "2");
    }

    fn temp_file_path(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}.ar"))
    }
}
