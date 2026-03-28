# std::csv

**Stability**: Experimental
**Module**: `std::csv`

## Overview

Minimal CSV line splitting. Splits on commas without RFC 4180 quoting support.
Full quoted-field parsing is deferred to v4.

> ⚠️ **Experimental**: API may change in minor versions.

## Functions

### `csv_split_line(line: String) -> Vec<String>`

Splits a CSV line by commas and returns a `Vec<String>` of fields.
Does not handle quoted fields or escaped commas.

**Example:**

```ark
use std::csv

let fields = csv::csv_split_line("alice,30,tokyo")
// fields = ["alice", "30", "tokyo"]
println(get(fields, 0)) // "alice"
println(get(fields, 1)) // "30"
println(get(fields, 2)) // "tokyo"
```
