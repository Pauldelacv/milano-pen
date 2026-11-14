import { useEffect } from 'react';
import { useUiStore } from '../stores/uiStore';

/** Applies the theme setting; "system" follows macOS via prefers-color-scheme. */
export function useTheme(): void {
  const theme = useUiStore((s) => s.settings.theme);
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') delete root.dataset.theme;
    else root.dataset.theme = theme;
  }, [theme]);
}
