/**
 * Playground editor shell with syntax highlighting.
 *
 * A minimal, dependency-free code editor that uses the Arukellt tokenizer
 * for syntax highlighting. Built on standard Web APIs — no React, Vue,
 * or other framework dependency.
 *
 * ## Architecture
 *
 * The editor uses the classic "transparent textarea over highlighted
 * backdrop" pattern:
 *
 * 1. A `<textarea>` handles all editing (type, delete, select, paste,
 *    undo/redo) natively. Its text colour is transparent so the caret
 *    is visible but the text is not.
 *
 * 2. A `<pre>` element behind the textarea displays the syntax-highlighted
 *    HTML. It mirrors the textarea content exactly and is kept in sync
 *    on every input event.
 *
 * 3. Scroll positions are synchronised so the highlighted text always
 *    aligns with the textarea caret.
 *
 * @module
 */
import { highlightTokens } from "./highlight.js";
import { injectTheme } from "./theme.js";
// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------
/**
 * Create a playground editor and mount it inside the given container.
 *
 * @param container - The DOM element to mount the editor into.
 * @param options - Editor configuration.
 * @returns An {@link ArkEditor} instance.
 *
 * @example
 * ```ts
 * import { createPlayground, createEditor } from "@arukellt/playground";
 *
 * const pg = await createPlayground(wasmModulePath, { wasmUrl });
 * const editor = createEditor(document.getElementById("editor")!, {
 *   initialValue: "fn main() {\n    let x = 42\n}\n",
 *   tokenize: (src) => pg.tokenize(src),
 * });
 * ```
 */
export function createEditor(container, options = {}) {
    const { initialValue = "", tokenize, tabSize = 4, debounceMs = 30, injectDefaultTheme = true, readOnly = false, placeholder, } = options;
    // Inject default theme if requested.
    if (injectDefaultTheme) {
        injectTheme();
    }
    // --- DOM structure ---
    const root = document.createElement("div");
    root.className = "ark-editor";
    root.setAttribute("role", "group");
    root.setAttribute("aria-label", "Arukellt code editor");
    const backdrop = document.createElement("pre");
    backdrop.className = "ark-editor-backdrop";
    backdrop.setAttribute("aria-hidden", "true");
    const textarea = document.createElement("textarea");
    textarea.className = "ark-editor-textarea";
    textarea.setAttribute("autocapitalize", "off");
    textarea.setAttribute("autocomplete", "off");
    textarea.setAttribute("autocorrect", "off");
    textarea.setAttribute("spellcheck", "false");
    textarea.setAttribute("aria-label", "Code input");
    textarea.style.tabSize = String(tabSize);
    backdrop.style.tabSize = String(tabSize);
    if (readOnly) {
        textarea.readOnly = true;
    }
    if (placeholder) {
        textarea.placeholder = placeholder;
    }
    root.appendChild(backdrop);
    root.appendChild(textarea);
    container.appendChild(root);
    // --- State ---
    let destroyed = false;
    let debounceTimer;
    const listeners = [];
    if (options.onChange) {
        listeners.push(options.onChange);
    }
    // --- Highlighting ---
    /** Render highlighted HTML into the backdrop. */
    function renderHighlight(source, tokens) {
        if (destroyed)
            return;
        backdrop.innerHTML = highlightTokens(source, tokens);
    }
    /** Fallback: render plain (unhighlighted) text. */
    function renderPlain(source) {
        if (destroyed)
            return;
        const escaped = source
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
        backdrop.innerHTML = escaped + "\n";
    }
    /**
     * Trigger re-highlighting for the current textarea value.
     *
     * For synchronous tokenize functions, highlighting is applied
     * immediately. For async functions, a debounced call is used.
     */
    function updateHighlight() {
        const source = textarea.value;
        if (!tokenize) {
            renderPlain(source);
            return;
        }
        const result = tokenize(source);
        if (result instanceof Promise) {
            // Async path — debounce to avoid flooding the worker.
            if (debounceTimer !== undefined) {
                clearTimeout(debounceTimer);
            }
            // Show plain text immediately for responsiveness.
            renderPlain(source);
            debounceTimer = setTimeout(() => {
                result.then((resp) => {
                    // Only apply if the source hasn't changed since the request.
                    if (!destroyed && textarea.value === source) {
                        renderHighlight(source, resp.tokens);
                    }
                }, () => {
                    // Tokenization failed — keep plain text rendering.
                });
            }, debounceMs);
        }
        else {
            // Sync path — immediate.
            renderHighlight(source, result.tokens);
        }
    }
    /** Notify all change listeners. */
    function notifyChange() {
        const value = textarea.value;
        for (const listener of listeners) {
            listener(value);
        }
    }
    // --- Event handlers ---
    function handleInput() {
        updateHighlight();
        notifyChange();
    }
    function handleScroll() {
        backdrop.scrollTop = textarea.scrollTop;
        backdrop.scrollLeft = textarea.scrollLeft;
    }
    function handleKeydown(event) {
        // Tab key: insert spaces instead of moving focus.
        if (event.key === "Tab" && !event.ctrlKey && !event.metaKey) {
            event.preventDefault();
            const start = textarea.selectionStart;
            const end = textarea.selectionEnd;
            const spaces = " ".repeat(tabSize);
            if (start === end) {
                // No selection — insert spaces at cursor.
                document.execCommand("insertText", false, spaces);
            }
            else if (!event.shiftKey) {
                // Selection — indent selected lines.
                const value = textarea.value;
                const before = value.slice(0, start);
                const selected = value.slice(start, end);
                const after = value.slice(end);
                const indented = selected.replace(/^/gm, spaces);
                textarea.value = before + indented + after;
                textarea.selectionStart = start;
                textarea.selectionEnd = start + indented.length;
                updateHighlight();
                notifyChange();
            }
        }
    }
    // --- Wire up events ---
    textarea.addEventListener("input", handleInput);
    textarea.addEventListener("scroll", handleScroll);
    textarea.addEventListener("keydown", handleKeydown);
    // --- Initial render ---
    textarea.value = initialValue;
    updateHighlight();
    // --- Cleanup tracking ---
    const abortController = new AbortController();
    // --- Public API ---
    const editor = {
        getValue() {
            return textarea.value;
        },
        setValue(value) {
            textarea.value = value;
            updateHighlight();
        },
        focus() {
            textarea.focus();
        },
        onChange(listener) {
            listeners.push(listener);
            return () => {
                const idx = listeners.indexOf(listener);
                if (idx !== -1) {
                    listeners.splice(idx, 1);
                }
            };
        },
        get element() {
            return root;
        },
        get textarea() {
            return textarea;
        },
        destroy() {
            if (destroyed)
                return;
            destroyed = true;
            if (debounceTimer !== undefined) {
                clearTimeout(debounceTimer);
            }
            textarea.removeEventListener("input", handleInput);
            textarea.removeEventListener("scroll", handleScroll);
            textarea.removeEventListener("keydown", handleKeydown);
            abortController.abort();
            listeners.length = 0;
            if (root.parentNode) {
                root.parentNode.removeChild(root);
            }
        },
    };
    return editor;
}
//# sourceMappingURL=editor.js.map