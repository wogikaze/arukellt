/**
 * Capability detection for the Arukellt playground.
 *
 * The playground runs entirely client-side in the browser (ADR-017).
 * There is no WASI host, no file system, no network, and no process
 * environment available. This module detects source-level usage of
 * features that require these unavailable host capabilities and
 * produces structured warnings so the UI can explain what is
 * unsupported and why.
 *
 * ## Design
 *
 * Detection operates on **source text** using pattern matching.
 * This avoids coupling to AST shape and works even when the source
 * has parse errors (the user might be mid-edit). Each pattern is
 * associated with a {@link CapabilityInfo} describing the missing
 * capability and why it is unavailable in the sandbox.
 *
 * Warnings can be converted to {@link Diagnostic} objects for seamless
 * display in the diagnostics panel alongside parse errors.
 *
 * @module
 */
import type { Diagnostic } from "./types.js";
/** Identifier for an unsupported capability category. */
export type CapabilityId = "wasi-host" | "file-io" | "network" | "process-env";
/** Description of an unsupported playground capability. */
export interface CapabilityInfo {
    /** Machine-readable identifier. */
    readonly id: CapabilityId;
    /** Human-readable capability name. */
    readonly name: string;
    /** Explanation of why this capability is unavailable in the playground. */
    readonly reason: string;
}
/** A detected usage of an unsupported capability in source code. */
export interface CapabilityWarning {
    /** The unsupported capability that was detected. */
    readonly capability: CapabilityInfo;
    /** Start byte offset of the match in source (0-based). */
    readonly start: number;
    /** End byte offset of the match in source (exclusive). */
    readonly end: number;
    /** The matched source text. */
    readonly matchText: string;
    /** User-facing warning message. */
    readonly message: string;
}
/**
 * Check source code for usage of unsupported playground capabilities.
 *
 * Scans the source text for patterns that indicate usage of host
 * capabilities (WASI, file I/O, network, environment variables) that
 * are not available in the browser sandbox.
 *
 * Returns an array of warnings, each describing what was detected,
 * where it was found, and why it is unsupported.
 *
 * @param source - The Arukellt source code to check.
 * @returns Array of capability warnings (may be empty).
 *
 * @example
 * ```ts
 * const warnings = checkCapabilities('import host\nhost::env::get("PATH")');
 * // warnings[0].capability.id === "wasi-host"
 * // warnings[1].capability.id === "process-env"
 * ```
 */
export declare function checkCapabilities(source: string): CapabilityWarning[];
/**
 * Convert capability warnings to {@link Diagnostic} objects.
 *
 * The returned diagnostics use severity `"warning"` and include
 * explanatory notes about why the capability is unavailable. They
 * can be merged with parse diagnostics for unified display in the
 * diagnostics panel and inline overlay.
 *
 * @param warnings - Capability warnings from {@link checkCapabilities}.
 * @returns Array of diagnostics suitable for display.
 *
 * @example
 * ```ts
 * const warnings = checkCapabilities(source);
 * const diags = capabilityWarningsToDiagnostics(warnings);
 * // Merge with parse diagnostics:
 * const all = [...parseResult.diagnostics, ...diags];
 * panel.update(all, source);
 * ```
 */
export declare function capabilityWarningsToDiagnostics(warnings: CapabilityWarning[]): Diagnostic[];
/**
 * All unsupported capability categories in the playground.
 *
 * Useful for building help text or capability documentation in the UI.
 */
export declare const UNSUPPORTED_CAPABILITIES: readonly CapabilityInfo[];
/**
 * Look up a capability by its identifier.
 *
 * @param id - The capability identifier (e.g., `"file-io"`).
 * @returns The capability info, or `undefined` if not found.
 */
export declare function getCapabilityInfo(id: CapabilityId): CapabilityInfo | undefined;
//# sourceMappingURL=capability-check.d.ts.map