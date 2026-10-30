# Cookbook: File Processing

Read a file, process its lines, and write output using `std::host::fs`, `std::host::stdio`, and `std::text`.

> Policy: user-facing samples should avoid direct low-level prelude helper calls.
> Prefer module-local wrappers/facades; if needed, hide prelude details behind local helper functions.

## Read a File and Print Its Contents

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::fs::Fs
use std::host::stdio::Stdio

let result = Fs::read_to_string("input.txt")
match result {
    Ok(contents) => Stdio::println(contents),
    Err(e) => Stdio::eprintln(concat("error: ", e)),
}
```

## Process Lines

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::fs::Fs
use std::host::stdio::Stdio
use std::text

let result = Fs::read_to_string("data.csv")
match result {
    Ok(contents) => {
        let lines = contents.split("\n")
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if line.is_empty() == false {
                Stdio::println(concat("line ", concat(to_string(i), concat(": ", line))))
            }
            i = i + 1
        }
    },
    Err(e) => Stdio::eprintln(concat("read failed: ", e)),
}
```

## Filter and Write Output

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::fs::Fs
use std::host::stdio::Stdio
use std::text

// Read input, keep only non-empty lines, write to output
let result = Fs::read_to_string("input.txt")
match result {
    Ok(contents) => {
        let lines = contents.split("\n")
        let mut output = ""
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if line.is_empty() == false {
                output = concat(output, concat(line, "\n"))
            }
            i = i + 1
        }
        let write_result = Fs::write_string("output.txt", output)
        match write_result {
            Ok(_) => Stdio::println("done"),
            Err(e) => Stdio::eprintln(concat("write failed: ", e)),
        }
    },
    Err(e) => Stdio::eprintln(concat("read failed: ", e)),
}
```

## Read, Transform, and Write (Pipeline Pattern)

<!-- skip-doc-check reason="legacy example not fixture-backed" owner="#683" kind="non-runnable" expires="2026-12-31" --> <!-- TODO(#461): fix or wrap this doc example -->
```ark
use std::host::fs::Fs
use std::host::stdio::Stdio
use std::text

fn to_upper_line(line: String) -> String {
    text::to_uppercase(line)
}

let result = Fs::read_to_string("names.txt")
match result {
    Ok(contents) => {
        let lines = contents.split("\n")
        let mut output = ""
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if line.is_empty() == false {
                output = concat(output, concat(to_upper_line(line), "\n"))
            }
            i = i + 1
        }
        Fs::write_string("upper_names.txt", output)
    },
    Err(e) => Stdio::eprintln(concat("error: ", e)),
}
```
