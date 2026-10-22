<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { tooltip } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import IconMinimize from '~icons/fluent/subtract-24-regular';
  import IconMaximize from '~icons/fluent/square-24-regular';
  import IconRestore from '~icons/fluent/square-multiple-24-regular';
  import IconClose from '~icons/fluent/dismiss-24-regular';

  interface Props {
    variant: 'windows' | 'macos';
  }

  let { variant }: Props = $props();

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);

  onMount(() => {
    const sync = () => void appWindow.isMaximized().then((value) => (isMaximized = value));
    sync();
    const unlisten = appWindow.onResized(sync);
    return () => void unlisten.then((stop) => stop());
  });

  let maximizeLabel = $derived(i18n.t(isMaximized ? 'actions.restore' : 'actions.maximize'));
</script>

{#if variant === 'windows'}
  <div class="window-controls">
    <button class="caption-btn" use:tooltip={i18n.t('actions.minimize')} aria-label={i18n.t('actions.minimize')} onclick={() => appWindow.minimize()}>
      <IconMinimize />
    </button>
    <button class="caption-btn" use:tooltip={maximizeLabel} aria-label={maximizeLabel} onclick={() => appWindow.toggleMaximize()}>
      {#if isMaximized}<IconRestore />{:else}<IconMaximize />{/if}
    </button>
    <button class="caption-btn is-close" use:tooltip={i18n.t('actions.close')} aria-label={i18n.t('actions.close')} onclick={() => appWindow.close()}>
      <IconClose />
    </button>
  </div>
{:else}
  <div class="traffic-lights">
    <button class="light is-close" aria-label={i18n.t('actions.close')} onclick={() => appWindow.close()}>
      <svg viewBox="0 0 6 6" fill="none" aria-hidden="true">
        <path d="M1 1L5 5M5 1L1 5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" />
      </svg>
    </button>
    <button class="light is-minimize" aria-label={i18n.t('actions.minimize')} onclick={() => appWindow.minimize()}>
      <svg viewBox="0 0 6 6" fill="none" aria-hidden="true">
        <path d="M0.75 3H5.25" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" />
      </svg>
    </button>
    <button class="light is-maximize" aria-label={maximizeLabel} onclick={() => appWindow.toggleMaximize()}>
      <svg viewBox="0 0 6 6" fill="none" aria-hidden="true">
        <path d="M1 1H4L1 4V1Z" fill="currentColor" />
        <path d="M5 5H2L5 2V5Z" fill="currentColor" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .window-controls {
    display: flex;
    align-self: stretch;
    flex-shrink: 0;
  }

  .caption-btn {
    display: grid;
    place-items: center;
    width: var(--window-control-width);
    height: 100%;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-secondary);
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }

  .caption-btn :global(svg) {
    width: 16px;
    height: 16px;
  }

  @media (hover: hover) {
    .caption-btn:hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }

    .caption-btn.is-close:hover {
      background: var(--status-error);
      color: var(--on-status);
    }
  }

  .traffic-lights {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .light {
    display: grid;
    place-items: center;
    width: 12px;
    height: 12px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 50%;
    box-sizing: border-box;
    transition: filter var(--duration-fast) ease;
  }

  .light:active {
    filter: brightness(0.82);
  }

  .light svg {
    width: 6px;
    height: 6px;
    color: var(--color-mac-icon);
    opacity: 0;
    transition: opacity var(--duration-fast) ease;
    pointer-events: none;
  }

  .traffic-lights:hover .light svg {
    opacity: 1;
  }

  .light.is-close {
    background-color: var(--color-mac-close);
    border-color: var(--color-mac-close-border);
  }

  .light.is-minimize {
    background-color: var(--color-mac-minimize);
    border-color: var(--color-mac-minimize-border);
  }

  .light.is-maximize {
    background-color: var(--color-mac-maximize);
    border-color: var(--color-mac-maximize-border);
  }
</style>
