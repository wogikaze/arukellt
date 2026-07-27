import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

export type Theme = "dark" | "light";

const STORAGE_KEY = "arukellt-board:theme";

interface ThemeContextValue {
    theme: Theme;
    toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function initialTheme(): Theme {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "dark" || stored === "light") return stored;
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

export function ThemeProvider({ children }: { children: ReactNode }): ReactNode {
    const [theme, setTheme] = useState<Theme>(initialTheme);

    useEffect(() => {
        document.documentElement.dataset.theme = theme;
        localStorage.setItem(STORAGE_KEY, theme);
    }, [theme]);

    const toggleTheme = useCallback(() => setTheme((current) => (current === "dark" ? "light" : "dark")), []);
    const value = useMemo(() => ({ theme, toggleTheme }), [theme, toggleTheme]);
    return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
    const value = useContext(ThemeContext);
    if (!value) throw new Error("useTheme must be used inside ThemeProvider");
    return value;
}
