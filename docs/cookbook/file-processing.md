# Cookbook: File Processing

Read a file, process its lines, and write output using `std::host::fs`, `std::host::stdio`, and `std::text`.

## Read a File and Print Its Contents

```ark
use std::host::fs
use std::host::stdio

let result = fs::read_to_string("input.txt")
match result {
    Ok(contents) => stdio::println(contents),
    Err(e) => stdio::eprintln(concat("error: ", e)),
}
```

## Process Lines

```ark
use std::host::fs
use std::host::stdio
use std::text

let result = fs::read_to_string("data.csv")
match result {
    Ok(contents) => {
        let lines = text::split(contents, "\n")
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if text::is_empty(line) == false {
                stdio::println(concat("line ", concat(i32_to_string(i), concat(": ", line))))
            }
            i = i + 1
        }
    },
    Err(e) => stdio::eprintln(concat("read failed: ", e)),
}
```

## Filter and Write Output

```ark
use std::host::fs
use std::host::stdio
use std::text

// Read input, keep only non-empty lines, write to output
let result = fs::read_to_string("input.txt")
match result {
    Ok(contents) => {
        let lines = text::split(contents, "\n")
        let mut output = ""
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if text::is_empty(line) == false {
                output = concat(output, concat(line, "\n"))
            }
            i = i + 1
        }
        let write_result = fs::write_string("output.txt", output)
        match write_result {
            Ok(_) => stdio::println("done"),
            Err(e) => stdio::eprintln(concat("write failed: ", e)),
        }
    },
    Err(e) => stdio::eprintln(concat("read failed: ", e)),
}
```

## Read, Transform, and Write (Pipeline Pattern)

```ark
use std::host::fs
use std::host::stdio
use std::text

fn to_upper_line(line: String) -> String {
    text::to_uppercase(line)
}

let result = fs::read_to_string("names.txt")
match result {
    Ok(contents) => {
        let lines = text::split(contents, "\n")
        let mut output = ""
        let n = len(lines)
        let mut i = 0
        while i < n {
            let line = get(lines, i)
            if text::is_empty(line) == false {
                output = concat(output, concat(to_upper_line(line), "\n"))
            }
            i = i + 1
        }
        fs::write_string("upper_names.txt", output)
    },
    Err(e) => stdio::eprintln(concat("error: ", e)),
}
```
