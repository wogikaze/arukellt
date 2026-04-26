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
import type { TokenizeResponse } from "./types.js";
/**
 * A tokenize function that the editor calls to syntax-highlight the source.
 *
 * Compatible with both the synchronous `Playground.tokenize()` and the
 * asynchronous `WorkerPlayground.tokenize()` APIs.
 */
export type TokenizeFn = (source: string) => TokenizeResponse | Promise<TokenizeResponse>;
/** Options for creating an editor instance. */
export interface EditorOptions {
    /** Initial source code to display. */
    initialValue?: string;
    /**
     * Tokenize function for syntax highlighting.
     *
     * If omitted, the editor still works as a plain text editor without
     * syntax highlighting.
     */
    tokenize?: TokenizeFn;
    /** Callback invoked whenever the editor content changes. */
    onChange?: (value: string) => void;
    /** Tab size in spaces (default: `4`). */
    tabSize?: number;
    /**
     * Debounce interval in milliseconds for async tokenization (default: `30`).
     *
     * Only applies when `tokenize` returns a `Promise`. Synchronous
     * tokenize functions are called immediately on every input.
     */
    debounceMs?: number;
    /**
     * Whether to auto-inject the default theme CSS into the page.
     *
     * Set to `false` if you provide your own stylesheet.
     * Default: `true`.
     */
    injectDefaultTheme?: boolean;
    /**
     * Whether the textarea is read-only.
     * Default: `false`.
     */
    readOnly?: boolean;
    /**
     * Placeholder text shown when the editor is empty.
     */
    placeholder?: string;
}
/** The playground editor instance. */
export interface ArkEditor {
    /** Get the current editor value. */
    getValue(): string;
    /** Set the editor value and re-highlight. */
    setValue(value: string): void;
    /** Focus the editor textarea. */
    focus(): void;
    /**
     * Register a change listener. Returns an unsubscribe function.
     *
     * This is an alternative to the `onChange` option — both mechanisms
     * work and can be used together.
     */
    onChange(listener: (value: string) => void): () => void;
    /** The root DOM element of the editor. */
    readonly element: HTMLElement;
    /** The underlying textarea element (for advanced use). */
    readonly textarea: HTMLTextAreaElement;
    /** Destroy the editor and clean up event listeners. */
    destroy(): void;
}
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
export declare function createEditor(container: HTMLElement, options?: EditorOptions): ArkEditor;
//# sourceMappingURL=editor.d.ts.map