# std::toml

**Stability**: Experimental
**Module**: `std::toml`

## Overview

Minimal TOML line parser for simple `key=value` files.
Skips comment lines (starting with `#`) and blank lines.
Full table/array support is deferred to v4.

> ⚠️ **Experimental**: API may change in minor versions.

## Functions

### `toml_parse_line(line: String) -> String`

Parses a single TOML line. Returns the line as-is for key=value content.
Returns `""` for blank lines and comment lines (starting with `#`).

**Example:**
```ark
use std::toml

let r1 = toml::toml_parse_line("name = \"arukellt\"")
println(r1) // "name = \"arukellt\""

let r2 = toml::toml_parse_line("# this is a comment")
println(r2) // ""

let r3 = toml::toml_parse_line("")
println(r3) // ""
```
