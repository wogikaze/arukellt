/**
 * Integrated playground application component.
 *
 * Wires together the editor shell, parse API, and diagnostics panel into
 * a single cohesive component. Editing triggers re-parse and diagnostic
 * updates, including inline error markers in the editor.
 *
 * ## Architecture
 *
 * ```
 * ┌─────────────────────────────────────────────┐
 * │ .ark-playground-app                         │
 * │ ┌─────────────────────────────────────────┐ │
 * │ │ Editor (textarea + backdrop + overlay)  │ │
 * │ │   └── .ark-editor-diagnostics (markers) │ │
 * │ └─────────────────────────────────────────┘ │
 * │ ┌─────────────────────────────────────────┐ │
 * │ │ Diagnostics Panel (list of issues)      │ │
 * │ └─────────────────────────────────────────┘ │
 * └─────────────────────────────────────────────┘
 * ```
 *
 * The diagnostic overlay is a `<pre>` element inserted into the editor's
 * DOM between the backdrop and textarea. It shows transparent text with
 * wavy underlines at diagnostic label positions, perfectly aligned with
 * the editor text.
 *
 * @module
 */
import { createEditor } from "./editor.js";
import { createDiagnosticsPanel, buildDiagnosticOverlay, injectDiagnosticStyles, } from "./diagnostics.js";
import { checkCapabilities, capabilityWarningsToDiagnostics, } from "./capability-check.js";
import { isRunnableT2Output } from "./compiler-client.js";
import { buildStatusMessage, runStatusMessage, sectionsFromCompileResult, sectionsFromRunResult, } from "./console-bridge.js";
import { createRunOutputPanel, injectRunOutputStyles } from "./run-output.js";
import { createStdinPanel, injectStdinPanelStyles } from "./stdin-panel.js";
// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------
/**
 * Create an integrated playground application.
 *
 * Mounts an editor and diagnostics panel inside the given container,
 * wired together so that editing triggers re-parsing and updates both
 * the diagnostics panel and inline error markers.
 *
 * @param container - The DOM element to mount the application into.
 * @param options - Application configuration.
 * @returns A {@link PlaygroundApp} instance.
 *
 * @example
 * ```ts
 * import { createPlayground, createPlaygroundApp } from "@arukellt/playground";
 *
 * const pg = await createPlayground(wasmPath, { wasmUrl });
 * const app = createPlaygroundApp(document.getElementById("app")!, {
 *   initialValue: "fn main() {\n    let x = 42\n}\n",
 *   parse: (src) => pg.parse(src),
 *   tokenize: (src) => pg.tokenize(src),
 * });
 * ```
 */
export function createPlaygroundApp(container, options) {
    const { initialValue = "", parse, tokenize, parseDebounceMs = 300, highlightDebounceMs = 30, tabSize = 4, placeholder, readOnly = false, onDiagnostics, compilerClient: initialCompilerClient, } = options;
    // Inject diagnostic styles (idempotent).
    injectDiagnosticStyles();
    injectRunOutputStyles();
    injectStdinPanelStyles();
    // --- Layout ---
    const wrapper = document.createElement("div");
    wrapper.className = "ark-playground-app";
    container.appendChild(wrapper);
    const editorContainer = document.createElement("div");
    editorContainer.className = "ark-playground-editor-container";
    wrapper.appendChild(editorContainer);
    const stdinPanel = createStdinPanel(wrapper, { injectStyles: false });
    // --- Editor ---
    const editor = createEditor(editorContainer, {
        initialValue,
        tokenize,
        tabSize,
        debounceMs: highlightDebounceMs,
        placeholder,
        readOnly,
    });
    // --- Diagnostic overlay (inline markers in the editor) ---
    //
    // A <pre> element inserted into the editor's DOM. It displays the same
    // text as the source but with transparent colour — only the wavy
    // underline decorations on diagnostic ranges are visible.
    //
    // Positioned absolutely over the backdrop using the same font metrics,
    // so underlines align perfectly with the editor text. The textarea
    // (z-index: 1) sits above it for mouse/keyboard interaction.
    const overlay = document.createElement("pre");
    overlay.className = "ark-editor-diagnostics";
    overlay.setAttribute("aria-hidden", "true");
    overlay.style.tabSize = String(tabSize);
    // Insert between backdrop and textarea for correct paint order.
    editor.element.insertBefore(overlay, editor.textarea);
    // Sync overlay scroll with the textarea.
    function handleOverlayScroll() {
        overlay.scrollTop = editor.textarea.scrollTop;
        overlay.scrollLeft = editor.textarea.scrollLeft;
    }
    editor.textarea.addEventListener("scroll", handleOverlayScroll);
    // --- Diagnostics panel ---
    const diagnosticsPanel = createDiagnosticsPanel(wrapper, {
        injectStyles: false, // already injected above
    });
    let compilerClient = initialCompilerClient ?? null;
    let lastCompiledWasm = null;
    let canRun = false;
    let runOutputPanel = null;
    function ensureRunOutputPanel() {
        if (!runOutputPanel) {
            runOutputPanel = createRunOutputPanel(wrapper, { injectStyles: false });
        }
        return runOutputPanel;
    }
    function setRunStatus(message, isError = false) {
        if (!compilerClient)
            return;
        ensureRunOutputPanel().update([], message, isError);
    }
    function showCompileOutput(result) {
        const runnable = isRunnableT2Output(result.wasmBytes);
        ensureRunOutputPanel().update(sectionsFromCompileResult(result), buildStatusMessage(result, runnable), !result.ok || !runnable);
        return runnable;
    }
    function showRunOutput(result) {
        ensureRunOutputPanel().update(sectionsFromRunResult(result), runStatusMessage(result), !result.ok);
    }
    // --- Parse + diagnostics wiring ---
    let destroyed = false;
    let parseTimer;
    // Version counter to discard stale async parse results.
    let parseVersion = 0;
    /** Apply a parse result to the diagnostics panel and overlay. */
    function applyDiagnostics(response, source) {
        // Check for unsupported capability usage and merge warnings
        // with parse diagnostics so they display together.
        const capWarnings = checkCapabilities(source);
        const capDiags = capabilityWarningsToDiagnostics(capWarnings);
        const allDiagnostics = [
            ...response.diagnostics,
            ...capDiags,
        ];
        diagnosticsPanel.update(allDiagnostics, source);
        // Update inline markers.
        const overlayHtml = buildDiagnosticOverlay(source, allDiagnostics);
        overlay.innerHTML = overlayHtml;
        onDiagnostics?.(allDiagnostics, response);
    }
    /** Trigger a parse of the current editor content. */
    function triggerParse() {
        if (destroyed)
            return;
        const source = editor.getValue();
        const version = ++parseVersion;
        const result = parse(source);
        if (result instanceof Promise) {
            result.then((response) => {
                if (!destroyed && version === parseVersion) {
                    applyDiagnostics(response, source);
                }
            }, () => {
                // Parse failed — clear diagnostics.
                if (!destroyed && version === parseVersion) {
                    diagnosticsPanel.clear();
                    overlay.innerHTML = "";
                }
            });
        }
        else {
            applyDiagnostics(result, source);
        }
    }
    /** Schedule a debounced re-parse. */
    function debouncedParse() {
        if (parseTimer !== undefined) {
            clearTimeout(parseTimer);
        }
        parseTimer = setTimeout(triggerParse, parseDebounceMs);
    }
    // Listen for editor changes.
    const unsubChange = editor.onChange(() => {
        debouncedParse();
    });
    // Run initial parse.
    triggerParse();
    function buildRunOptions() {
        return {
            stdin: new TextEncoder().encode(stdinPanel.getValue()),
            stdinMode: "line",
        };
    }
    // --- Public API ---
    return {
        get editor() {
            return editor;
        },
        get diagnosticsPanel() {
            return diagnosticsPanel;
        },
        get runOutputPanel() {
            return runOutputPanel;
        },
        get stdinPanel() {
            return stdinPanel;
        },
        get canRun() {
            return canRun;
        },
        parse() {
            // Cancel any pending debounced parse and run immediately.
            if (parseTimer !== undefined) {
                clearTimeout(parseTimer);
                parseTimer = undefined;
            }
            triggerParse();
        },
        async build() {
            if (!compilerClient || destroyed)
                return null;
            lastCompiledWasm = null;
            canRun = false;
            runOutputPanel?.clear();
            setRunStatus("Building…");
            try {
                const result = await compilerClient.compile(editor.getValue());
                canRun = showCompileOutput(result);
                if (result.ok && result.wasmBytes) {
                    lastCompiledWasm = result.wasmBytes;
                }
                return result;
            }
            catch (err) {
                const message = err instanceof Error ? err.message : "Build failed.";
                setRunStatus(message, true);
                return null;
            }
        },
        async run() {
            if (!compilerClient || destroyed || !lastCompiledWasm)
                return null;
            setRunStatus("Running…");
            try {
                const result = await compilerClient.run(lastCompiledWasm, buildRunOptions());
                showRunOutput(result);
                return result;
            }
            catch (err) {
                const message = err instanceof Error ? err.message : "Run failed.";
                setRunStatus(message, true);
                return null;
            }
        },
        setCompilerClient(client) {
            compilerClient = client;
            if (!client) {
                lastCompiledWasm = null;
                canRun = false;
                runOutputPanel?.clear();
            }
        },
        setStdin(text) {
            stdinPanel.setValue(text);
        },
        destroy() {
            if (destroyed)
                return;
            destroyed = true;
            if (parseTimer !== undefined) {
                clearTimeout(parseTimer);
            }
            editor.textarea.removeEventListener("scroll", handleOverlayScroll);
            unsubChange();
            diagnosticsPanel.destroy();
            runOutputPanel?.destroy();
            stdinPanel.destroy();
            editor.destroy();
            if (wrapper.parentNode) {
                wrapper.parentNode.removeChild(wrapper);
            }
        },
    };
}
//# sourceMappingURL=playground-app.js.map