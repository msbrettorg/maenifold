import { useCallback, useEffect, useMemo, useState } from "react";

import {
  applyTheme,
  getInitialTheme,
  type Theme,
} from "@/lib/theme";

type UseThemeResult = {
  theme: Theme;
  isDark: boolean;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
};

export function useTheme(): UseThemeResult {
  const [theme, setThemeState] = useState<Theme>(() => getInitialTheme());

  const setTheme = useCallback((nextTheme: Theme) => {
    setThemeState(nextTheme);
    applyTheme(nextTheme);

    if (typeof window !== "undefined") {
      window.localStorage.setItem("theme", nextTheme);
    }
  }, []);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme(theme === "dark" ? "light" : "dark");
  }, [setTheme, theme]);

  return useMemo(
    () => ({
      theme,
      isDark: theme === "dark",
      setTheme,
      toggleTheme,
    }),
    [setTheme, theme, toggleTheme],
  );
}
