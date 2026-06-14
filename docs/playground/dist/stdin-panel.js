/**
 * Virtual stdin panel for playground Run (ADR-020 companion surface).
 *
 * @module
 */
export const STDIN_PANEL_CSS = `
.ark-stdin-panel {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px;
  background: var(--ark-stdin-bg, #181825);
  border: 1px solid var(--ark-stdin-border, #313244);
  border-radius: 10px;
}

.ark-stdin-label {
  font-family: system-ui, sans-serif;
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--ark-stdin-label, #9399b2);
}

.ark-stdin-input {
  min-height: 4.5rem;
  resize: vertical;
  background: var(--ark-stdin-input-bg, #1e1e2e);
  color: var(--ark-stdin-input-fg, #cdd6f4);
  border: 1px solid var(--ark-stdin-input-border, #45475a);
  border-radius: 8px;
  padding: 10px 12px;
  font: 0.82rem/1.4 ui-monospace, SFMono-Regular, Menlo, monospace;
}

.ark-stdin-hint {
  font-family: system-ui, sans-serif;
  font-size: 0.78rem;
  color: var(--ark-stdin-hint, #9399b2);
}
`;
let stylesInjected = false;
export function injectStdinPanelStyles() {
    if (stylesInjected || typeof document === "undefined")
        return;
    const style = document.createElement("style");
    style.setAttribute("data-ark-stdin-panel", "true");
    style.textContent = STDIN_PANEL_CSS;
    document.head.appendChild(style);
    stylesInjected = true;
}
export function createStdinPanel(container, options = {}) {
    const { initialValue = "", injectStyles = true } = options;
    if (injectStyles) {
        injectStdinPanelStyles();
    }
    const root = document.createElement("section");
    root.className = "ark-stdin-panel";
    root.setAttribute("aria-label", "Program stdin");
    const label = document.createElement("span");
    label.className = "ark-stdin-label";
    label.textContent = "Stdin";
    const textarea = document.createElement("textarea");
    textarea.id = "ark-stdin-input";
    textarea.className = "ark-stdin-input";
    textarea.spellcheck = false;
    textarea.placeholder = "1 2 +";
    textarea.setAttribute("aria-label", "Program stdin (one line per read)");
    textarea.value = initialValue;
    const hint = document.createElement("p");
    hint.className = "ark-stdin-hint";
    hint.textContent =
        "Fed to the program on Run (line-delimited). Empty second line ends REPLs.";
    root.appendChild(label);
    root.appendChild(textarea);
    root.appendChild(hint);
    container.appendChild(root);
    return {
        element: root,
        textarea,
        getValue() {
            return textarea.value;
        },
        setValue(text) {
            textarea.value = text;
        },
        destroy() {
            root.remove();
        },
    };
}
//# sourceMappingURL=stdin-panel.js.map