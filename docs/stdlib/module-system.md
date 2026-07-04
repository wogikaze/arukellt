# Module System

Arukellt supports two import syntaxes for bringing modules into scope.

## `import` (local modules)

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
import math
import utils as u

fn main() {
    math::add(1, 2)
    u::helper()
}
```

`import name` loads `name.ark` from the same directory as the current file.

## `use` (namespaced modules)

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::core
use std::text as t
use std::collections::string

fn main() {
    core::identity(42)
    t::repeat("ha", 3)
}
```

`use std::path::to::module` loads from the `std/` directory tree, resolving
`::` to `/` in the filesystem. The last path segment becomes the default
module alias.

### Path resolution order

For `use std::foo::bar`:

1. `std/foo/bar.ark` (direct file)
2. `std/foo/bar/mod.ark` (directory module)

For non-`std` paths, local directory is checked first, then `std/`.

### Module alias

<!-- skip-doc-check -->
```ark
use std::core as c     // alias: c
use std::core          // alias: core (last segment)
```

## Qualified calls

Imported module functions are called with `module::function()` syntax:

<!-- skip-doc-check --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
import math
fn main() {
    let x = math::add(1, 2)
}
```

Public symbols from imported modules are also available as bare names
unless they conflict with prelude definitions. In case of conflict,
use the qualified form.

## Prelude interaction

The prelude (`std/prelude.ark`) is always auto-imported. When an imported
module defines a symbol that conflicts with a prelude symbol, the first
definition wins and the duplicate is silently skipped. Use qualified
calls to disambiguate.

## Error diagnostics

- **E0104**: Module not found (file does not exist at resolved path)
- **E0103**: Circular import detected

## Directory structure convention

```
std/
├── prelude.ark          # auto-imported
├── core/
│   └── mod.ark          # use std::core
├── text/
│   └── mod.ark          # use std::text
├── bytes/
│   └── mod.ark          # use std::bytes
├── collections/
│   ├── string.ark       # use std::collections::string
│   └── ...
└── host/
    ├── stdio.ark        # use std::host::stdio
    ├── fs.ark           # use std::host::fs
    ├── env.ark          # use std::host::env
    ├── process.ark      # use std::host::process
    ├── clock.ark        # use std::host::clock
    └── random.ark       # use std::host::random
```

## Limitations (v3)

- No wildcard imports (`use std::foo::*`)
- No `pub use` re-exports
- Maximum 4 path segments recommended

## Function-level destructuring (v4 — issue #717)

`use path::module::{fn_a, fn_b}` imports functions directly into scope,
allowing bare calls without module qualification:

```arukellt
use std::text::string::{split, join}

fn main() {
    let parts = split("a,b", ",")    // resolves to string::split
    let result = join(parts, ",")    // resolves to string::join
}
```

The resolver registers a use-alias mapping the short name (`split`) to
the qualified path (`string::split`). When a bare name is encountered in
a path expression, the resolver rewrites it to the qualified path before
typechecking and MIR lowering. Locals and parameters shadow use-aliases.
