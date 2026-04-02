# Playground Examples Catalog

The playground ships a curated catalog of example programs defined in
`playground/src/examples.ts`.  Each example is linked to a CI-verified
test fixture in `tests/fixtures/` so that playground examples always
stay in sync with the compiler.

## Architecture

```
playground/src/examples.ts   ← catalog (source + metadata + fixturePath)
        │
        ├── ExampleEntry.fixturePath  ──►  tests/fixtures/<path>.ark
        │                                     │
        │                                     └── listed in manifest.txt
        │                                          (verified by CI harness)
        └── FIXTURE_BASE_PATH = "tests/fixtures"
```

- **`ExampleEntry.fixturePath`** — relative path from `tests/fixtures/`
  to the `.ark` file that CI compiles and runs.
- **`FIXTURE_BASE_PATH`** — constant `"tests/fixtures"`, so consumers
  can resolve the full repository-relative path as
  `${FIXTURE_BASE_PATH}/${entry.fixturePath}`.
- **`getFixtureMap()`** — returns a `Map<string, string>` of example
  IDs → fixture paths for machine-readable tooling.

## How to add a new example

1. **Create or identify a fixture**.  Place a `.ark` file under
   `tests/fixtures/<category>/`.  It must compile and run correctly.

2. **Add the fixture to `tests/fixtures/manifest.txt`**.  Use the
   `run:<category>/<file>.ark` format so the CI harness includes it.
   Verify locally:

   ```bash
   bash scripts/run/verify-harness.sh --quick
   ```

3. **Add the example to the catalog** in
   `playground/src/examples.ts`.  Fill in all required fields:

   ```ts
   {
     id: "my-example",            // kebab-case slug
     name: "My Example",          // display name
     description: "What it shows.", // short description
     source: `fn main() { ... }\n`, // playground source (ends with \n)
     tags: ["basics"],            // optional categorization tags
     fixturePath: "category/my_example.ark",  // relative to tests/fixtures/
   },
   ```

   > **Note:** The `source` field contains a simplified playground
   > version of the program (no `use std::host::stdio` imports,
   > using `println()` directly).  The fixture file contains the
   > full CI-compilable version with explicit imports.

4. **Run the playground tests**:

   ```bash
   cd playground && npx tsc --noEmit          # type-check
   cd playground && npx tsc && node --test dist/tests/*.test.js  # run tests
   ```

5. **Verify end-to-end**:

   ```bash
   bash scripts/run/verify-harness.sh --quick
   ```

## Current catalog

| Example ID    | Fixture Path                    | Tags              |
|---------------|---------------------------------|-------------------|
| hello-world   | `hello/hello.ark`               | basics            |
| variables     | `guide/variables.ark`           | basics            |
| functions     | `guide/fn_add.ark`              | basics            |
| structs       | `structs/basic_struct.ark`      | types             |
| enums         | `enums/exhaustive_match.ark`    | types             |
| fibonacci     | `functions/recursive.ark`       | algorithms        |
| traits        | `trait/trait_impl.ark`          | types, traits     |

## Machine-readable access

```ts
import { getFixtureMap, FIXTURE_BASE_PATH } from "@arukellt/playground";

const fixtures = getFixtureMap();
// Map { "hello-world" => "hello/hello.ark", "variables" => "guide/variables.ark", ... }

for (const [id, path] of fixtures) {
  console.log(`${id} → ${FIXTURE_BASE_PATH}/${path}`);
}
```

## Design decisions

- **Playground source ≠ fixture source**.  The playground `source`
  field uses simplified syntax (bare `println()`) for a friendlier
  first experience.  The fixture file uses full CI-compilable syntax
  (`use std::host::stdio`, `stdio::println()`).  Both exercise the
  same language feature.

- **`fixturePath` is required**.  Every catalog entry must link to a
  fixture so new examples cannot be added without a corresponding
  CI test.  The test suite enforces this.

- **One fixture per example**.  Each example maps to exactly one
  fixture file.  This keeps the mapping simple and avoids ambiguity.
