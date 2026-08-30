import type { UserProfile } from './api'

const VARS = ['--bg', '--text', '--text-h', '--border', '--button-bg', '--button-text'] as const

const PRESETS: Record<'light' | 'dark', Record<(typeof VARS)[number], string>> = {
  light: {
    '--bg': '#ffffff',
    '--text': '#24292f',
    '--text-h': '#08060d',
    '--border': '#e5e4e7',
    '--button-bg': '#f0f0f0',
    '--button-text': '#24292f',
  },
  dark: {
    '--bg': '#16171d',
    '--text': '#d1d5db',
    '--text-h': '#f3f4f6',
    '--border': '#2e303a',
    '--button-bg': '#2a2c34',
    '--button-text': '#d1d5db',
  },
}

type ThemeInput = Pick<UserProfile, 'theme' | 'customBg' | 'customText' | 'customButtonBg' | 'customButtonText'>

/** Applies the user's chosen color theme by setting (or clearing) CSS custom properties
 * on the document root. 'system' clears all overrides so app.css's prefers-color-scheme
 * media query takes over. */
export function applyTheme(profile: ThemeInput) {
  const root = document.documentElement.style

  if (profile.theme === 'system') {
    for (const v of VARS) root.removeProperty(v)
    return
  }

  if (profile.theme === 'light' || profile.theme === 'dark') {
    const preset = PRESETS[profile.theme]
    for (const v of VARS) root.setProperty(v, preset[v])
    return
  }

  // Custom: apply whichever of the four colors are set, clear the rest back to default.
  const set = (varName: (typeof VARS)[number], value: string | null) => {
    if (value) root.setProperty(varName, value)
    else root.removeProperty(varName)
  }
  set('--bg', profile.customBg)
  set('--text', profile.customText)
  set('--text-h', profile.customText)
  set('--button-bg', profile.customButtonBg)
  set('--button-text', profile.customButtonText)
  root.removeProperty('--border')
}
