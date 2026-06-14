/**
 * Virtual stdin panel for playground Run (ADR-020 companion surface).
 *
 * @module
 */
export interface StdinPanelOptions {
    /** Initial stdin text. */
    initialValue?: string;
    /** Whether to auto-inject panel CSS. Default: `true`. */
    injectStyles?: boolean;
}
export interface StdinPanel {
    readonly element: HTMLElement;
    readonly textarea: HTMLTextAreaElement;
    getValue(): string;
    setValue(text: string): void;
    destroy(): void;
}
export declare const STDIN_PANEL_CSS = "\n.ark-stdin-panel {\n  flex-shrink: 0;\n  display: flex;\n  flex-direction: column;\n  gap: 6px;\n  padding: 12px 16px;\n  background: var(--ark-stdin-bg, #181825);\n  border: 1px solid var(--ark-stdin-border, #313244);\n  border-radius: 10px;\n}\n\n.ark-stdin-label {\n  font-family: system-ui, sans-serif;\n  font-size: 0.72rem;\n  text-transform: uppercase;\n  letter-spacing: 0.08em;\n  color: var(--ark-stdin-label, #9399b2);\n}\n\n.ark-stdin-input {\n  min-height: 4.5rem;\n  resize: vertical;\n  background: var(--ark-stdin-input-bg, #1e1e2e);\n  color: var(--ark-stdin-input-fg, #cdd6f4);\n  border: 1px solid var(--ark-stdin-input-border, #45475a);\n  border-radius: 8px;\n  padding: 10px 12px;\n  font: 0.82rem/1.4 ui-monospace, SFMono-Regular, Menlo, monospace;\n}\n\n.ark-stdin-hint {\n  font-family: system-ui, sans-serif;\n  font-size: 0.78rem;\n  color: var(--ark-stdin-hint, #9399b2);\n}\n";
export declare function injectStdinPanelStyles(): void;
export declare function createStdinPanel(container: HTMLElement, options?: StdinPanelOptions): StdinPanel;
//# sourceMappingURL=stdin-panel.d.ts.map