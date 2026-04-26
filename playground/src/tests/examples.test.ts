/**
 * Tests for the examples catalog module.
 *
 * Verifies the examples catalog structure, lookup functions,
 * fixture references, and data integrity.
 *
 * @module
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  EXAMPLES,
  FIXTURE_BASE_PATH,
  getExample,
  getExampleList,
  getExamplesByTag,
  getFixtureMap,
} from "../examples.js";
import type { ExampleEntry } from "../examples.js";

// ---------------------------------------------------------------------------
// Catalog structure
// ---------------------------------------------------------------------------

describe("Examples catalog structure", () => {
  it("has at least one example", () => {
    assert.ok(EXAMPLES.length > 0, "catalog should not be empty");
  });

  it("all examples have required fields", () => {
    for (const ex of EXAMPLES) {
      assert.ok(typeof ex.id === "string" && ex.id.length > 0, `id: ${ex.id}`);
      assert.ok(typeof ex.name === "string" && ex.name.length > 0, `name: ${ex.name}`);
      assert.ok(
        typeof ex.description === "string" && ex.description.length > 0,
        `description for ${ex.id}`,
      );
      assert.ok(
        typeof ex.source === "string" && ex.source.length > 0,
        `source for ${ex.id}`,
      );
    }
  });

  it("all IDs are unique", () => {
    const ids = EXAMPLES.map((e) => e.id);
    const unique = new Set(ids);
    assert.equal(unique.size, ids.length, "duplicate IDs detected");
  });

  it("all IDs are kebab-case", () => {
    const kebabRe = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;
    for (const ex of EXAMPLES) {
      assert.ok(
        kebabRe.test(ex.id),
        `ID "${ex.id}" should be kebab-case`,
      );
    }
  });

  it("all sources end with newline", () => {
    for (const ex of EXAMPLES) {
      assert.ok(
        ex.source.endsWith("\n"),
        `Source for "${ex.id}" should end with newline`,
      );
    }
  });

  it("tags are arrays of non-empty strings (when present)", () => {
    for (const ex of EXAMPLES) {
      if (ex.tags !== undefined) {
        assert.ok(Array.isArray(ex.tags), `tags for ${ex.id} should be array`);
        for (const tag of ex.tags) {
          assert.ok(
            typeof tag === "string" && tag.length > 0,
            `tag in ${ex.id} should be non-empty string`,
          );
        }
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Known examples
// ---------------------------------------------------------------------------

describe("Known examples", () => {
  it("includes hello-world", () => {
    const hw = EXAMPLES.find((e) => e.id === "hello-world");
    assert.ok(hw, "hello-world should exist");
    assert.ok(hw!.source.includes("main"), "hello-world should have main fn");
  });

  it("includes fibonacci", () => {
    const fib = EXAMPLES.find((e) => e.id === "fibonacci");
    assert.ok(fib, "fibonacci should exist");
    assert.ok(fib!.source.includes("fib"), "fibonacci should reference fib");
  });
});

// ---------------------------------------------------------------------------
// getExample
// ---------------------------------------------------------------------------

describe("getExample", () => {
  it("returns example for valid ID", () => {
    const ex = getExample("hello-world");
    assert.ok(ex);
    assert.equal(ex!.id, "hello-world");
  });

  it("returns undefined for invalid ID", () => {
    const ex = getExample("nonexistent-example");
    assert.equal(ex, undefined);
  });

  it("returns undefined for empty string", () => {
    const ex = getExample("");
    assert.equal(ex, undefined);
  });

  it("is case-sensitive", () => {
    const ex = getExample("Hello-World");
    assert.equal(ex, undefined, "lookup should be case-sensitive");
  });
});

// ---------------------------------------------------------------------------
// getExampleList
// ---------------------------------------------------------------------------

describe("getExampleList", () => {
  it("returns all examples", () => {
    const list = getExampleList();
    assert.equal(list.length, EXAMPLES.length);
  });

  it("returns the same reference as EXAMPLES", () => {
    const list = getExampleList();
    assert.equal(list, EXAMPLES);
  });
});

// ---------------------------------------------------------------------------
// getExamplesByTag
// ---------------------------------------------------------------------------

describe("getExamplesByTag", () => {
  it("returns a Map", () => {
    const byTag = getExamplesByTag();
    assert.ok(byTag instanceof Map);
  });

  it("groups examples correctly", () => {
    const byTag = getExamplesByTag();

    // "basics" tag should include hello-world, variables, functions
    const basics = byTag.get("basics");
    assert.ok(basics, '"basics" tag should exist');
    assert.ok(basics!.includes("hello-world"));
    assert.ok(basics!.includes("variables"));
    assert.ok(basics!.includes("functions"));
  });

  it("includes all tagged examples", () => {
    const byTag = getExamplesByTag();
    const allTaggedIds = new Set<string>();
    for (const ids of byTag.values()) {
      for (const id of ids) {
        allTaggedIds.add(id);
      }
    }

    // Every example with tags should appear
    for (const ex of EXAMPLES) {
      if (ex.tags && ex.tags.length > 0) {
        assert.ok(
          allTaggedIds.has(ex.id),
          `${ex.id} should appear in tag map`,
        );
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Fixture references
// ---------------------------------------------------------------------------

describe("Fixture references", () => {
  it("every example has a fixturePath", () => {
    for (const ex of EXAMPLES) {
      assert.ok(
        typeof ex.fixturePath === "string" && ex.fixturePath.length > 0,
        `${ex.id} should have a non-empty fixturePath`,
      );
    }
  });

  it("all fixture paths end with .ark", () => {
    for (const ex of EXAMPLES) {
      assert.ok(
        ex.fixturePath.endsWith(".ark"),
        `fixturePath for "${ex.id}" should end with .ark, got "${ex.fixturePath}"`,
      );
    }
  });

  it("all fixture paths are unique", () => {
    const paths = EXAMPLES.map((e) => e.fixturePath);
    const unique = new Set(paths);
    assert.equal(
      unique.size,
      paths.length,
      "duplicate fixture paths detected",
    );
  });

  it("fixture paths do not start with /", () => {
    for (const ex of EXAMPLES) {
      assert.ok(
        !ex.fixturePath.startsWith("/"),
        `fixturePath for "${ex.id}" should be relative, got "${ex.fixturePath}"`,
      );
    }
  });

  it("FIXTURE_BASE_PATH is tests/fixtures", () => {
    assert.equal(FIXTURE_BASE_PATH, "tests/fixtures");
  });
});

// ---------------------------------------------------------------------------
// getFixtureMap
// ---------------------------------------------------------------------------

describe("getFixtureMap", () => {
  it("returns a Map", () => {
    const map = getFixtureMap();
    assert.ok(map instanceof Map);
  });

  it("has one entry per example", () => {
    const map = getFixtureMap();
    assert.equal(map.size, EXAMPLES.length);
  });

  it("maps example IDs to fixture paths", () => {
    const map = getFixtureMap();
    for (const ex of EXAMPLES) {
      assert.equal(
        map.get(ex.id),
        ex.fixturePath,
        `fixture map for "${ex.id}" should match fixturePath`,
      );
    }
  });

  it("hello-world maps to hello/hello.ark", () => {
    const map = getFixtureMap();
    assert.equal(map.get("hello-world"), "hello/hello.ark");
  });
});
