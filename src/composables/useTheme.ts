import { ref, watchEffect } from "vue";

export type ThemeMode = "light" | "dark";

const STORAGE_KEY = "app-theme";

function getSystemTheme(): ThemeMode {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function loadTheme(): ThemeMode {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "light" || stored === "dark") return stored;
  return getSystemTheme();
}

const themeMode = ref<ThemeMode>(loadTheme());

export function useTheme() {
  function setTheme(mode: ThemeMode) {
    themeMode.value = mode;
    localStorage.setItem(STORAGE_KEY, mode);
  }

  function toggleTheme() {
    setTheme(themeMode.value === "dark" ? "light" : "dark");
  }

  watchEffect(() => {
    document.documentElement.setAttribute("data-theme", themeMode.value);
  });

  return {
    themeMode,
    isDark: () => themeMode.value === "dark",
    setTheme,
    toggleTheme,
  };
}
