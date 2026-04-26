# Incremental Parse Design

## Parse Unit Granularity
The fundamental unit of incremental parsing is the **file** (module). When a file changes, the entire file is re-parsed. Fine-grained incremental parsing at the declaration or function level is considered out of scope for Phase 1 to limit complexity.

## Invalidation Strategy
- A central file watcher or language server component maintains the filesystem state.
- When an edit occurs, the specific `.ark` file is marked as dirty.
- The cache for the AST corresponding to that file is invalidated.

## Cache/Memoization Keys
- The `FileId` or canonical path serves as the primary key.
- The value is the parsed `ast::Module`.

## Integration Points
- The `CompilerSession` or driver queries the parser layer for an AST by `FileId`.
- If the AST is cached and clean, it's returned immediately.
- If dirty, the file is re-parsed, the typechecker is notified to drop dependent type information for that file, and the AST cache is updated.

## Stability Guarantees
- Whitespace-only changes do not trigger downstream typechecking if the AST hash remains identical (optional future optimization).
- Source maps and span information must be fully recomputed upon file re-parse.
