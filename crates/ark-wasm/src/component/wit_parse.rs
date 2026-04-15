//! Minimal WIT text parser for Arukellt's Component Model support.
//!
//! Parses a subset of WIT sufficient for Arukellt import/export declarations:
//! - `package` header
//! - `interface` blocks with `func` declarations
//! - `record`, `enum`, `variant`, `resource` type declarations
//! - Primitive types, `list<T>`, `option<T>`, `result<T, E>`, `tuple<...>`
//!
//! Single-file WIT only. `use` (cross-interface references) is out of scope for v2.

use super::{WitEnum, WitFunction, WitRecord, WitType, WitVariant};

/// A parsed WIT interface.
#[derive(Debug, Clone)]
pub struct WitInterface {
    pub name: String,
    pub functions: Vec<WitFunction>,
    pub records: Vec<WitRecord>,
    pub enums: Vec<WitEnum>,
    pub variants: Vec<WitVariant>,
    pub resources: Vec<String>,
}

/// A parsed WIT document.
#[derive(Debug, Clone)]
pub struct WitDocument {
    pub package: Option<String>,
    pub interfaces: Vec<WitInterface>,
}

/// WIT parse error.
#[derive(Debug, Clone)]
pub struct WitParseError {
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl std::fmt::Display for WitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for WitParseError {}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn err(&self, msg: impl Into<String>) -> WitParseError {
        WitParseError {
            line: self.line,
            col: self.col,
            message: msg.into(),
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self, n: usize) {
        for ch in self.input[self.pos..self.pos + n].chars() {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        self.pos += n;
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            // Skip whitespace
            let before = self.pos;
            while let Some(ch) = self.peek_char() {
                if ch.is_whitespace() {
                    self.advance(1);
                } else {
                    break;
                }
            }
            // Skip line comments
            if self.remaining().starts_with("//") {
                while let Some(ch) = self.peek_char() {
                    self.advance(1);
                    if ch == '\n' {
                        break;
                    }
                }
                continue;
            }
            // Skip block comments
            if self.remaining().starts_with("/*") {
                self.advance(2);
                let mut depth = 1u32;
                while depth > 0 && !self.is_eof() {
                    if self.remaining().starts_with("/*") {
                        self.advance(2);
                        depth += 1;
                    } else if self.remaining().starts_with("*/") {
                        self.advance(2);
                        depth -= 1;
                    } else {
                        self.advance(1);
                    }
                }
                continue;
            }
            if self.pos == before {
                break;
            }
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), WitParseError> {
        self.skip_ws_and_comments();
        match self.peek_char() {
            Some(ch) if ch == expected => {
                self.advance(1);
                Ok(())
            }
            Some(ch) => Err(self.err(format!("expected '{}', found '{}'", expected, ch))),
            None => Err(self.err(format!("expected '{}', found EOF", expected))),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), WitParseError> {
        self.skip_ws_and_comments();
        if self.remaining().starts_with(kw) {
            let after = self.pos + kw.len();
            let next_ch = self
                .input
                .get(after..after + 1)
                .and_then(|s| s.chars().next());
            if next_ch.is_none() || !is_ident_char(next_ch.unwrap()) {
                self.advance(kw.len());
                return Ok(());
            }
        }
        Err(self.err(format!("expected keyword '{}'", kw)))
    }

    fn parse_ident(&mut self) -> Result<String, WitParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_ident_char(ch) || ch == '-' {
                self.advance(1);
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.err("expected identifier"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_package_path(&mut self) -> Result<String, WitParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if is_ident_char(ch) || ch == '-' || ch == ':' || ch == '/' || ch == '@' || ch == '.' {
                self.advance(1);
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.err("expected package path"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn try_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws_and_comments();
        if self.remaining().starts_with(kw) {
            let after = self.pos + kw.len();
            let next_ch = self
                .input
                .get(after..after + 1)
                .and_then(|s| s.chars().next());
            if next_ch.is_none() || !is_ident_char(next_ch.unwrap()) {
                self.advance(kw.len());
                return true;
            }
        }
        false
    }

    fn parse_wit_type(&mut self) -> Result<WitType, WitParseError> {
        self.skip_ws_and_comments();
        // Try primitive types first
        for (name, ty) in PRIMITIVE_TYPES {
            if self.try_keyword(name) {
                return Ok(ty.clone());
            }
        }

        // Container types
        if self.try_keyword("list") {
            self.expect_char('<')?;
            let inner = self.parse_wit_type()?;
            self.expect_char('>')?;
            return Ok(WitType::List(Box::new(inner)));
        }
        if self.try_keyword("option") {
            self.expect_char('<')?;
            let inner = self.parse_wit_type()?;
            self.expect_char('>')?;
            return Ok(WitType::Option(Box::new(inner)));
        }
        if self.try_keyword("result") {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('<') {
                self.advance(1);
                let ok = if self.peek_char() == Some('_') {
                    self.advance(1);
                    None
                } else {
                    Some(Box::new(self.parse_wit_type()?))
                };
                let err = if self.peek_char() == Some(',') {
                    self.advance(1);
                    self.skip_ws_and_comments();
                    if self.peek_char() == Some('_') {
                        self.advance(1);
                        None
                    } else {
                        Some(Box::new(self.parse_wit_type()?))
                    }
                } else {
                    None
                };
                self.expect_char('>')?;
                return Ok(WitType::Result { ok, err });
            }
            return Ok(WitType::Result {
                ok: None,
                err: None,
            });
        }
        if self.try_keyword("tuple") {
            self.expect_char('<')?;
            let mut elems = Vec::new();
            loop {
                self.skip_ws_and_comments();
                if self.peek_char() == Some('>') {
                    self.advance(1);
                    break;
                }
                if !elems.is_empty() {
                    self.expect_char(',')?;
                }
                elems.push(self.parse_wit_type()?);
            }
            return Ok(WitType::Tuple(elems));
        }
        if self.try_keyword("own") {
            self.expect_char('<')?;
            let inner = self.parse_wit_type()?;
            self.expect_char('>')?;
            return Ok(WitType::Own(Box::new(inner)));
        }
        if self.try_keyword("borrow") {
            self.expect_char('<')?;
            let inner = self.parse_wit_type()?;
            self.expect_char('>')?;
            return Ok(WitType::Borrow(Box::new(inner)));
        }
        if self.try_keyword("flags") {
            self.expect_char('{')?;
            let mut names = Vec::new();
            loop {
                self.skip_ws_and_comments();
                if self.peek_char() == Some('}') {
                    self.advance(1);
                    break;
                }
                names.push(self.parse_ident()?);
                self.skip_ws_and_comments();
                if self.peek_char() == Some(',') {
                    self.advance(1);
                }
            }
            return Ok(WitType::Flags(names));
        }

        // Named type reference (record, enum, variant, resource)
        let name = self.parse_ident()?;
        Ok(WitType::Record(name))
    }

    fn parse_func_decl(&mut self) -> Result<WitFunction, WitParseError> {
        let name = self.parse_ident()?;
        self.expect_char(':')?;
        self.expect_keyword("func")?;
        self.expect_char('(')?;
        let mut params = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some(')') {
                self.advance(1);
                break;
            }
            if !params.is_empty() {
                self.expect_char(',')?;
            }
            let pname = self.parse_ident()?;
            self.expect_char(':')?;
            let ptype = self.parse_wit_type()?;
            params.push((pname, ptype));
        }
        self.skip_ws_and_comments();
        let result = if self.remaining().starts_with("->") {
            self.advance(2);
            Some(self.parse_wit_type()?)
        } else {
            None
        };
        // Optional semicolon
        self.skip_ws_and_comments();
        if self.peek_char() == Some(';') {
            self.advance(1);
        }
        Ok(WitFunction {
            name,
            params,
            result,
        })
    }

    fn parse_record(&mut self) -> Result<WitRecord, WitParseError> {
        let name = self.parse_ident()?;
        self.expect_char('{')?;
        let mut fields = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') {
                self.advance(1);
                break;
            }
            let fname = self.parse_ident()?;
            self.expect_char(':')?;
            let ftype = self.parse_wit_type()?;
            fields.push((fname, ftype));
            self.skip_ws_and_comments();
            if self.peek_char() == Some(',') {
                self.advance(1);
            }
        }
        Ok(WitRecord { name, fields })
    }

    fn parse_enum(&mut self) -> Result<WitEnum, WitParseError> {
        let name = self.parse_ident()?;
        self.expect_char('{')?;
        let mut variants = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') {
                self.advance(1);
                break;
            }
            let vname = self.parse_ident()?;
            variants.push(vname);
            self.skip_ws_and_comments();
            if self.peek_char() == Some(',') {
                self.advance(1);
            }
        }
        Ok(WitEnum { name, variants })
    }

    fn parse_variant(&mut self) -> Result<WitVariant, WitParseError> {
        let name = self.parse_ident()?;
        self.expect_char('{')?;
        let mut cases = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') {
                self.advance(1);
                break;
            }
            let cname = self.parse_ident()?;
            self.skip_ws_and_comments();
            let payload = if self.peek_char() == Some('(') {
                self.advance(1);
                let ty = self.parse_wit_type()?;
                self.expect_char(')')?;
                Some(ty)
            } else {
                None
            };
            cases.push((cname, payload));
            self.skip_ws_and_comments();
            if self.peek_char() == Some(',') {
                self.advance(1);
            }
        }
        Ok(WitVariant { name, cases })
    }

    fn parse_resource(&mut self) -> Result<String, WitParseError> {
        let name = self.parse_ident()?;
        self.skip_ws_and_comments();
        if self.peek_char() == Some('{') {
            // Resource with methods — skip the block for v2
            self.advance(1);
            let mut depth = 1u32;
            while depth > 0 && !self.is_eof() {
                match self.peek_char() {
                    Some('{') => {
                        depth += 1;
                        self.advance(1);
                    }
                    Some('}') => {
                        depth -= 1;
                        self.advance(1);
                    }
                    _ => self.advance(1),
                }
            }
        } else if self.peek_char() == Some(';') {
            self.advance(1);
        }
        Ok(name)
    }

    fn parse_interface(&mut self) -> Result<WitInterface, WitParseError> {
        let name = self.parse_ident()?;
        self.expect_char('{')?;
        let mut iface = WitInterface {
            name,
            functions: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
            variants: Vec::new(),
            resources: Vec::new(),
        };
        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') {
                self.advance(1);
                break;
            }
            if self.is_eof() {
                return Err(self.err("unexpected EOF in interface block"));
            }
            if self.try_keyword("record") {
                iface.records.push(self.parse_record()?);
            } else if self.try_keyword("enum") {
                iface.enums.push(self.parse_enum()?);
            } else if self.try_keyword("variant") {
                iface.variants.push(self.parse_variant()?);
            } else if self.try_keyword("resource") {
                iface.resources.push(self.parse_resource()?);
            } else {
                // Must be a func declaration
                iface.functions.push(self.parse_func_decl()?);
            }
        }
        Ok(iface)
    }

    fn parse_document(&mut self) -> Result<WitDocument, WitParseError> {
        let mut doc = WitDocument {
            package: None,
            interfaces: Vec::new(),
        };
        loop {
            self.skip_ws_and_comments();
            if self.is_eof() {
                break;
            }
            if self.try_keyword("package") {
                let pkg = self.parse_package_path()?;
                self.expect_char(';')?;
                doc.package = Some(pkg);
            } else if self.try_keyword("interface") {
                doc.interfaces.push(self.parse_interface()?);
            } else if self.try_keyword("world") {
                // Skip world blocks — we only care about interfaces for imports
                let _name = self.parse_ident()?;
                self.expect_char('{')?;
                let mut depth = 1u32;
                while depth > 0 && !self.is_eof() {
                    match self.peek_char() {
                        Some('{') => {
                            depth += 1;
                            self.advance(1);
                        }
                        Some('}') => {
                            depth -= 1;
                            self.advance(1);
                        }
                        _ => self.advance(1),
                    }
                }
            } else {
                return Err(self.err(format!(
                    "expected 'package', 'interface', or 'world', found {:?}",
                    &self.remaining()[..self.remaining().len().min(20)]
                )));
            }
        }
        Ok(doc)
    }
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

const PRIMITIVE_TYPES: &[(&str, WitType)] = &[
    ("string", WitType::StringType),
    ("bool", WitType::Bool),
    ("char", WitType::Char),
    ("u8", WitType::U8),
    ("u16", WitType::U16),
    ("u32", WitType::U32),
    ("u64", WitType::U64),
    ("s8", WitType::S8),
    ("s16", WitType::S16),
    ("s32", WitType::S32),
    ("s64", WitType::S64),
    ("f32", WitType::F32),
    ("f64", WitType::F64),
];

/// Parse a WIT document from text.
pub fn parse_wit(input: &str) -> Result<WitDocument, WitParseError> {
    let mut parser = Parser::new(input);
    parser.parse_document()
}

/// Convert parsed WIT interface functions to MIR imports.
pub fn wit_interface_to_mir_imports(iface: &WitInterface) -> Vec<ark_mir::mir::MirImport> {
    iface
        .functions
        .iter()
        .map(|func| ark_mir::mir::MirImport {
            interface: iface.name.clone(),
            name: func.name.clone(),
            param_types: func.params.iter().map(|(_, ty)| ty.to_wit()).collect(),
            return_type: func.result.as_ref().map(|ty| ty.to_wit()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_interface() {
        let wit = r#"
            package test:host;

            interface host-fns {
                greet: func(name: string) -> string;
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.package.as_deref(), Some("test:host"));
        assert_eq!(doc.interfaces.len(), 1);
        assert_eq!(doc.interfaces[0].name, "host-fns");
        assert_eq!(doc.interfaces[0].functions.len(), 1);
        assert_eq!(doc.interfaces[0].functions[0].name, "greet");
        assert_eq!(doc.interfaces[0].functions[0].params.len(), 1);
        assert_eq!(doc.interfaces[0].functions[0].params[0].0, "name");
        assert_eq!(
            doc.interfaces[0].functions[0].result,
            Some(WitType::StringType)
        );
    }

    #[test]
    fn parse_multiple_functions() {
        let wit = r#"
            interface math {
                add: func(a: s32, b: s32) -> s32;
                negate: func(x: f64) -> f64;
                log: func(msg: string);
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].functions.len(), 3);
        assert_eq!(doc.interfaces[0].functions[2].name, "log");
        assert!(doc.interfaces[0].functions[2].result.is_none());
    }

    #[test]
    fn parse_record_type() {
        let wit = r#"
            interface shapes {
                record point {
                    x: f64,
                    y: f64,
                }
                distance: func(a: point, b: point) -> f64;
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].records.len(), 1);
        assert_eq!(doc.interfaces[0].records[0].name, "point");
        assert_eq!(doc.interfaces[0].records[0].fields.len(), 2);
    }

    #[test]
    fn parse_enum_type() {
        let wit = r#"
            interface colors {
                enum color {
                    red,
                    green,
                    blue,
                }
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].enums.len(), 1);
        assert_eq!(doc.interfaces[0].enums[0].variants.len(), 3);
    }

    #[test]
    fn parse_variant_type() {
        let wit = r#"
            interface values {
                variant value {
                    int-val(s32),
                    str-val(string),
                    none,
                }
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].variants.len(), 1);
        assert_eq!(doc.interfaces[0].variants[0].cases.len(), 3);
        assert!(doc.interfaces[0].variants[0].cases[2].1.is_none());
    }

    #[test]
    fn parse_container_types() {
        let wit = r#"
            interface api {
                get-items: func() -> list<s32>;
                find: func(id: s32) -> option<string>;
                try-parse: func(input: string) -> result<s32, string>;
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        let funcs = &doc.interfaces[0].functions;
        assert_eq!(funcs[0].result, Some(WitType::List(Box::new(WitType::S32))));
        assert_eq!(
            funcs[1].result,
            Some(WitType::Option(Box::new(WitType::StringType)))
        );
        assert!(matches!(funcs[2].result, Some(WitType::Result { .. })));
    }

    #[test]
    fn parse_resource_declaration() {
        let wit = r#"
            interface store {
                resource connection {
                    open: func(url: string) -> result<s32, string>;
                }
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].resources.len(), 1);
        assert_eq!(doc.interfaces[0].resources[0], "connection");
    }

    #[test]
    fn parse_own_borrow_types() {
        let wit = r#"
            interface handles {
                resource file-handle;
                read: func(f: borrow<file-handle>) -> list<u8>;
                close: func(f: own<file-handle>);
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        let funcs = &doc.interfaces[0].functions;
        assert!(matches!(funcs[0].params[0].1, WitType::Borrow(_)));
        assert!(matches!(funcs[1].params[0].1, WitType::Own(_)));
    }

    #[test]
    fn parse_flags_type() {
        let wit = r#"
            interface perms {
                has-any: func(p: flags { read, write }) -> bool;
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        let funcs = &doc.interfaces[0].functions;
        assert_eq!(
            funcs[0].params[0].1,
            WitType::Flags(vec!["read".to_string(), "write".to_string()])
        );
        assert_eq!(funcs[0].params[0].1.to_wit(), "flags { read, write }");
    }

    #[test]
    fn wit_to_mir_imports_roundtrip() {
        let wit = r#"
            interface host {
                add: func(a: s32, b: s32) -> s32;
                log: func(msg: string);
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        let imports = wit_interface_to_mir_imports(&doc.interfaces[0]);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].interface, "host");
        assert_eq!(imports[0].name, "add");
        assert_eq!(imports[0].param_types, vec!["s32", "s32"]);
        assert_eq!(imports[0].return_type, Some("s32".to_string()));
        assert_eq!(imports[1].name, "log");
        assert!(imports[1].return_type.is_none());
    }

    #[test]
    fn parse_comments() {
        let wit = r#"
            // This is a comment
            interface api {
                /* multi-line
                   comment */
                hello: func() -> string;
            }
        "#;
        let doc = parse_wit(wit).unwrap();
        assert_eq!(doc.interfaces[0].functions[0].name, "hello");
    }
}
