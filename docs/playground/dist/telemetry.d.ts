/**
 * Playground telemetry and error-reporting guardrails.
 *
 * ## v1 Policy (enforced here)
 *
 * Zero telemetry. Zero outbound network requests from the application.
 * All errors are reported locally to the browser developer console only.
 *
 * ## v2+ Extension point
 *
 * If analytics or error aggregation are added in a future version, they
 * MUST be opt-in, default-OFF, and must comply with the full policy before
 * being shipped.  This module is the single hook point — update these
 * functions rather than scattering new reporting calls across the codebase.
 *
 * Full policy: `docs/playground/privacy-telemetry-policy.md`
 *
 * @module
 */
/**
 * v1 compile-time constant: telemetry is **disabled**.
 *
 * No analytics scripts are loaded. No events are sent to any endpoint.
 * Any code that conditionally collects data MUST read this flag and MUST
 * NOT collect when it is `true`.
 *
 * To introduce telemetry in a future version:
 * 1. Update `docs/playground/privacy-telemetry-policy.md` first.
 * 2. Change this to a runtime-readable preference (not a build constant).
 * 3. Default the preference to `false` (opt-in).
 */
export declare const TELEMETRY_DISABLED: true;
/**
 * Report an application error to the browser developer console.
 *
 * In v1 this is the sole error-reporting channel: no data leaves the
 * browser. If client-side error aggregation is added in v2+ (opt-in,
 * see policy §4.2), extend this function — callers do not change.
 *
 * @param context - Human-readable description of where the error occurred.
 * @param error   - The caught error value.
 */
export declare function reportError(context: string, error: unknown): void;
/**
 * Report a Wasm load failure and return a user-facing message string.
 *
 * The error is written to the console; the returned string can be
 * displayed in the UI so users know what happened and what to do next.
 *
 * Example:
 * ```ts
 * try {
 *   await wasm.default(wasmUrl);
 * } catch (err) {
 *   const msg = reportWasmLoadError(err);
 *   showErrorBanner(msg);
 * }
 * ```
 *
 * @param error - The caught error value from the Wasm initialisation call.
 * @returns A user-facing error message string (never null).
 */
export declare function reportWasmLoadError(error: unknown): string;
/**
 * Report an unexpected compiler panic and return a user-facing message string.
 *
 * Use this when a Wasm call throws an unexpected exception (panic boundary).
 * The error is written to the console; the returned string should be shown
 * in the diagnostics area or a modal.
 *
 * @param error - The caught error value.
 * @returns A user-facing error message string (never null).
 */
export declare function reportCompilerPanic(error: unknown): string;
//# sourceMappingURL=telemetry.d.ts.map