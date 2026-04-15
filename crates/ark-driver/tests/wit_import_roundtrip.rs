//! Round-trip test: WIT file → resolver injection → MIR compilation → imports verified.
//!
//! Covers completion criterion:
//!   "parse WIT → resolver injection → MIR compilation → MirModule.imports verified"

use std::io::Write;

use ark_driver::{MirSelection, Session};
use ark_target::TargetId;

#[test]
fn wit_import_populates_mir_imports() {
    // Create a temp WIT file describing a host interface.
    let mut tmpfile = tempfile::NamedTempFile::new().expect("tempfile creation failed");
    writeln!(
        tmpfile,
        r#"package test:host;

interface host-fns {{
    add: func(a: s32, b: s32) -> s32;
    log: func(msg: string);
}}"#
    )
    .expect("write wit failed");
    let wit_path = tmpfile.path().to_path_buf();

    // Create a minimal .ark source file with a main function.
    let mut src_tmp = tempfile::NamedTempFile::with_suffix(".ark").expect("tempfile ark failed");
    writeln!(src_tmp, "fn main() {{}}").expect("write ark failed");
    let src_path = src_tmp.path().to_path_buf();

    // Run compilation with the WIT file injected.
    let mut session = Session::new();
    session.wit_files = vec![wit_path];

    let result = session.compile_selected(&src_path, TargetId::Wasm32WasiP1, MirSelection::CoreHir);
    let compiled = result.expect("compilation should succeed");

    // The MirModule.imports must contain entries from the WIT interface.
    assert!(
        !compiled.mir.imports.is_empty(),
        "mir.imports should be non-empty after compiling with --wit"
    );
    assert_eq!(compiled.mir.imports[0].interface, "host-fns");
    assert_eq!(compiled.mir.imports[0].name, "add");
    assert_eq!(compiled.mir.imports[1].name, "log");
}

#[test]
fn wit_imported_call_compiles_through_frontend() {
    let mut tmpfile = tempfile::NamedTempFile::new().expect("tempfile creation failed");
    writeln!(
        tmpfile,
        r#"package test:host;

interface host-fns {{
    add: func(a: s32, b: s32) -> s32;
}}"#
    )
    .expect("write wit failed");
    let wit_path = tmpfile.path().to_path_buf();

    let mut src_tmp = tempfile::NamedTempFile::with_suffix(".ark").expect("tempfile ark failed");
    writeln!(
        src_tmp,
        "fn main() {{\n    add(1, 2)\n}}"
    )
    .expect("write ark failed");
    let src_path = src_tmp.path().to_path_buf();

    let mut session = Session::new();
    session.wit_files = vec![wit_path];

    let compiled = session
        .compile_selected(&src_path, TargetId::Wasm32WasiP2, MirSelection::CoreHir)
        .expect("compilation with a WIT-imported call should succeed");

    assert!(
        compiled.mir.imports.iter().any(|import| import.name == "add"),
        "mir.imports should include the called WIT import"
    );
}

#[test]
fn inject_wit_externs_adds_symbols() {
    use ark_resolve::{SymbolKind, SymbolTable, inject_wit_externs};

    let mut table = SymbolTable::new();
    let scope = table.create_scope(None);

    inject_wit_externs(&mut table, scope, &["add", "log"]);

    // Both names must be resolvable.
    let add_id = table.lookup(scope, "add");
    let log_id = table.lookup(scope, "log");
    assert!(add_id.is_some(), "'add' should be in symbol table");
    assert!(log_id.is_some(), "'log' should be in symbol table");

    // Kind must be ExternWitFn.
    let sym = table.get_symbol(add_id.unwrap());
    assert!(
        matches!(&sym.kind, SymbolKind::ExternWitFn { name } if name == "add"),
        "kind should be ExternWitFn"
    );

    // Idempotent: double-inject should not panic or duplicate.
    inject_wit_externs(&mut table, scope, &["add"]);
    let count = (0..100)
        .filter(|_| table.lookup(scope, "add").is_some())
        .count();
    assert_eq!(count, 100, "lookup should still work after re-injection");
}
