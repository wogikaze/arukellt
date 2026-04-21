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
export declare const FIXTURE_BASE_PATH: "tests/fixtures";
/**
 * The curated examples catalog.
 *
 * Each entry can be loaded via the `#example/<id>` fragment URL
 * (ADR-021 §8) or via the examples loader UI component.
 */
export declare const EXAMPLES: readonly ExampleEntry[];
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
export declare function getExample(id: string): ExampleEntry | undefined;
/**
 * Get all available example entries.
 *
 * Returns the frozen examples array. Do not mutate.
 */
export declare function getExampleList(): readonly ExampleEntry[];
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
export declare function getExamplesByTag(): Map<string, string[]>;
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
export declare function getFixtureMap(): Map<string, string>;
//# sourceMappingURL=examples.d.ts.map