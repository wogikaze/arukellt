/**
 * Curated example programs for the Arukellt playground.
 *
 * Provides a hardcoded catalog of example programs that can be loaded
 * in the playground via the `#example/<id>` fragment URL (ADR-021 §8)
 * or through the examples loader UI.
 *
 * Each example references a corresponding test fixture in
 * `tests/fixtures/` so that playground examples stay in sync with
 * CI-verified code.  See `docs/playground/examples-catalog.md` for the
 * convention on how to add or update examples.
 *
 * For v1, this is a simple TypeScript array — no server needed.
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A curated example program entry. */
export interface ExampleEntry {
  /** Unique slug identifier (kebab-case, e.g., `"hello-world"`). */
  id: string;
  /** Display name for the example. */
  name: string;
  /** Short description of what the example demonstrates. */
  description: string;
  /** The Arukellt source code. */
  source: string;
  /** Tags for categorization (optional). */
  tags?: string[];
  /**
   * Path to the corresponding CI-verified test fixture, relative to
   * `tests/fixtures/`.  Must appear in `tests/fixtures/manifest.txt`.
   */
  fixturePath: string;
}

/**
 * Base path for test fixtures, relative to the repository root.
 *
 * Consumers that need to resolve fixture files on disk can join this
 * with `ExampleEntry.fixturePath`.
 */
export const FIXTURE_BASE_PATH = "tests/fixtures" as const;

// ---------------------------------------------------------------------------
// Example catalog
// ---------------------------------------------------------------------------

/**
 * The curated examples catalog.
 *
 * Each entry can be loaded via the `#example/<id>` fragment URL
 * (ADR-021 §8) or via the examples loader UI component.
 */
export const EXAMPLES: readonly ExampleEntry[] = [
  {
    id: "hello-world",
    name: "Hello World",
    description: "A minimal Arukellt program that prints a greeting.",
    source: `fn main() {
    println("Hello, world!")
}
`,
    tags: ["basics"],
    fixturePath: "hello/hello.ark",
  },
  {
    id: "variables",
    name: "Variables",
    description: "Declaring and using variables with let and mut.",
    source: `fn main() {
    let x = 42
    let mut y = 10
    y = y + x
    println(y)
}
`,
    tags: ["basics"],
    fixturePath: "guide/variables.ark",
  },
  {
    id: "functions",
    name: "Functions",
    description: "Defining and calling functions with parameters and return types.",
    source: `fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let result = add(3, 4)
    println(result)
}
`,
    tags: ["basics"],
    fixturePath: "guide/fn_add.ark",
  },
  {
    id: "structs",
    name: "Structs",
    description: "Defining structs and accessing their fields.",
    source: `pub struct Point {
    x: f64,
    y: f64,
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    println(p.x)
    println(p.y)
}
`,
    tags: ["types"],
    fixturePath: "structs/basic_struct.ark",
  },
  {
    id: "enums",
    name: "Enums",
    description: "Defining enums and pattern matching with match expressions.",
    source: `pub enum Color {
    Red,
    Green,
    Blue,
}

fn describe(c: Color) -> str {
    match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn main() {
    let c = Color::Green
    println(describe(c))
}
`,
    tags: ["types"],
    fixturePath: "enums/exhaustive_match.ark",
  },
  {
    id: "fibonacci",
    name: "Fibonacci",
    description: "Computing Fibonacci numbers with recursion.",
    source: `fn fib(n: i32) -> i32 {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    let n = 10
    println(fib(n))
}
`,
    tags: ["algorithms"],
    fixturePath: "functions/recursive.ark",
  },
  {
    id: "traits",
    name: "Traits",
    description: "Defining and implementing traits for types.",
    source: `pub trait Greet {
    fn hello(self) -> str
}

pub struct Person {
    name: str,
}

impl Greet for Person {
    fn hello(self) -> str {
        "Hello, " + self.name
    }
}

fn main() {
    let p = Person { name: "Alice" }
    println(p.hello())
}
`,
    tags: ["types", "traits"],
    fixturePath: "trait/trait_impl.ark",
  },
];

// ---------------------------------------------------------------------------
// Lookup functions
// ---------------------------------------------------------------------------

/**
 * Find an example by its slug identifier.
 *
 * @param id - The example slug (e.g., `"hello-world"`).
 * @returns The example entry, or `undefined` if not found.
 *
 * @example
 * ```ts
 * const ex = getExample("hello-world");
 * if (ex) {
 *   editor.setValue(ex.source);
 * }
 * ```
 */
export function getExample(id: string): ExampleEntry | undefined {
  return EXAMPLES.find((e) => e.id === id);
}

/**
 * Get all available example entries.
 *
 * Returns the frozen examples array. Do not mutate.
 */
export function getExampleList(): readonly ExampleEntry[] {
  return EXAMPLES;
}

/**
 * Get example IDs grouped by tag.
 *
 * @returns A map from tag name to array of example IDs with that tag.
 *
 * @example
 * ```ts
 * const byTag = getExamplesByTag();
 * const basics = byTag.get("basics"); // ["hello-world", "variables", "functions"]
 * ```
 */
export function getExamplesByTag(): Map<string, string[]> {
  const byTag = new Map<string, string[]>();
  for (const example of EXAMPLES) {
    for (const tag of example.tags ?? []) {
      let ids = byTag.get(tag);
      if (!ids) {
        ids = [];
        byTag.set(tag, ids);
      }
      ids.push(example.id);
    }
  }
  return byTag;
}

/**
 * Get a machine-readable mapping from example ID to fixture path.
 *
 * Each fixture path is relative to `tests/fixtures/`.  Use
 * {@link FIXTURE_BASE_PATH} to build the full repository-relative path.
 *
 * @returns A `Map<string, string>` where keys are example IDs and
 * values are fixture paths (e.g., `"hello/hello.ark"`).
 *
 * @example
 * ```ts
 * const fixtures = getFixtureMap();
 * const path = fixtures.get("hello-world"); // "hello/hello.ark"
 * const full = `${FIXTURE_BASE_PATH}/${path}`; // "tests/fixtures/hello/hello.ark"
 * ```
 */
export function getFixtureMap(): Map<string, string> {
  const map = new Map<string, string>();
  for (const example of EXAMPLES) {
    map.set(example.id, example.fixturePath);
  }
  return map;
}
