# std::component

**Stability**: Experimental
**Module**: `std::component`

## Overview

Component Model version and canonical ABI metadata. Provides version constants for the Wasm Component Model.
Full canonical ABI lift/lower helpers are planned for v4.

> ⚠️ **Experimental**: API may change in minor versions.

## Functions

### `canonical_abi_version() -> i32`

Returns the canonical ABI version number (currently `1`).

**Example:**
```ark
use std::component

let v = component::canonical_abi_version()
println(i32_to_string(v)) // "1"
```

### `component_model_version() -> String`

Returns the Component Model specification version string.

**Example:**
```ark
use std::component

let ver = component::component_model_version()
println(ver) // "0.2.0"
```
