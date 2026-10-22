<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { openExternalUrl } from '$lib/utils/ipc';
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import { tooltip } from '$lib/motion';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconWindows from '~icons/fluent/desktop-24-regular';
  import IconLinux from '~icons/fluent/laptop-24-regular';

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open = false, onclose }: Props = $props();

  const isLinux = typeof navigator !== 'undefined' && /Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent);
  const initialTab = isLinux ? 'linux' : 'windows';

  let activeTab = $state<'windows' | 'linux'>('windows');
  let copied = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (open) {
      activeTab = initialTab;
    }
  });

  const tabOptions = [
    { value: 'windows', label: 'Windows', icon: IconWindows },
    { value: 'linux', label: 'Linux', icon: IconLinux }
  ];

  const LINUX_COMMANDS = [
    { distro: 'Ubuntu / Debian', command: 'sudo apt install gstreamer1.0-plugins-bad gstreamer1.0-libav' },
    { distro: 'Arch Linux', command: 'sudo pacman -S gst-plugins-bad gst-libav' },
    { distro: 'Fedora', command: 'sudo dnf install gstreamer1-plugins-bad-freeworld gstreamer1-libav' }
  ];

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = text;
      notify.success(i18n.t('post.codec_copied'), { glyph: 'copied' });
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = null; }, 2000);
    } catch (error) {
      notify.error(i18n.t('common.copy_failed'), error);
    }
  }
</script>

{#snippet codecBody()}
  <div class="codec-modal-body">
    <p class="codec-intro">
      {i18n.t('post.codec_why_desc')}
    </p>

    <div class="segmented-wrapper">
      <SegmentedControl
        options={tabOptions}
        value={activeTab}
        onchange={(v) => activeTab = v}
        compact={true}
      />
    </div>

    {#if activeTab === 'windows'}
      <div class="tab-content">
        <div class="cmd-group">
          <span class="cmd-label">Microsoft Store</span>
          <Button
            variant="tonal"
            class="store-btn"
            onclick={() => void openExternalUrl('https://apps.microsoft.com/detail/9nmzlz57r3t7')}
          >
            <IconOpen class="mr-2" />
            {i18n.t('post.codec_windows_m2_btn')}
          </Button>
        </div>

        <p class="tab-note">
          {i18n.t('post.codec_alt_player_desc')}
        </p>
      </div>
    {:else if activeTab === 'linux'}
      <div class="tab-content">
        {#each LINUX_COMMANDS as { distro, command } (distro)}
          <div class="cmd-group">
            <span class="cmd-label">{distro}</span>
            <div class="cmd-box">
              <code class="cmd-text">{command}</code>
              <button type="button" class="copy-btn" onclick={() => void copy(command)} use:tooltip={i18n.t('common.copy')} aria-label={i18n.t('common.copy')}>
                {#if copied === command}<IconCheck class="is-done" />{:else}<IconCopy />{/if}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

{#if layoutState.isMobile}
  <BottomSheet
    isOpen={open}
    title={i18n.t('post.codec_guide_title')}
    {onclose}
  >
    {@render codecBody()}
  </BottomSheet>
{:else}
  <Modal
    isOpen={open}
    title={i18n.t('post.codec_guide_title')}
    size="md"
    {onclose}
  >
    {@render codecBody()}
  </Modal>
{/if}

<style>
  .codec-modal-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .codec-intro,
  .tab-note {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .segmented-wrapper {
    display: flex;
    width: 100%;
  }

  .segmented-wrapper :global(.segmented-control-container) {
    display: flex;
    width: 100%;
  }

  .segmented-wrapper :global(.tab-btn) {
    flex: 1;
    justify-content: center;
  }

  .tab-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .cmd-group {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .cmd-label {
    color: var(--text-secondary);
    font-size: var(--type-title-sm-size);
    line-height: var(--type-title-sm-line);
    font-weight: 500;
  }

  .cmd-box {
    display: flex;
    align-items: center;
    gap: 8px;
    align-self: stretch;
    padding: 8px 8px 8px 16px;
    border-radius: var(--radius-md);
    background: var(--surface-container-highest);
  }

  .cmd-text {
    flex: 1 1 auto;
    min-width: 0;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    word-break: break-all;
    user-select: all;
  }

  .copy-btn {
    display: grid;
    place-items: center;
    flex: none;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
  }

  .copy-btn :global(svg) {
    width: 18px;
    height: 18px;
  }

  .copy-btn :global(.is-done) {
    color: var(--accent-primary);
  }

  @media (hover: hover) {
    .copy-btn:hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }
  }
</style>
