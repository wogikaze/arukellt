# JSON Round-Trip Parity — Normalization Rules

This directory contains the test corpus for JSON round-trip parity and differential tests
introduced in issue #526.

## What "round-trip parity" means

A JSON value passes round-trip parity when:

  parse(stringify(parse(raw))) produces a value semantically equivalent to parse(raw)

We do NOT require byte-for-byte identical output because the serializer may reorder
object keys or normalize whitespace.  Instead we verify:

  1. Re-parsing the stringified form succeeds (no parse error).
  2. Accessing individual fields/indexes from the re-parsed value returns the same
     primitive values as the original parse.

## Normalization rules applied in this corpus

- Object key order: not guaranteed; tests access by key name, not by position.
- Whitespace: stringify() produces compact (no extra spaces) JSON; pretty output is
  tested separately in json_pretty.
- Number representation: integers are serialized without trailing ".0" or exponents
  unless the original text used those forms.
- String encoding: all control characters (tab, newline, carriage return) and the
  backslash/double-quote characters are escaped as \t \n \r \\ \" respectively.
- null / true / false: always lower-case literals.

## Fixture inventory (added in #526)

  json_rt_nested_object.ark   — nested object parse → access → stringify → re-parse
  json_rt_array.ark           — array parse → index → stringify → re-parse
  json_rt_scalars.ark         — null, bool (true/false), 0, negative number, type predicates
  json_rt_string_escape.ark   — tab/newline/CR/backslash/quote encoding + golden differential
  json_differential.ark       — mixed-type object stringify compared against known-correct structure

## Differential tests

A "differential" fixture checks Arukellt output against a known-correct golden value
(hardcoded in the fixture or in the .expected file).  The golden values were derived
by running the same JSON through a reference implementation (Python json module) and
recording expected field values.
