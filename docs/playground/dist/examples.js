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
/**
 * Base path for test fixtures, relative to the repository root.
 *
 * Consumers that need to resolve fixture files on disk can join this
 * with `ExampleEntry.fixturePath`.
 */
export const FIXTURE_BASE_PATH = "tests/fixtures";
// ---------------------------------------------------------------------------
// Example catalog
// ---------------------------------------------------------------------------
/**
 * The curated examples catalog.
 *
 * Each entry can be loaded via the `#example/<id>` fragment URL
 * (ADR-021 §8) or via the examples loader UI component.
 */
export const EXAMPLES = [
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

fn main() {
    let c = Color::Green
    match c {
        Color::Red => println("red"),
        Color::Green => println("green"),
        Color::Blue => println("blue"),
    }
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
        source: `pub trait Greet {}

pub struct Person {
    name: String,
}

impl Greet for Person {
    fn hello(self) -> String {
        concat(String_from("Hello, "), self.name)
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
    {
        id: "rpn-repl",
        name: "RPN REPL",
        description: "Reverse Polish notation calculator REPL. Edit stdin (e.g. \"2 7 -\") and Run.",
        stdin: "1 2 +\n",
        stdinMode: "line",
        source: `// T2 RPN REPL — interactive stdin loop.
fn read_stdin() -> String { __intrinsic_stdin_read_to_string() }

enum RpnOp { Invalid, Add, Sub, Mul }

fn is_ascii_ws(c: i32) -> bool { c == 32 || c == 9 || c == 13 }

fn print_error(msg: String) { println(concat("error: ", msg)) }

fn print_stack_count_error(count: i32) {
    print("error: stack has ")
    print(i32_to_string(count))
    println(" values")
}

fn print_result(val: i32) { println(concat("ok: result = ", i32_to_string(val))) }

fn operator_from_token(tok: String) -> RpnOp {
    if eq(tok, "+") { RpnOp::Add }
    else { if eq(tok, "-") { RpnOp::Sub } else { if eq(tok, "*") { RpnOp::Mul } else { RpnOp::Invalid } } }
}

fn apply_op_values(op: RpnOp, a: i32, b: i32) -> i32 {
    match op { RpnOp::Invalid => 0, RpnOp::Add => a + b, RpnOp::Sub => a - b, RpnOp::Mul => a * b }
}

fn apply_op(stack: Vec<i32>, op: RpnOp) -> Vec<i32> {
    if len(stack) < 2 { print_error(String_from("stack underflow")); stack }
    else {
        let b: i32 = get_unchecked(stack, len(stack) - 1)
        let _ = pop(stack)
        let a: i32 = get_unchecked(stack, len(stack) - 1)
        let _ = pop(stack)
        push(stack, apply_op_values(op, a, b))
        stack
    }
}

fn process_token(stack: Vec<i32>, tok: String) -> Vec<i32> {
    match operator_from_token(tok) {
        RpnOp::Invalid => match parse_i32(tok) {
            Result::Ok(val) => { push(stack, val); stack }
            Result::Err(_) => { print_error(concat("invalid token: ", tok)); stack }
        }
        op => apply_op(stack, op),
    }
}

fn token_end(line: String, start: i32, line_len: i32) -> i32 {
    let mut end: i32 = start + 1
    while end < line_len {
        if is_ascii_ws(char_at(line, end)) { return end }
        end = end + 1
    }
    line_len
}

fn finish_line(stack: Vec<i32>, input_trimmed: String) {
    let sp: i32 = len(stack)
    if sp == 1 {
        let val: i32 = get_unchecked(stack, 0)
        println(i32_to_string(val))
        print_result(val)
    } else { if sp > 1 { println(input_trimmed); print_stack_count_error(sp) } }
}

fn eval_line(line: String) {
    let mut stack: Vec<i32> = Vec_new_i32()
    let input_trimmed: String = trim(line)
    let line_len: i32 = len(input_trimmed)
    let mut i: i32 = 0
    while i < line_len {
        if is_ascii_ws(char_at(input_trimmed, i)) { i = i + 1 }
        else {
            let start: i32 = i
            let end: i32 = token_end(input_trimmed, start, line_len)
            stack = process_token(stack, substring(input_trimmed, start, end))
            i = end
        }
    }
    finish_line(stack, input_trimmed)
}

fn main() {
    println("RPN REPL")
    println("enter space-separated tokens (e.g. 3 4 +). Ctrl+Z to exit.")
    println("")
    let mut running: bool = true
    while running {
        print("> ")
        let input: String = read_stdin()
        if len(trim(input)) == 0 { println("bye"); running = false }
        else { eval_line(input) }
    }
}
`,
        tags: ["algorithms", "interactive"],
        fixturePath: "examples/rpn_repl.ark",
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
export function getExample(id) {
    return EXAMPLES.find((e) => e.id === id);
}
/**
 * Get all available example entries.
 *
 * Returns the frozen examples array. Do not mutate.
 */
export function getExampleList() {
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
export function getExamplesByTag() {
    const byTag = new Map();
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
export function getFixtureMap() {
    const map = new Map();
    for (const example of EXAMPLES) {
        map.set(example.id, example.fixturePath);
    }
    return map;
}
//# sourceMappingURL=examples.js.map