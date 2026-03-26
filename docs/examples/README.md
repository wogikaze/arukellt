# Executable Examples

These examples serve as both documentation and integration tests.
Each `.ark` file has a corresponding `.expected` file with the expected stdout.

## Running Examples

```bash
# Run a single example
target/release/arukellt run docs/examples/hello.ark

# Run all examples and verify output
for f in docs/examples/*.ark; do
  base=$(basename "$f" .ark)
  expected="docs/examples/$base.expected"
  if [ -f "$expected" ]; then
    actual=$(target/release/arukellt run "$f" 2>/dev/null)
    exp=$(cat "$expected")
    if [ "$actual" = "$exp" ]; then
      echo "PASS: $base"
    else
      echo "FAIL: $base"
    fi
  fi
done
```

## Examples

| File | Description |
|------|-------------|
| `hello.ark` | Hello world |
| `struct-enum.ark` | Struct and enum basics |
| `vec.ark` | Vec operations |
| `closure.ark` | Closures and higher-order functions |
| `result.ark` | Result type and error handling |
| `for-loop.ark` | For loop with ranges |
| `string-interpolation.ark` | f-string interpolation |
