<script lang="ts">
  import { onMount } from 'svelte'
  import {
    api,
    START_PAGES,
    LAUNCH_POSITIONS,
    THEMES,
    type StartPage,
    type LaunchPosition,
    type Theme,
    type WindowGeometry,
  } from './api'
  import { store, navGuard, refreshUserProfile } from './store.svelte'
  import { isDirty } from './dateUtils'
  import { applyTheme } from './theme'

  let { onBack }: { onBack: () => void } = $props()

  const THEME_LABELS: Record<Theme, string> = {
    system: 'System (follow OS)',
    light: 'Light',
    dark: 'Dark',
    custom: 'Custom',
  }

  const START_PAGE_LABELS: Record<StartPage, string> = {
    timer: 'Timer',
    'quick-timer': 'Quick Timer',
    clients: 'Clients',
    contracts: 'Contracts',
    entries: 'Entries',
    trash: 'Trash',
    reports: 'Reports',
    import: 'Import',
    user: 'User',
    admin: 'Admin',
    backup: 'Backup & Restore',
    appearance: 'Appearance',
  }

  const LAUNCH_POSITION_LABELS: Record<LaunchPosition, string> = {
    default: 'Default (OS decides)',
    'top-left': 'Upper Left',
    'top-center': 'Upper Center',
    'top-right': 'Upper Right',
    'middle-left': 'Middle Left',
    center: 'Center',
    'middle-right': 'Middle Right',
    'bottom-left': 'Lower Left',
    'bottom-center': 'Lower Center',
    'bottom-right': 'Lower Right',
    custom: 'Custom (remembered position)',
  }

  let windowWidth = $state<number | ''>('')
  let windowHeight = $state<number | ''>('')
  let windowX = $state<number | ''>('')
  let windowY = $state<number | ''>('')
  let defaultStartPage = $state<StartPage>('timer')
  let launchPosition = $state<LaunchPosition>('default')
  let theme = $state<Theme>('system')
  let customBg = $state('#ffffff')
  let customText = $state('#24292f')
  let customButtonBg = $state('#f0f0f0')
  let customButtonText = $state('#24292f')
  let busy = $state(false)
  let loadedFor = $state<string | null>(null)

  let currentGeometry = $state<WindowGeometry | null>(null)

  onMount(async () => {
    currentGeometry = await api.getWindowGeometry()
  })

  $effect(() => {
    applyTheme({ theme, customBg, customText, customButtonBg, customButtonText })
  })

  async function useCurrentAsDefault() {
    const geo = await api.getWindowGeometry()
    currentGeometry = geo
    windowWidth = geo.width
    windowHeight = geo.height
    windowX = geo.x
    windowY = geo.y
    launchPosition = 'custom'
  }

  $effect(() => {
    const key = JSON.stringify(store.userProfile)
    if (loadedFor !== key) {
      windowWidth = store.userProfile.windowWidth ?? ''
      windowHeight = store.userProfile.windowHeight ?? ''
      windowX = store.userProfile.windowX ?? ''
      windowY = store.userProfile.windowY ?? ''
      defaultStartPage = store.userProfile.defaultStartPage
      launchPosition = store.userProfile.launchPosition
      theme = store.userProfile.theme
      customBg = store.userProfile.customBg ?? '#ffffff'
      customText = store.userProfile.customText ?? '#24292f'
      customButtonBg = store.userProfile.customButtonBg ?? '#f0f0f0'
      customButtonText = store.userProfile.customButtonText ?? '#24292f'
      loadedFor = key
    }
  })

  let dirty = $derived(
    isDirty(
      { windowWidth, windowHeight, windowX, windowY, defaultStartPage, launchPosition, theme, customBg, customText, customButtonBg, customButtonText },
      {
        windowWidth: store.userProfile.windowWidth ?? '',
        windowHeight: store.userProfile.windowHeight ?? '',
        windowX: store.userProfile.windowX ?? '',
        windowY: store.userProfile.windowY ?? '',
        defaultStartPage: store.userProfile.defaultStartPage,
        launchPosition: store.userProfile.launchPosition,
        theme: store.userProfile.theme,
        customBg: store.userProfile.customBg ?? '#ffffff',
        customText: store.userProfile.customText ?? '#24292f',
        customButtonBg: store.userProfile.customButtonBg ?? '#f0f0f0',
        customButtonText: store.userProfile.customButtonText ?? '#24292f',
      },
    ),
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  async function save() {
    busy = true
    try {
      await api.saveUserProfile({
        ...store.userProfile,
        windowWidth: windowWidth === '' ? null : Number(windowWidth),
        windowHeight: windowHeight === '' ? null : Number(windowHeight),
        windowX: windowX === '' ? null : Number(windowX),
        windowY: windowY === '' ? null : Number(windowY),
        defaultStartPage,
        launchPosition,
        theme,
        customBg,
        customText,
        customButtonBg,
        customButtonText,
      })
      await refreshUserProfile()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <button onclick={onBack}>&larr; Back to Admin</button>

  <h2>Appearance</h2>

  <h3>Position</h3>
  <div class="field">
    <label for="window-width">Window size</label>
    <input id="window-width" type="number" min="200" placeholder="Width" bind:value={windowWidth} style="width: 6rem; flex: 0 0 auto" />
    <span>&times;</span>
    <input id="window-height" type="number" min="200" placeholder="Height" bind:value={windowHeight} style="width: 6rem; flex: 0 0 auto" />
    <span class="muted">px (applied next launch)</span>
  </div>
  <div class="field">
    <span class="field-spacer" aria-hidden="true"></span>
    <span class="muted">
      {#if currentGeometry}
        Current size: {currentGeometry.width} &times; {currentGeometry.height}px at ({currentGeometry.x}, {currentGeometry.y})
      {:else}
        Loading current window info…
      {/if}
    </span>
  </div>
  <div class="field">
    <span class="field-spacer" aria-hidden="true"></span>
    <button type="button" onclick={useCurrentAsDefault}>Use current position &amp; size as default</button>
  </div>
  <div class="field">
    <label for="launch-position">Launch position</label>
    <select id="launch-position" bind:value={launchPosition}>
      {#each LAUNCH_POSITIONS as pos}
        <option value={pos}>{LAUNCH_POSITION_LABELS[pos]}</option>
      {/each}
    </select>
    {#if launchPosition === 'custom' && windowX !== '' && windowY !== ''}
      <span class="muted">({windowX}, {windowY})</span>
    {/if}
  </div>

  <h3>Start Page</h3>
  <div class="field">
    <label for="start-page">Start page</label>
    <select id="start-page" bind:value={defaultStartPage}>
      {#each START_PAGES as page}
        <option value={page}>{START_PAGE_LABELS[page]}</option>
      {/each}
    </select>
  </div>

  <h3>Colors</h3>
  <div class="field">
    <label for="theme">Theme</label>
    <select id="theme" bind:value={theme}>
      {#each THEMES as t}
        <option value={t}>{THEME_LABELS[t]}</option>
      {/each}
    </select>
  </div>
  {#if theme === 'custom'}
    <div class="field">
      <label for="custom-bg">Background</label>
      <input id="custom-bg" type="color" bind:value={customBg} />
    </div>
    <div class="field">
      <label for="custom-text">Text</label>
      <input id="custom-text" type="color" bind:value={customText} />
    </div>
    <div class="field">
      <label for="custom-button-bg">Button background</label>
      <input id="custom-button-bg" type="color" bind:value={customButtonBg} />
    </div>
    <div class="field">
      <label for="custom-button-text">Button text</label>
      <input id="custom-button-text" type="color" bind:value={customButtonText} />
    </div>
  {/if}

  <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty}>Save</button>
</section>

<style>
  .panel {
    margin-bottom: 2rem;
    max-width: 32rem;
  }
  h3 {
    margin-top: 1.75rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .field label,
  .field-spacer {
    width: 7rem;
    flex-shrink: 0;
  }
  .field input,
  .field select {
    flex: 1;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
  .muted {
    color: #666;
    font-size: 0.85rem;
  }
</style>
