// Ambient declarations for the globals interop.js attaches to `window`.
export {}

declare global {
  interface Window {
    startDrag?: () => void
    closeWindow?: () => void
    minimizeWindow?: () => void
    minimizeToTray?: () => void
    maximizeWindow?: () => void
    toggleFullscreen?: () => Promise<void>
    glacierViewTransition?: (apply: () => void) => void | Promise<void>
    setUiScale?: (pct: number) => void
    setDropActive?: (active: boolean) => void
    onHostFileDropped?: (path: string) => void
    copyToClipboard?: (text: string) => Promise<boolean>
    setAccentColor?: (color: string) => void
    setTheme?: (preset: string) => void
    setBlurIntensity?: (px: number) => void
    setCustomBackground?: (filePath: string) => void
    setCompactMode?: (enabled: boolean) => void
    setAnimationsEnabled?: (enabled: boolean) => void
    setAnimationSpeed?: (speed: number) => void
    applyStoredSettings?: (...args: unknown[]) => void
    setFont?: (family: string) => void
    setCustomCss?: (css: string) => void
    setThemeVars?: (vars: Record<string, string>) => void
    clearThemeVars?: () => void
    setBackgroundFit?: (fit: string, opacity: number) => void
    openDiscordOAuth?: (clientId: string) => void
    focusSearchInput?: () => void
    glacierSkin?: unknown
  }
}
