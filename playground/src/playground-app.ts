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

import type { ParseResponse, Diagnostic } from "./types.js";
import type { TokenizeFn, ArkEditor } from "./editor.js";
import { createEditor } from "./editor.js";
import {
  createDiagnosticsPanel,
  buildDiagnosticOverlay,
  injectDiagnosticStyles,
} from "./diagnostics.js";
import type { DiagnosticsPanel } from "./diagnostics.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/**
 * A parse function compatible with both synchronous and asynchronous
 * playground APIs.
 *
 * - Synchronous: `Playground.parse()` returns `ParseResponse` directly.
 * - Asynchronous: `WorkerPlayground.parse()` returns `Promise<ParseResponse>`.
 */
export type ParseFn = (
  source: string,
) => ParseResponse | Promise<ParseResponse>;

/** Options for creating a playground application. */
export interface PlaygroundAppOptions {
  /** Initial source code to display. */
  initialValue?: string;

  /**
   * Parse function for diagnostics.
   *
   * Compatible with both `Playground.parse()` (sync) and
   * `WorkerPlayground.parse()` (async).
   */
  parse: ParseFn;

  /**
   * Tokenize function for syntax highlighting.
   *
   * If omitted, the editor works as a plain text editor without
   * syntax highlighting.
   */
  tokenize?: TokenizeFn;

  /**
   * Debounce interval in milliseconds for re-parsing on edit.
   * Default: `300`.
   */
  parseDebounceMs?: number;

  /**
   * Debounce interval in milliseconds for syntax highlighting.
   * Default: `30`.
   */
  highlightDebounceMs?: number;

  /** Tab size in spaces. Default: `4`. */
  tabSize?: number;

  /** Placeholder text shown when the editor is empty. */
  placeholder?: string;

  /** Whether the editor is read-only. Default: `false`. */
  readOnly?: boolean;

  /**
   * Callback invoked whenever diagnostics are updated.
   */
  onDiagnostics?: (
    diagnostics: Diagnostic[],
    response: ParseResponse,
  ) => void;
}

/** A playground application instance. */
export interface PlaygroundApp {
  /** The editor instance. */
  readonly editor: ArkEditor;

  /** The diagnostics panel instance. */
  readonly diagnosticsPanel: DiagnosticsPanel;

  /** Force an immediate re-parse of the current editor content. */
  parse(): void;

  /** Destroy the application and clean up all resources. */
  destroy(): void;
}

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
export function createPlaygroundApp(
  container: HTMLElement,
  options: PlaygroundAppOptions,
): PlaygroundApp {
  const {
    initialValue = "",
    parse,
    tokenize,
    parseDebounceMs = 300,
    highlightDebounceMs = 30,
    tabSize = 4,
    placeholder,
    readOnly = false,
    onDiagnostics,
  } = options;

  // Inject diagnostic styles (idempotent).
  injectDiagnosticStyles();

  // --- Layout ---
  const wrapper = document.createElement("div");
  wrapper.className = "ark-playground-app";
  container.appendChild(wrapper);

  const editorContainer = document.createElement("div");
  editorContainer.className = "ark-playground-editor-container";
  wrapper.appendChild(editorContainer);

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
  function handleOverlayScroll(): void {
    overlay.scrollTop = editor.textarea.scrollTop;
    overlay.scrollLeft = editor.textarea.scrollLeft;
  }
  editor.textarea.addEventListener("scroll", handleOverlayScroll);

  // --- Diagnostics panel ---
  const diagnosticsPanel = createDiagnosticsPanel(wrapper, {
    injectStyles: false, // already injected above
  });

  // --- Parse + diagnostics wiring ---
  let destroyed = false;
  let parseTimer: ReturnType<typeof setTimeout> | undefined;

  // Version counter to discard stale async parse results.
  let parseVersion = 0;

  /** Apply a parse result to the diagnostics panel and overlay. */
  function applyDiagnostics(
    response: ParseResponse,
    source: string,
  ): void {
    diagnosticsPanel.update(response.diagnostics, source);

    // Update inline markers.
    const overlayHtml = buildDiagnosticOverlay(source, response.diagnostics);
    overlay.innerHTML = overlayHtml;

    onDiagnostics?.(response.diagnostics, response);
  }

  /** Trigger a parse of the current editor content. */
  function triggerParse(): void {
    if (destroyed) return;

    const source = editor.getValue();
    const version = ++parseVersion;

    const result = parse(source);

    if (result instanceof Promise) {
      result.then(
        (response) => {
          if (!destroyed && version === parseVersion) {
            applyDiagnostics(response, source);
          }
        },
        () => {
          // Parse failed — clear diagnostics.
          if (!destroyed && version === parseVersion) {
            diagnosticsPanel.clear();
            overlay.innerHTML = "";
          }
        },
      );
    } else {
      applyDiagnostics(result, source);
    }
  }

  /** Schedule a debounced re-parse. */
  function debouncedParse(): void {
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

  // --- Public API ---
  return {
    get editor(): ArkEditor {
      return editor;
    },

    get diagnosticsPanel(): DiagnosticsPanel {
      return diagnosticsPanel;
    },

    parse(): void {
      // Cancel any pending debounced parse and run immediately.
      if (parseTimer !== undefined) {
        clearTimeout(parseTimer);
        parseTimer = undefined;
      }
      triggerParse();
    },

    destroy(): void {
      if (destroyed) return;
      destroyed = true;

      if (parseTimer !== undefined) {
        clearTimeout(parseTimer);
      }

      editor.textarea.removeEventListener("scroll", handleOverlayScroll);
      unsubChange();
      diagnosticsPanel.destroy();
      editor.destroy();

      if (wrapper.parentNode) {
        wrapper.parentNode.removeChild(wrapper);
      }
    },
  };
}
