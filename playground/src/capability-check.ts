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

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Identifier for an unsupported capability category. */
export type CapabilityId =
  | "wasi-host"
  | "file-io"
  | "network"
  | "process-env";

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

// ---------------------------------------------------------------------------
// Capability registry
// ---------------------------------------------------------------------------

/** WASI host calls — no WASI runtime in the browser. */
const WASI_HOST: CapabilityInfo = {
  id: "wasi-host",
  name: "WASI Host Calls",
  reason:
    "The playground runs in a browser sandbox with no WASI runtime. " +
    "Host-level system calls (std::host) are not available.",
};

/** File system access — no file system in the browser sandbox. */
const FILE_IO: CapabilityInfo = {
  id: "file-io",
  name: "File System Access",
  reason:
    "The playground runs in a browser sandbox without file system access. " +
    "File I/O operations (std::fs) cannot be executed here.",
};

/** Network access — no direct network in the browser sandbox. */
const NETWORK: CapabilityInfo = {
  id: "network",
  name: "Network Access",
  reason:
    "The playground runs in a browser sandbox without direct network access. " +
    "Network and HTTP operations (std::net, std::http) are not available.",
};

/** Process environment — no OS environment in the browser sandbox. */
const PROCESS_ENV: CapabilityInfo = {
  id: "process-env",
  name: "Process Environment",
  reason:
    "The playground runs in a browser sandbox without access to " +
    "process environment variables or OS-level configuration.",
};

// ---------------------------------------------------------------------------
// Pattern definitions
// ---------------------------------------------------------------------------

/**
 * A detection rule: a regex pattern and the capability it indicates.
 * @internal
 */
interface DetectionRule {
  /** Pattern to match against source text. Must use the `g` flag. */
  readonly pattern: RegExp;
  /** The unsupported capability this pattern indicates. */
  readonly capability: CapabilityInfo;
  /** Template for the user-facing message. `{match}` is replaced with the matched text. */
  readonly messageTemplate: string;
}

/**
 * All detection rules, ordered by specificity (most specific first).
 *
 * More specific patterns (e.g., `std::host::env`) are listed before
 * general patterns (e.g., `import host`) so that when the same source
 * region matches multiple rules, the most informative warning wins.
 *
 * @internal
 */
const DETECTION_RULES: readonly DetectionRule[] = [
  // --- File I/O (most specific first) ---
  {
    pattern: /\bstd::fs::\w+/g,
    capability: FILE_IO,
    messageTemplate:
      "'{match}' requires file system access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bhost::fs::\w+/g,
    capability: FILE_IO,
    messageTemplate:
      "'{match}' requires file system access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bimport\s+fs\b/g,
    capability: FILE_IO,
    messageTemplate:
      "The 'fs' module requires file system access, which is unavailable in the browser sandbox.",
  },

  // --- Network ---
  {
    pattern: /\bstd::net::\w+/g,
    capability: NETWORK,
    messageTemplate:
      "'{match}' requires network access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bstd::http::\w+/g,
    capability: NETWORK,
    messageTemplate:
      "'{match}' requires network access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bhost::http::\w+/g,
    capability: NETWORK,
    messageTemplate:
      "'{match}' requires network access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bhost::net::\w+/g,
    capability: NETWORK,
    messageTemplate:
      "'{match}' requires network access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bimport\s+net\b/g,
    capability: NETWORK,
    messageTemplate:
      "The 'net' module requires network access, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bimport\s+http\b/g,
    capability: NETWORK,
    messageTemplate:
      "The 'http' module requires network access, which is unavailable in the browser sandbox.",
  },

  // --- Process environment ---
  {
    pattern: /\bstd::env::\w+/g,
    capability: PROCESS_ENV,
    messageTemplate:
      "'{match}' requires access to process environment variables, " +
      "which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bhost::env::\w+/g,
    capability: PROCESS_ENV,
    messageTemplate:
      "'{match}' requires access to process environment variables, " +
      "which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bimport\s+env\b/g,
    capability: PROCESS_ENV,
    messageTemplate:
      "The 'env' module requires access to process environment variables, " +
      "which is unavailable in the browser sandbox.",
  },

  // --- General WASI host (broadest patterns last) ---
  {
    pattern: /\bstd::host::\w+/g,
    capability: WASI_HOST,
    messageTemplate:
      "'{match}' requires a WASI host runtime, which is unavailable in the browser sandbox.",
  },
  {
    pattern: /\bimport\s+host\b/g,
    capability: WASI_HOST,
    messageTemplate:
      "The 'host' module requires a WASI host runtime, which is unavailable in the browser sandbox.",
  },
];

// ---------------------------------------------------------------------------
// Detection logic
// ---------------------------------------------------------------------------

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
export function checkCapabilities(source: string): CapabilityWarning[] {
  if (source.length === 0) {
    return [];
  }

  const warnings: CapabilityWarning[] = [];

  // Track covered byte ranges to avoid duplicate warnings when a
  // specific rule (e.g., host::env::get) and a general rule (e.g.,
  // import host) both match overlapping regions.
  const covered = new Set<string>();

  for (const rule of DETECTION_RULES) {
    // Reset regex state for each rule (global regexes are stateful).
    rule.pattern.lastIndex = 0;

    let match: RegExpExecArray | null;
    while ((match = rule.pattern.exec(source)) !== null) {
      const start = match.index;
      const end = start + match[0].length;
      const key = `${start}:${end}`;

      // Skip if a more-specific rule already covered this exact range.
      if (covered.has(key)) {
        continue;
      }

      // Check for overlap with any already-covered range that fully
      // contains this match or is fully contained by it.
      let overlaps = false;
      for (const existing of covered) {
        const [eStart, eEnd] = existing.split(":").map(Number);
        // Skip if an existing range overlaps with this one.
        if (eStart <= start && eEnd >= end) {
          overlaps = true;
          break;
        }
      }
      if (overlaps) {
        continue;
      }

      covered.add(key);

      const matchText = match[0];
      const message = rule.messageTemplate.replace("{match}", matchText);

      warnings.push({
        capability: rule.capability,
        start,
        end,
        matchText,
        message,
      });
    }
  }

  // Sort by source position for stable ordering.
  warnings.sort((a, b) => a.start - b.start);

  return warnings;
}

// ---------------------------------------------------------------------------
// Diagnostic conversion
// ---------------------------------------------------------------------------

/** Synthetic diagnostic code used for capability warnings. */
const CAPABILITY_WARNING_CODE = "W9000";

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
export function capabilityWarningsToDiagnostics(
  warnings: CapabilityWarning[],
): Diagnostic[] {
  return warnings.map((w) => ({
    code: CAPABILITY_WARNING_CODE,
    severity: "warning" as const,
    phase: "parse" as const,
    message: w.message,
    labels: [
      {
        file_id: 0,
        start: w.start,
        end: w.end,
        message: `unsupported in playground: ${w.capability.name}`,
      },
    ],
    notes: [w.capability.reason],
    suggestion:
      "This code can be parsed and type-checked, but cannot be " +
      "executed in the playground. Use a local Arukellt installation " +
      "to run programs that require host capabilities.",
  }));
}

// ---------------------------------------------------------------------------
// Capability info access
// ---------------------------------------------------------------------------

/**
 * All unsupported capability categories in the playground.
 *
 * Useful for building help text or capability documentation in the UI.
 */
export const UNSUPPORTED_CAPABILITIES: readonly CapabilityInfo[] = [
  WASI_HOST,
  FILE_IO,
  NETWORK,
  PROCESS_ENV,
];

/**
 * Look up a capability by its identifier.
 *
 * @param id - The capability identifier (e.g., `"file-io"`).
 * @returns The capability info, or `undefined` if not found.
 */
export function getCapabilityInfo(
  id: CapabilityId,
): CapabilityInfo | undefined {
  return UNSUPPORTED_CAPABILITIES.find((c) => c.id === id);
}
