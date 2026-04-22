/**
 * Share link encoding and decoding for the Arukellt playground.
 *
 * Implements ADR-021: Playground Share URL Format. Share URLs encode
 * playground state in the URL fragment using DEFLATE compression and
 * base64url encoding:
 *
 *     <base-url>/playground#share/<version>/<payload>
 *
 * Encoding pipeline (ADR-021 §2):
 *
 *     PlaygroundState → JSON.stringify (canonical keys) → UTF-8
 *       → deflate-raw → base64url → #share/1/<payload>
 *
 * Decoding is the exact reverse.
 *
 * @module
 */
/**
 * Playground state encoded in a share URL payload (ADR-021 §3).
 *
 * Unknown fields are preserved for forward compatibility (§3.3).
 */
export interface SharePayload {
    /** Source code text (UTF-8, required). May be empty string. */
    src: string;
    /** Compiler/frontend version (semver, optional). */
    ver?: string;
    /** Example ID slug (optional). */
    ex?: string;
    /** Feature flags — keys are kebab-case, values are booleans or strings (optional). */
    f?: Record<string, boolean | string>;
    /** Index signature for unknown fields (forward compatibility, §3.3). */
    [key: string]: unknown;
}
/** Result of encoding playground state into a share URL fragment. */
export interface ShareEncodeResult {
    /** The URL fragment string (e.g., `#share/1/eNpL...`). */
    fragment: string;
    /** Whether the fragment exceeds the target length budget (8,192 chars). */
    exceedsTarget: boolean;
    /** Whether the fragment exceeds the hard limit (65,536 chars) and must not be used. */
    exceedsHardLimit: boolean;
    /** Total fragment length in characters. */
    fragmentLength: number;
}
/** Result of decoding a share URL fragment. */
export type ShareDecodeResult = {
    ok: true;
    payload: SharePayload;
} | {
    ok: false;
    error: string;
};
/**
 * Parsed URL fragment action (ADR-021 §8).
 *
 * The playground router dispatches on the fragment prefix:
 * - `share` — decode a share link
 * - `example` — load a curated example by ID
 * - `none` — empty/absent fragment → default state
 * - `unknown` — unrecognized prefix → no-op (load default state)
 */
export type FragmentAction = {
    type: "share";
    version: number;
    encodedPayload: string;
} | {
    type: "example";
    id: string;
} | {
    type: "none";
} | {
    type: "unknown";
    prefix: string;
};
/**
 * Level of version mismatch between a share link and the running playground.
 *
 * - `"none"` — versions are identical.
 * - `"patch"` — only patch differs (bug fixes, unlikely to affect behavior).
 * - `"minor"` — minor version differs (new features, some behavior changes possible).
 * - `"major"` — major version differs (breaking changes likely).
 * - `"unknown"` — the share link has no `ver` field (version-unspecified, ADR-021 §4.3).
 * - `"prerelease"` — same major.minor.patch but pre-release suffixes differ.
 */
export type VersionMismatchLevel = "none" | "patch" | "minor" | "major" | "unknown" | "prerelease";
/**
 * Result of comparing a share link's version against the running playground version.
 *
 * Used to decide whether to display a version mismatch banner (ADR-021 §4.2).
 */
export interface VersionMismatchInfo {
    /** The mismatch level. `"none"` means versions match exactly. */
    level: VersionMismatchLevel;
    /** The version from the share link (`undefined` if absent). */
    linkVersion: string | undefined;
    /** The current running playground version. */
    currentVersion: string;
    /**
     * Human-readable message for display in a banner.
     *
     * - `null` when level is `"none"` or `"unknown"` (no banner needed).
     * - Non-null for `"patch"`, `"minor"`, `"major"`, and `"prerelease"` mismatches.
     *
     * Format: "This snippet was shared from version X.Y.Z. You are viewing
     * it with version A.B.C. Behavior may differ." (ADR-021 §4.2)
     */
    message: string | null;
}
/**
 * Encode playground state into a share URL fragment.
 *
 * Follows the ADR-021 §2 pipeline:
 *
 *     state → canonical JSON → UTF-8 → deflate-raw → base64url → fragment
 *
 * @param payload - The playground state to encode. Must include `src`.
 * @returns Encoded share fragment with length metadata.
 *
 * @example
 * ```ts
 * const result = await encodeSharePayload({ src: 'fn main() {}' });
 * // result.fragment === "#share/1/eNpL..."
 * window.location.hash = result.fragment.slice(1);
 * ```
 */
export declare function encodeSharePayload(payload: SharePayload): Promise<ShareEncodeResult>;
/**
 * Decode a share URL fragment back into playground state.
 *
 * Follows the ADR-021 §2 reverse pipeline:
 *
 *     fragment → base64url → inflate-raw → UTF-8 → JSON.parse → validate
 *
 * On failure, returns an error result instead of throwing (ADR-021 §6.3).
 *
 * @param fragment - The URL fragment string (with or without leading `#`).
 * @returns Decoded payload on success, or an error description.
 *
 * @example
 * ```ts
 * const result = await decodeSharePayload(window.location.hash);
 * if (result.ok) {
 *   editor.setValue(result.payload.src);
 * } else {
 *   showError(result.error);
 * }
 * ```
 */
export declare function decodeSharePayload(fragment: string): Promise<ShareDecodeResult>;
/**
 * Parse a URL fragment to determine the playground action.
 *
 * Inspects the fragment prefix to dispatch:
 * - `#share/<v>/<payload>` → share link (decode with {@link decodeSharePayload})
 * - `#example/<id>` → curated example (load with examples module)
 * - empty/absent → default state
 * - unrecognized → unknown (treated as no-op per ADR-021 §8)
 *
 * @param fragment - The URL fragment (with or without leading `#`).
 */
export declare function parseFragment(fragment: string): FragmentAction;
/** Current share format version used when encoding. */
export declare const CURRENT_SHARE_VERSION: number;
/** Target URL length budget in characters (ADR-021 §5.1). */
export declare const SHARE_URL_TARGET_LENGTH: number;
/** Hard URL length limit in characters (ADR-021 §5.4). */
export declare const SHARE_URL_HARD_LIMIT: number;
/**
 * Check for a version mismatch between a decoded share payload and the
 * currently running playground version (ADR-021 §4.2).
 *
 * Returns structured information about the mismatch level and a
 * human-readable message suitable for display in an informational banner.
 *
 * Per ADR-021 §4.2:
 * - The source code is always loaded as-is (no transformation).
 * - The playground MAY display a banner when versions differ.
 * - The playground MUST NOT refuse to load the snippet.
 * - Re-sharing updates `ver` to the current version.
 *
 * Per ADR-021 §4.3:
 * - If `ver` is absent, the snippet is treated as version-unspecified
 *   and no banner is shown.
 *
 * @param payload - The decoded share payload.
 * @param currentVersion - The currently running playground/compiler version (semver).
 * @returns Version mismatch information.
 *
 * @example
 * ```ts
 * const decoded = await decodeSharePayload(fragment);
 * if (decoded.ok) {
 *   const mismatch = checkVersionMismatch(decoded.payload, pg.version());
 *   if (mismatch.message) {
 *     showBanner(mismatch.message);
 *   }
 * }
 * ```
 */
export declare function checkVersionMismatch(payload: SharePayload, currentVersion: string): VersionMismatchInfo;
/**
 * Encode playground state with automatic version injection.
 *
 * Convenience wrapper around {@link encodeSharePayload} that automatically
 * sets the `ver` field to the provided compiler version. If the payload
 * already has a `ver` field, it is overwritten with the current version
 * (ADR-021 §4.2: re-sharing updates `ver` to the current version).
 *
 * @param payload - The playground state to encode. Must include `src`.
 * @param currentVersion - The current compiler/frontend version (semver).
 * @returns Encoded share fragment with length metadata.
 *
 * @example
 * ```ts
 * const result = await encodeSharePayloadWithVersion(
 *   { src: 'fn main() {}' },
 *   pg.version(),
 * );
 * window.location.hash = result.fragment.slice(1);
 * ```
 */
export declare function encodeSharePayloadWithVersion(payload: SharePayload, currentVersion: string): Promise<ShareEncodeResult>;
/**
 * Documents what is guaranteed when opening a share link across different
 * playground versions. This is a machine-readable contract that the
 * playground UI can display or test against.
 *
 * **Guaranteed across all versions:**
 * - Source code (`src`) is preserved byte-for-byte (UTF-8 round-trip lossless).
 * - Feature flags (`f`) are preserved (unknown flags are kept but may be ignored).
 * - Example ID (`ex`) is preserved.
 * - Unknown fields are preserved (forward compatibility, ADR-021 §3.3).
 *
 * **NOT guaranteed across versions:**
 * - Parse diagnostics may differ (new/removed errors, changed messages).
 * - Formatting output may differ (formatter improvements).
 * - Tokenization may produce different token streams.
 * - Feature flags may be interpreted differently or ignored.
 *
 * **Version mismatch behavior (ADR-021 §4.2):**
 * - Source code is always loaded — the playground never refuses to load.
 * - An informational banner MAY be shown for version mismatches.
 * - Re-sharing always updates `ver` to the current playground version.
 */
export declare const REPRODUCIBILITY_CONTRACT: {
    /** Fields whose values are preserved byte-for-byte across all versions. */
    readonly guaranteedFields: readonly string[];
    /** Behaviors that may change between versions. */
    readonly notGuaranteed: readonly string[];
    /** The playground MUST NOT refuse to load a share link due to version mismatch. */
    readonly alwaysLoads: true;
    /** Re-sharing a loaded snippet MUST update `ver` to the current version. */
    readonly reshareUpdatesVersion: true;
};
//# sourceMappingURL=share.d.ts.map