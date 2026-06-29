//! `arukellt doc <symbol>` — look up stdlib documentation from std/manifest.toml.

use ark_stdlib::{ManifestFunction, ManifestModule, StdlibManifest};
use ark_target::TargetId;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Manifest loading
// ---------------------------------------------------------------------------

fn find_repo_root() -> Option<std::path::PathBuf> {
    // 1. ARUKELLT_MANIFEST env var overrides everything (points to manifest.toml)
    if let Ok(p) = std::env::var("ARUKELLT_MANIFEST") {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            return path.parent().map(|p| p.to_path_buf());
        }
    }

    // 2. Walk up from cwd looking for std/manifest.toml
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join("std").join("manifest.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }

    // 3. Compiled-in fallback: CARGO_MANIFEST_DIR → ../../ (workspace root)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = std::path::PathBuf::from(manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());
        if let Some(root) = p
            && root.join("std").join("manifest.toml").exists()
        {
            return Some(root);
        }
    }

    None
}

fn load_stdlib_manifest() -> Result<StdlibManifest, String> {
    // Allow overriding with full path to manifest.toml via ARUKELLT_MANIFEST
    if let Ok(p) = std::env::var("ARUKELLT_MANIFEST") {
        let path = std::path::PathBuf::from(p);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        return StdlibManifest::parse(&text).map_err(|e| format!("cannot parse manifest: {e}"));
    }

    let root = find_repo_root().ok_or_else(|| {
        "could not locate std/manifest.toml — run from the Arukellt workspace, or set ARUKELLT_MANIFEST".to_string()
    })?;
    StdlibManifest::load_from_repo(&root)
}

// ---------------------------------------------------------------------------
// Symbol resolution helpers
// ---------------------------------------------------------------------------

/// Possible ways to interpret a user-provided symbol query.
enum QueryKind {
    /// e.g. `println` — just a bare name
    BareName(String),
    /// e.g. `std::host::http::get` — module path + function name
    ModuleAndName { module: String, name: String },
    /// e.g. `std::host::http` — a module path (ends with a module name)
    ModulePath(String),
}

fn classify_query(symbol: &str) -> QueryKind {
    if !symbol.contains("::") {
        return QueryKind::BareName(symbol.to_string());
    }

    // Split off the last segment
    let (prefix, last) = symbol
        .rsplit_once("::")
        .expect("already checked contains '::'");

    // Heuristic: if the prefix itself matches a known module or there's a last
    // segment that looks like a function (lowercase start), treat as ModuleAndName.
    // Otherwise treat as module path.
    // We'll do a simple check: lowercase first character → likely function name.
    if last.starts_with(|c: char| c.is_lowercase()) {
        QueryKind::ModuleAndName {
            module: prefix.to_string(),
            name: last.to_string(),
        }
    } else {
        // Uppercase start → type or module, treat as module path
        QueryKind::ModulePath(symbol.to_string())
    }
}

/// Find an exact function by bare name (searches all functions).
fn find_by_name<'a>(manifest: &'a StdlibManifest, name: &str) -> Vec<&'a ManifestFunction> {
    manifest
        .functions
        .iter()
        .filter(|f| f.name == name && f.kind.as_deref() != Some("intrinsic"))
        .collect()
}

/// Find a function by module + name.
fn find_by_module_and_name<'a>(
    manifest: &'a StdlibManifest,
    module: &str,
    name: &str,
) -> Vec<&'a ManifestFunction> {
    manifest
        .functions
        .iter()
        .filter(|f| {
            f.name == name
                && f.module.as_deref() == Some(module)
                && f.kind.as_deref() != Some("intrinsic")
        })
        .collect()
}

/// Find a module entry by name.
fn find_module<'a>(manifest: &'a StdlibManifest, name: &str) -> Option<&'a ManifestModule> {
    manifest.modules.iter().find(|m| m.name == name)
}

/// Fuzzy-match: return up to `limit` nearest function names to `query`.
fn find_closest(manifest: &StdlibManifest, query: &str, limit: usize) -> Vec<String> {
    let mut scored: Vec<(usize, String)> = manifest
        .functions
        .iter()
        .filter(|f| f.kind.as_deref() != Some("intrinsic"))
        .map(|f| {
            // Score against bare name and full qualified name
            let bare = strsim::levenshtein(query, &f.name);
            let full = if let Some(ref m) = f.module {
                let qualified = format!("{}::{}", m, f.name);
                strsim::levenshtein(query, &qualified)
            } else {
                usize::MAX
            };
            let score = bare.min(full);
            let display = if let Some(ref m) = f.module {
                format!("{}::{}", m, f.name)
            } else {
                f.name.clone()
            };
            (score, display)
        })
        .collect();
    scored.sort_by_key(|(d, _)| *d);
    scored.dedup_by_key(|(_, n)| n.clone());
    scored.into_iter().take(limit).map(|(_, n)| n).collect()
}

// ---------------------------------------------------------------------------
// Text output helpers
// ---------------------------------------------------------------------------

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err() && std::env::var("ARUKELLT_NO_COLOR").is_err()
}

fn bold(s: &str) -> String {
    if use_color() {
        format!("\x1b[1m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    if use_color() {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn yellow(s: &str) -> String {
    if use_color() {
        format!("\x1b[33m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn format_signature(f: &ManifestFunction) -> String {
    let params = f.params.join(", ");
    let ret = f.returns.as_deref().unwrap_or("()");
    format!("fn {}({}) -> {}", f.name, params, ret)
}

fn target_availability_summary(f: &ManifestFunction) -> String {
    if let Some(ref avail) = f.availability {
        match (avail.t1, avail.t3) {
            (true, true) => {
                if let Some(ref note) = avail.note {
                    format!("wasm32-wasi-p1, wasm32-wasi-p2 ({})", note)
                } else {
                    "wasm32-wasi-p1, wasm32-wasi-p2".to_string()
                }
            }
            (true, false) => "wasm32-wasi-p1 only".to_string(),
            (false, true) => {
                if let Some(ref note) = avail.note {
                    format!("wasm32-wasi-p2 only ({})", note)
                } else {
                    "wasm32-wasi-p2 only".to_string()
                }
            }
            (false, false) => "not available on any standard target".to_string(),
        }
    } else if !f.target.is_empty() {
        f.target.join(", ")
    } else {
        "wasm32-wasi-p1, wasm32-wasi-p2".to_string()
    }
}

fn target_warning(f: &ManifestFunction, filter_target: &TargetId) -> Option<String> {
    let target_str = filter_target.canonical_name();
    let is_t1 = *filter_target == TargetId::Wasm32WasiP1;
    let is_t3 = *filter_target == TargetId::Wasm32WasiP2;

    if let Some(ref avail) = f.availability {
        if is_t1 && !avail.t1 {
            return Some(format!("⚠  Not available on {}", target_str));
        }
        if is_t3 && !avail.t3 {
            return Some(format!("⚠  Not available on {}", target_str));
        }
    } else if !f.target.is_empty() {
        let available = f.target.iter().any(|t| t.as_str() == target_str);
        if !available {
            return Some(format!("⚠  Not available on {}", target_str));
        }
    }
    None
}

fn print_function(f: &ManifestFunction, filter_target: Option<&TargetId>) {
    // Target warning first
    if let Some(ft) = filter_target
        && let Some(warn) = target_warning(f, ft)
    {
        println!("{}", yellow(&warn));
    }

    // Signature
    println!("{}", bold(&format_signature(f)));

    // Module
    if let Some(ref m) = f.module {
        println!("  {} {}", dim("Module:"), m);
    }

    // Stability
    let stability = f.stability.as_deref().unwrap_or("unknown");
    println!("  {} {}", dim("Stability:"), stability);

    // Target availability
    let avail = target_availability_summary(f);
    println!("  {} {}", dim("Supported on:"), avail);

    // Deprecated
    if let Some(ref dep) = f.deprecated_by {
        println!("  {} Use {} instead", yellow("⚠ Deprecated."), dep);
    }

    // Doc
    if let Some(ref doc) = f.doc {
        println!();
        println!("  {}", doc);
    }

    // Errors
    if let Some(ref errors) = f.errors {
        println!();
        println!("  {} {}", dim("Errors:"), errors);
    }

    // Examples
    if !f.examples.is_empty() {
        println!();
        println!("  {}:", dim("Examples"));
        for ex in &f.examples {
            if let Some(ref desc) = ex.description {
                println!("    {}", dim(&format!("// {desc}")));
            }
            for line in ex.code.lines() {
                println!("    {}", line);
            }
            if let Some(ref out) = ex.output {
                println!("    {}", dim(&format!("// output: {out}")));
            }
        }
    }

    // See also
    if let Some(ref see) = f.see_also {
        println!();
        println!("  {} {}", dim("See also:"), see);
    }
}

fn print_module_info(
    manifest: &StdlibManifest,
    module: &ManifestModule,
    filter_target: Option<&TargetId>,
) {
    println!("{}", bold(&format!("module {}", module.name)));
    if let Some(ref stab) = module.stability {
        println!("  {} {}", dim("Stability:"), stab);
    }
    if !module.target.is_empty() {
        println!("  {} {}", dim("Supported on:"), module.target.join(", "));
    }
    if let Some(ref doc) = module.doc {
        println!();
        println!("  {}", doc);
    }

    // List functions in this module
    let fns: Vec<&ManifestFunction> = manifest
        .functions
        .iter()
        .filter(|f| {
            f.module.as_deref() == Some(&module.name) && f.kind.as_deref() != Some("intrinsic")
        })
        .collect();

    if !fns.is_empty() {
        println!();
        println!("  {}:", dim("Functions"));
        for f in fns {
            // Target warning
            let warn = filter_target.and_then(|ft| target_warning(f, ft));
            let warn_str = warn.map(|w| format!(" {}", yellow(&w))).unwrap_or_default();
            let stab = f
                .stability
                .as_deref()
                .map(|s| format!("  [{s}]"))
                .unwrap_or_default();
            let sig = format_signature(f);
            println!("    {}{}{}", sig, stab, warn_str);
        }
    }
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DocJsonFunction<'a> {
    kind: &'static str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<&'a str>,
    params: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    returns: Option<&'a str>,
    stability: &'a str,
    target: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    availability: Option<DocJsonAvailability<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<&'a str>,
    examples: Vec<DocJsonExample<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    see_also: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deprecated_by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_warning: Option<String>,
}

#[derive(Serialize)]
struct DocJsonAvailability<'a> {
    t1: bool,
    t3: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
}

#[derive(Serialize)]
struct DocJsonExample<'a> {
    code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<&'a str>,
}

#[derive(Serialize)]
struct DocJsonModule<'a> {
    kind: &'static str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    stability: Option<&'a str>,
    target: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    doc: Option<&'a str>,
    functions: Vec<String>,
}

#[derive(Serialize)]
struct DocJsonNotFound {
    kind: &'static str,
    symbol: String,
    candidates: Vec<String>,
}

fn emit_function_json(f: &ManifestFunction, filter_target: Option<&TargetId>) {
    let avail = f.availability.as_ref().map(|a| DocJsonAvailability {
        t1: a.t1,
        t3: a.t3,
        note: a.note.as_deref(),
    });
    let examples: Vec<DocJsonExample> = f
        .examples
        .iter()
        .map(|e| DocJsonExample {
            code: &e.code,
            description: e.description.as_deref(),
            output: e.output.as_deref(),
        })
        .collect();
    let tw = filter_target.and_then(|ft| target_warning(f, ft));
    let out = DocJsonFunction {
        kind: "function",
        name: &f.name,
        module: f.module.as_deref(),
        params: &f.params,
        returns: f.returns.as_deref(),
        stability: f.stability.as_deref().unwrap_or("unknown"),
        target: &f.target,
        availability: avail,
        doc: f.doc.as_deref(),
        errors: f.errors.as_deref(),
        examples,
        see_also: f.see_also.as_deref(),
        deprecated_by: f.deprecated_by.as_deref(),
        target_warning: tw,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("DocJsonFunction serialization cannot fail")
    );
}

fn emit_module_json(manifest: &StdlibManifest, module: &ManifestModule) {
    let fns: Vec<String> = manifest
        .functions
        .iter()
        .filter(|f| {
            f.module.as_deref() == Some(&module.name) && f.kind.as_deref() != Some("intrinsic")
        })
        .map(|f| format!("{}::{}", module.name, f.name))
        .collect();
    let out = DocJsonModule {
        kind: "module",
        name: &module.name,
        stability: module.stability.as_deref(),
        target: &module.target,
        doc: module.doc.as_deref(),
        functions: fns,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("DocJsonModule serialization cannot fail")
    );
}

fn emit_not_found_json(symbol: &str, candidates: Vec<String>) {
    let out = DocJsonNotFound {
        kind: "not_found",
        symbol: symbol.to_string(),
        candidates,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("DocJsonNotFound serialization cannot fail")
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Main handler for `arukellt doc <symbol> [--json] [--target <TARGET>] [--all]`.
///
/// Returns `true` if the symbol was found, `false` otherwise (for exit code).
pub fn cmd_doc(symbol: &str, json: bool, filter_target: Option<&TargetId>, _all: bool) -> bool {
    let manifest = match load_stdlib_manifest() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: could not load stdlib manifest: {}", e);
            return false;
        }
    };

    match classify_query(symbol) {
        QueryKind::BareName(ref name) => {
            let fns = find_by_name(&manifest, name);
            if !fns.is_empty() {
                for f in &fns {
                    if json {
                        emit_function_json(f, filter_target);
                    } else {
                        print_function(f, filter_target);
                    }
                }
                true
            } else {
                // Also check if it's a module name
                if let Some(m) = find_module(&manifest, name) {
                    if json {
                        emit_module_json(&manifest, m);
                    } else {
                        print_module_info(&manifest, m, filter_target);
                    }
                    return true;
                }
                let candidates = find_closest(&manifest, name, 5);
                not_found(symbol, candidates, json);
                false
            }
        }
        QueryKind::ModuleAndName {
            ref module,
            ref name,
        } => {
            let fns = find_by_module_and_name(&manifest, module, name);
            if !fns.is_empty() {
                for f in &fns {
                    if json {
                        emit_function_json(f, filter_target);
                    } else {
                        print_function(f, filter_target);
                    }
                }
                true
            } else {
                // Maybe the whole thing is a module path?
                if let Some(m) = find_module(&manifest, symbol) {
                    if json {
                        emit_module_json(&manifest, m);
                    } else {
                        print_module_info(&manifest, m, filter_target);
                    }
                    return true;
                }
                let candidates = find_closest(&manifest, symbol, 5);
                not_found(symbol, candidates, json);
                false
            }
        }
        QueryKind::ModulePath(ref path) => {
            if let Some(m) = find_module(&manifest, path) {
                if json {
                    emit_module_json(&manifest, m);
                } else {
                    print_module_info(&manifest, m, filter_target);
                }
                true
            } else {
                // Maybe it matches a function name exactly?
                let fns = find_by_name(&manifest, path.rsplit("::").next().unwrap_or(path));
                if !fns.is_empty() {
                    for f in &fns {
                        if json {
                            emit_function_json(f, filter_target);
                        } else {
                            print_function(f, filter_target);
                        }
                    }
                    return true;
                }
                let candidates = find_closest(&manifest, path, 5);
                not_found(symbol, candidates, json);
                false
            }
        }
    }
}

fn not_found(symbol: &str, candidates: Vec<String>, json: bool) {
    if json {
        emit_not_found_json(symbol, candidates);
    } else {
        eprintln!("error: symbol '{}' not found", symbol);
        if !candidates.is_empty() {
            eprintln!();
            eprintln!("Did you mean?");
            for c in &candidates {
                eprintln!("  {}", c);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn load_manifest_for_tests() -> StdlibManifest {
        StdlibManifest::load_from_repo(&repo_root()).unwrap()
    }

    #[test]
    fn test_classify_bare_name() {
        match classify_query("println") {
            QueryKind::BareName(n) => assert_eq!(n, "println"),
            _ => panic!("expected BareName"),
        }
    }

    #[test]
    fn test_classify_module_and_name() {
        match classify_query("std::host::http::get") {
            QueryKind::ModuleAndName { module, name } => {
                assert_eq!(module, "std::host::http");
                assert_eq!(name, "get");
            }
            _ => panic!("expected ModuleAndName"),
        }
    }

    #[test]
    fn test_classify_module_path() {
        // A path ending with an uppercase segment or pure module path
        match classify_query("std::host::http") {
            QueryKind::ModuleAndName { .. } | QueryKind::ModulePath(_) => {}
            QueryKind::BareName(_) => panic!("expected ModuleAndName or ModulePath"),
        }
    }

    #[test]
    fn test_find_println() {
        let manifest = load_manifest_for_tests();
        let fns = find_by_name(&manifest, "println");
        assert!(!fns.is_empty(), "println should be found");
        let f = fns[0];
        assert_eq!(f.name, "println");
        assert!(
            f.stability.as_deref() == Some("stable"),
            "println should be stable"
        );
    }

    #[test]
    fn test_find_http_get_by_module() {
        let manifest = load_manifest_for_tests();
        let fns = find_by_module_and_name(&manifest, "std::host::http", "get");
        assert!(!fns.is_empty(), "http::get should be found by module+name");
        let f = fns[0];
        assert_eq!(f.name, "get");
        assert_eq!(f.module.as_deref(), Some("std::host::http"));
        // stability could be "experimental" or "available" depending on manifest version
        assert!(
            f.stability.is_some(),
            "http::get should have a stability field"
        );
    }

    #[test]
    fn test_fuzzy_closest_for_typo() {
        let manifest = load_manifest_for_tests();
        let candidates = find_closest(&manifest, "printlm", 3);
        assert!(
            !candidates.is_empty(),
            "should return candidates for typo 'printlm'"
        );
        // println should be among top candidates
        assert!(
            candidates.iter().any(|c| c.contains("println")),
            "println should appear in candidates for 'printlm', got: {:?}",
            candidates
        );
    }

    #[test]
    fn test_fuzzy_closest_for_nonexistent() {
        let manifest = load_manifest_for_tests();
        let candidates = find_closest(&manifest, "nonexistent_xyz_qwerty", 5);
        assert!(
            !candidates.is_empty(),
            "should return some candidates even for nonsense input"
        );
    }

    #[test]
    fn test_target_availability_both() {
        let manifest = load_manifest_for_tests();
        let fns = find_by_name(&manifest, "println");
        assert!(!fns.is_empty());
        let summary = target_availability_summary(fns[0]);
        assert!(
            summary.contains("wasm32-wasi-p1"),
            "println should support p1, got: {summary}"
        );
    }

    #[test]
    fn test_json_function_output() {
        // Test internal helpers to verify JSON struct serializes correctly
        let manifest = load_manifest_for_tests();
        let fns = find_by_name(&manifest, "println");
        assert!(!fns.is_empty());
        let f = fns[0];
        // Build the JSON struct manually to verify it serializes
        let avail = f.availability.as_ref().map(|a| DocJsonAvailability {
            t1: a.t1,
            t3: a.t3,
            note: a.note.as_deref(),
        });
        let out = DocJsonFunction {
            kind: "function",
            name: &f.name,
            module: f.module.as_deref(),
            params: &f.params,
            returns: f.returns.as_deref(),
            stability: f.stability.as_deref().unwrap_or("unknown"),
            target: &f.target,
            availability: avail,
            doc: f.doc.as_deref(),
            errors: f.errors.as_deref(),
            examples: f
                .examples
                .iter()
                .map(|e| DocJsonExample {
                    code: &e.code,
                    description: e.description.as_deref(),
                    output: e.output.as_deref(),
                })
                .collect(),
            see_also: f.see_also.as_deref(),
            deprecated_by: f.deprecated_by.as_deref(),
            target_warning: None,
        };
        let json_str = serde_json::to_string_pretty(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["kind"], "function");
        assert_eq!(parsed["name"], "println");
        assert!(parsed.get("stability").is_some(), "should have stability");
        assert!(
            parsed.get("availability").is_some(),
            "should have availability"
        );
    }
}
