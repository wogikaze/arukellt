# ADR-021: Playground Share URL Format — Encoding, Versioning, and Round-Trip Contract

ステータス: **DECIDED** — fragmentベースのshare URL形式（versioned path structure）
**Created**: 2026-05-15
**Scope**: Playground (web), share/permalink surface, `docs/adr/`, static hosting contract

---

## Context

ADR-017 established the playground v1 product contract. Share/permalink is explicitly
within v1 scope and must work with **static hosting only** — no server-side storage, no
database, no authentication, no user accounts. The playground runs entirely client-side
(parse, format, check, diagnostics via Wasm in browser), and sharing must follow the same
constraint: all shared state must be encoded **in the URL itself**.

The playground needs shareable URLs that contain:

- **Source code** — the user's Arukellt program text
- **Compiler version** — the version of the Wasm-compiled frontend bundle
- **Example ID** — optional reference to a curated example from the example set
- **Feature flags** — optional playground-level settings (e.g., diagnostic verbosity)

Comparable systems use various approaches:

| System | Approach | Tradeoffs |
|--------|----------|-----------|
| Rust Playground | Server-stored gist | Requires backend, rate limiting, abuse mitigation |
| Go Playground | Server-stored snippet | Requires backend, storage infrastructure |
| TypeScript Playground | URL hash + LZ-String | Client-only, URL length constrained, no backend |
| Compiler Explorer | URL hash + base64 or short-link server | Hybrid; short links require backend |

Since v1 has no backend (ADR-017), the TypeScript Playground approach — encoding all state
in the URL fragment — is the natural fit. This ADR specifies the exact format, encoding
pipeline, compression strategy, length limits, fallback behavior, and round-trip contract.

### Design Constraints

1. **No server required** — must work with any static file host (GitHub Pages, Netlify, S3).
2. **Fragment-only** — shared state lives in the URL fragment (`#`), not query string (`?`),
   so it is never sent to the server in HTTP requests (privacy by default).
3. **Forward compatible** — the format must survive future schema additions without breaking
   old URLs.
4. **Round-trip lossless** — `decode(encode(state)) ≡ state` for all valid inputs.
5. **Reasonable URL length** — must work in all major browsers and common sharing contexts
   (chat, email, issue trackers).

---

## Decision

### 1. URL Structure

Share URLs use the fragment portion of the playground URL with a versioned path structure:

```
<base-url>/playground#share/<format-version>/<payload>
```

**Example:**

```
https://arukellt.dev/playground#share/1/eNpLSS0u0c1IzcnJVyjPL8pJUQQALLwF5Q
```

Components:

| Component | Description |
|-----------|-------------|
| `<base-url>/playground` | Playground page URL (host-dependent) |
| `#share/` | Fragment prefix identifying a share link |
| `<format-version>` | Integer schema version (currently `1`) |
| `<payload>` | Compressed and encoded state (see §2–§4) |

The `#share/` prefix distinguishes share URLs from other fragment uses (e.g., `#example/hello`
for loading a curated example by ID, or future fragment-based navigation). The playground
router inspects the fragment prefix to determine the action.

### 2. Payload Encoding Pipeline

The encoding pipeline transforms playground state into a URL-safe string:

```
   PlaygroundState (object)
        │
        ▼
   JSON.stringify()          →  UTF-8 JSON string
        │
        ▼
   deflate (raw, no header)  →  compressed bytes
        │
        ▼
   base64url encode          →  URL-safe ASCII string
        │
        ▼
   Append to fragment        →  #share/1/<payload>
```

Decoding is the exact reverse:

```
   Fragment payload string
        │
        ▼
   base64url decode          →  compressed bytes
        │
        ▼
   inflate (raw)             →  UTF-8 JSON string
        │
        ▼
   JSON.parse()              →  PlaygroundState (object)
        │
        ▼
   Validate against schema   →  Validated state or error
```

#### 2.1 JSON Serialization

Playground state is serialized as a JSON object. Keys are serialized in a **canonical order**
(alphabetical) to ensure deterministic output — the same logical state always produces the
same URL. Implementations MUST sort keys before serialization.

#### 2.2 Compression: Raw DEFLATE (RFC 1951)

Compression uses **raw DEFLATE** (RFC 1951) — the DEFLATE algorithm without any wrapper
(no zlib header, no gzip header). This is the most compact representation and avoids the
2–6 byte overhead of wrapper formats.

**Rationale for DEFLATE over alternatives:**

| Option | Pros | Cons | Decision |
|--------|------|------|----------|
| Raw DEFLATE + base64url | Standard algorithm, excellent browser support (pako, fflate), good compression on text, well-understood | Requires base64url wrapping (33% size expansion) | **✅ Chosen** |
| LZ-String | Designed for URL storage, produces URL-safe output directly | Non-standard algorithm, single-maintainer JS library, no native browser API, compression ratio inferior to DEFLATE on structured text | ❌ Rejected |
| Brotli | Best compression ratio | No `CompressionStream` in all target browsers as of 2026, larger decompressor library | ❌ Rejected for v1 |
| No compression | Simplest | URLs become impractically long for any non-trivial program (>30 lines) | ❌ Rejected |

**Implementation note:** In browsers, the `CompressionStream` / `DecompressionStream` API
supports `"deflate-raw"` format (raw DEFLATE without wrapper). When unavailable, the `pako`
or `fflate` library provides the same algorithm. Implementations MUST use raw DEFLATE
(`deflate-raw` / `pako.deflateRaw` / `fflate.deflateSync`), NOT `deflate` (zlib-wrapped)
or `gzip`.

#### 2.3 Base64url Encoding (RFC 4648 §5)

The compressed bytes are encoded using **base64url** (RFC 4648 §5):

- Alphabet: `A-Z a-z 0-9 - _` (replacing `+` and `/` from standard base64)
- **No padding** (`=` characters are omitted)
- Result is URL-safe without percent-encoding

This is the same encoding used by JWT, WebAuthn, and other URL-embedded binary data formats.

### 3. Payload Schema (Version 1)

The JSON payload for format version `1` has the following schema:

```json
{
  "src": "<string>",
  "ver": "<string>",
  "ex":  "<string>",
  "f":   { "<key>": <value>, ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `src` | string | **yes** | Source code text (UTF-8). May be empty string but must be present. |
| `ver` | string | no | Compiler/frontend version that produced this share link (semver, e.g. `"0.1.0"`). |
| `ex` | string | no | Example ID. If present, indicates the source was loaded from a curated example. Value matches the example's slug (e.g. `"hello-world"`, `"fibonacci"`). |
| `f` | object | no | Feature flags. Keys are flag names (lowercase kebab-case), values are booleans or strings. Unknown flags are preserved on decode but ignored by the playground. |

**Field name rationale:** Short field names (`src`, `ver`, `ex`, `f`) reduce JSON size
before compression. Since these URLs are machine-generated and machine-parsed, readability
of the raw JSON is not a priority.

#### 3.1 Minimal Payload Example

The smallest valid share payload (empty program, no optional fields):

```json
{"src":""}
```

Compressed and encoded, this produces a share URL fragment of approximately 20 characters.

#### 3.2 Typical Payload Example

```json
{"ex":"hello-world","f":{"diag-verbose":true},"src":"fn main() {\n  println(\"Hello, world!\")\n}","ver":"0.1.0"}
```

Note: Keys are in alphabetical order (canonical form).

#### 3.3 Unknown Fields

Decoders MUST preserve unknown top-level fields when re-encoding. This allows future schema
versions to add fields that older playground versions carry through without loss. Decoders
MUST NOT reject payloads containing unknown fields.

### 4. Version Pinning

The `ver` field records which version of the Arukellt frontend (parser, typechecker, formatter)
generated the share link. This serves two purposes:

1. **Diagnostic context** — when a user reports a bug via a share link, the version identifies
   which compiler produced the diagnostics they saw.
2. **Future compatibility** — if the language semantics change between versions, the playground
   can display a notice: "This snippet was created with version X; you are running version Y."

#### 4.1 Version String Format

The `ver` field uses **semver** format (`MAJOR.MINOR.PATCH`, e.g. `"0.1.0"`), matching the
version of the `ark-parser` / playground Wasm bundle. Pre-release suffixes (e.g. `"0.2.0-dev"`)
are allowed.

#### 4.2 Version Mismatch Behavior

When the playground decodes a share URL with a `ver` different from the running version:

- The source code is loaded as-is (no transformation).
- The playground MAY display an informational banner: _"This snippet was shared from
  version X.Y.Z. You are viewing it with version A.B.C. Behavior may differ."_
- The playground MUST NOT refuse to load the snippet.
- Re-sharing the loaded snippet updates `ver` to the current version.

#### 4.3 Absent Version

If `ver` is omitted, the playground treats the snippet as version-unspecified. No banner is
shown. This is the expected case for manually constructed or v1-era URLs.

### 5. URL Length Limits and Fallback Strategy

#### 5.1 Target Length Budget

| Component | Budget |
|-----------|--------|
| Base URL + path | ~40 characters |
| Fragment prefix (`#share/1/`) | 9 characters |
| Payload | remaining |
| **Total URL target** | **≤ 8,192 characters** |

This leaves approximately **8,143 characters** for the base64url payload, which decodes to
approximately **6,107 bytes** of compressed data. With DEFLATE achieving ~40–60% compression
on typical source code, this supports roughly **10,000–15,000 characters** of source code.

#### 5.2 Browser and Platform Limits

| Platform | Practical URL limit | Status |
|----------|-------------------|--------|
| Chrome / Edge | ~2 MB in address bar | ✅ Well within budget |
| Firefox | ~65,536 characters | ✅ Well within budget |
| Safari | ~80,000 characters | ✅ Well within budget |
| Twitter / X | URLs shortened, fragment preserved in expanded form | ✅ Works |
| GitHub Issues / Markdown | No practical limit on link `href` | ✅ Works |
| Slack | Truncates display at ~1,000 chars but preserves full URL in link | ⚠️ Display-truncated but functional |
| Email clients | Varies; some wrap at 2,083 chars (legacy IE limit) | ⚠️ May break in some clients |

The 8,192-character target ensures compatibility with nearly all sharing contexts.

#### 5.3 Fallback When URL Exceeds Limit

When the encoded URL exceeds 8,192 characters (source code is very large):

1. **Warn the user** — display a message: _"This snippet is too large to share via URL
   (N characters). Consider shortening the code."_
2. **Still generate the URL** — the URL is produced and placed in the address bar. It will
   work in most browsers but may fail in some sharing contexts.
3. **Offer download** — provide a "Download as .ark file" button as an alternative sharing
   mechanism. The downloaded file contains the raw source code; metadata (version, flags)
   is included as a comment header.
4. **Do NOT silently truncate** — the source code is never truncated to fit the URL budget.
   The round-trip contract (§6) must hold or the URL must not be generated.

#### 5.4 Hard Limit

URLs exceeding **65,536 characters** (Firefox's limit) MUST NOT be generated. The playground
displays an error and offers only the file download fallback.

### 6. Round-Trip Contract

The fundamental invariant of the share format:

```
∀ state ∈ ValidPlaygroundState:
    decode(encode(state)) = state
```

Formally:

1. **Encode** transforms a `PlaygroundState` into a URL fragment string.
2. **Decode** transforms a URL fragment string back into a `PlaygroundState`.
3. For any valid state, encoding and then decoding MUST produce a state that is
   **semantically identical** to the original.

#### 6.1 Semantic Identity

Two states are semantically identical if and only if:

- `src` fields are byte-identical (UTF-8).
- `ver` fields are identical strings, or both absent.
- `ex` fields are identical strings, or both absent.
- `f` fields contain the same key-value pairs (order-independent), or both absent/empty.

#### 6.2 Canonical Encoding

Because JSON keys are serialized in alphabetical order (§2.1), the encoding is
**deterministic**: the same logical state always produces the same URL. This means:

```
∀ state: encode(state₁) = encode(state₂)  ⟺  state₁ = state₂
```

This property enables URL comparison as a proxy for state comparison.

#### 6.3 Decode Error Handling

If decoding fails at any stage (invalid base64url, decompression error, malformed JSON,
missing required `src` field), the playground:

- Loads the default state (empty editor or default example).
- Displays an error banner: _"Could not load shared snippet. The link may be corrupted
  or from an incompatible version."_
- Does NOT crash or show a blank page.

#### 6.4 Test Contract

When the share feature is implemented, the following round-trip tests MUST pass:

| Test case | Input `src` | Validates |
|-----------|-------------|-----------|
| Empty string | `""` | Minimal payload |
| ASCII-only | `"fn main() {}"` | Basic round-trip |
| Unicode | `"// こんにちは\nfn main() {}"` | UTF-8 preservation |
| Large program | 10,000 characters of valid Arukellt | Compression under URL limit |
| All optional fields | `src` + `ver` + `ex` + `f` | Full schema round-trip |
| Unknown fields | Payload with extra field `"x": 42` | Forward compatibility preservation |
| Special JSON chars | Source with `"`, `\`, `\n`, `\t`, `\u0000` | JSON escaping correctness |

These tests are specified here as a contract; implementation is a separate work order.

### 7. Forward Compatibility

#### 7.1 Format Version Progression

The `<format-version>` in the URL (`#share/<version>/...`) is incremented only when the
encoding pipeline changes in a backward-incompatible way:

| Change type | Version bump? | Example |
|-------------|--------------|---------|
| New optional JSON field | **No** (handled by §3.3) | Adding `"theme": "dark"` |
| New required JSON field | **Yes** | Making `"ver"` mandatory |
| Different compression algorithm | **Yes** | Switching from DEFLATE to Brotli |
| Different base encoding | **Yes** | Switching from base64url to base45 |
| Removing a field | **No** (decoders tolerate absent optional fields) | Removing `"ex"` |

#### 7.2 Multi-Version Decode Support

The playground MUST support decoding **all prior format versions**. When the format version
is incremented, the decoder retains the old version's decode path. The version integer in
the URL makes dispatch trivial:

```
switch (version) {
  case 1: return decodeV1(payload);
  case 2: return decodeV2(payload);
  default: return { error: "Unsupported share format version" };
}
```

#### 7.3 Encoding Always Uses Latest Version

The encoder always produces URLs with the **latest format version**. There is no mechanism
to produce old-format URLs. Re-sharing a decoded old-format URL produces a new-format URL.

### 8. Fragment Namespace

The playground URL fragment is partitioned by prefix:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `#share/<v>/` | Share/permalink (this ADR) | `#share/1/eNpLSS0u...` |
| `#example/<id>` | Load curated example by ID | `#example/hello-world` |
| _(no fragment)_ | Default state (empty editor) | `/playground` |

Future prefixes may be added (e.g., `#tutorial/`, `#diff/`). The playground router MUST
treat any unrecognized prefix as a no-op (load default state) rather than an error.

---

## Consequences

1. Share/permalink works on **any static host** — no backend, no database, no API keys.
2. URLs are **privacy-preserving** — the fragment is never sent to the server in HTTP requests.
3. The format supports **~10,000–15,000 characters** of source code within the 8,192-character
   URL budget, sufficient for playground-sized snippets.
4. **Forward compatibility** is built in: new optional fields can be added to the payload
   without incrementing the format version; old URLs remain decodable.
5. **Version pinning** enables diagnostic context and future compatibility notices.
6. The **round-trip contract** and test cases (§6.4) define acceptance criteria for the
   implementation work order.
7. Implementation requires a DEFLATE library in the browser bundle (e.g., `pako`, `fflate`,
   or native `CompressionStream` with `"deflate-raw"`). Library choice is an implementation
   decision, not specified by this ADR.
8. `docs/adr/README.md` is regenerated by `python3 scripts/gen/generate-docs.py` to include
   this entry.

---

## Alternatives Considered

### A. Server-Stored Snippets (Gist / Database)

**Approach:** POST source code to a server, receive a short ID, share URL contains only the ID.

**Rejected for v1:**
- ADR-017 explicitly requires no backend for v1.
- Requires server infrastructure, rate limiting, abuse mitigation, storage costs.
- Introduces a dependency on server availability for link resolution.
- Can be added as an **optional enhancement in v2** alongside the URL-encoded format
  (e.g., short URLs for large programs that exceed the URL length budget).

### B. LZ-String Encoding

**Approach:** Use the `lz-string` library's `compressToEncodedURIComponent()` function,
which produces URL-safe output without separate base64url encoding.

**Rejected:**
- `lz-string` is a single-maintainer library with a non-standard compression algorithm.
- No native browser API equivalent — always requires a JS library dependency.
- Compression ratio is inferior to DEFLATE on structured text (typical source code).
- DEFLATE has decades of standardization, multiple implementations, and native browser
  support via `CompressionStream`.

### C. Brotli Compression

**Approach:** Use Brotli (RFC 7932) for better compression ratio than DEFLATE.

**Rejected for v1:**
- `CompressionStream("br")` is not available in all target browsers as of 2026-04.
- Requires bundling a Brotli decompressor library (~30 KB), larger than DEFLATE libraries.
- The compression ratio advantage (~10–15% better than DEFLATE) does not justify the
  compatibility risk for v1.
- Can be introduced as format version 2 when browser support is universal.

### D. Query String Instead of Fragment

**Approach:** Store share data in `?share=...` query parameter.

**Rejected:**
- Query strings are sent to the server in HTTP requests — leaks source code to access logs,
  CDN logs, and analytics.
- Some CDNs and proxies have lower URL length limits for the query string portion.
- Fragment (`#`) is the standard location for client-only state in single-page applications.

### E. Uncompressed Base64url

**Approach:** Skip compression, encode JSON directly as base64url.

**Rejected:**
- A 100-line program (~2,000 characters of source) produces ~2,700 characters of base64url
  without compression vs ~1,100 characters with DEFLATE. The 60% size reduction from
  compression is essential for staying within URL length budgets.

---

## References

- [ADR-017: Playground Execution Model](ADR-017-playground-execution-model.md) — v1 product contract, share/permalink in scope
- [ADR-019: Anchor / Permalink Policy](ADR-019-anchor-permalink-policy.md) — naming conventions for doc anchors (orthogonal to this ADR)
- [ADR-020: T2 I/O Surface](ADR-020-t2-io-surface.md) — v2 execution target (not required for share format)
- [RFC 1951: DEFLATE Compressed Data Format](https://datatracker.ietf.org/doc/html/rfc1951)
- [RFC 4648 §5: Base64url Encoding](https://datatracker.ietf.org/doc/html/rfc4648#section-5)
- [TypeScript Playground — URL sharing implementation](https://www.typescriptlang.org/play) (prior art)
- [pako — zlib port to JavaScript](https://github.com/nicepkg/pako) (reference DEFLATE implementation)
- [fflate — fast JS compression library](https://github.com/nicepkg/fflate) (alternative DEFLATE implementation)
