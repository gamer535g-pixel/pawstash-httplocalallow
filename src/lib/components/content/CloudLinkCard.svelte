<script lang="ts">
  import { untrack } from 'svelte';
  import { i18n } from '$lib/i18n';
  import type { CloudFolderResult, CloudLockKind } from '$lib/types/cloud';
  import type { Post } from '$lib/types/content';
  import { apiPasswordHints, apiResolveCloudLink, apiUnlockCloudLink, openExternalUrl } from '$lib/utils/ipc';
  import { cloudLinkContext, cloudLockKind } from '$lib/utils/cloudLink';
  import { logger } from '$lib/utils/logger';
  import { tooltip } from '$lib/motion';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import CloudFolderModal, { fileSummary } from './CloudFolderModal.svelte';
  import SecretUnlockPanel from './SecretUnlockPanel.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import IconOpen from '~icons/fluent/open-24-regular';
  import IconFolder from '~icons/fluent/folder-open-24-regular';
  import IconRetry from '~icons/fluent/arrow-clockwise-24-regular';
  import IconCloud from '~icons/fluent/cloud-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    url: string;
    post?: Post | null;
    lock?: CloudLockKind | null;
    onresolved?: (result: CloudFolderResult) => void;
  }

  let { url, post = null, lock = null, onresolved }: Props = $props();

  let loading = $state(false);
  let resolvedData = $state<CloudFolderResult | null>(null);
  let error = $state<string | null>(null);
  let lockKind = $derived(lock);
  let hints = $state<string[]>([]);
  let modalOpen = $state(false);
  let scrapingEnabled = $derived(configState.settings.cloud_scraping_enabled !== false);
  let context = $derived(post ? cloudLinkContext(post) : undefined);

  let host = $derived.by(() => {
    try {
      return new URL(url).hostname.replace(/^www\./, '');
    } catch {
      return i18n.t('post.cloud_generic');
    }
  });

  async function resolveLink(openModalAfter = false) {
    if (loading || !scrapingEnabled || lockKind) return;
    if (resolvedData) {
      if (openModalAfter) modalOpen = true;
      return;
    }
    loading = true;
    error = null;
    try {
      resolvedData = await apiResolveCloudLink(url, context);
      onresolved?.(resolvedData);
      if (openModalAfter) modalOpen = true;
    } catch (err) {
      const kind = cloudLockKind(err);
      if (kind) {
        lockKind = kind;
      } else {
        error = i18n.t('post.cloud_resolve_failed');
        logger.warn(`Cloud link resolution failed for ${url}`, err);
      }
    } finally {
      loading = false;
    }
  }

  async function unlock(secret: string) {
    const result = await apiUnlockCloudLink(url, secret, context);
    resolvedData = result;
    lockKind = null;
    onresolved?.(result);
    modalOpen = true;
  }

  $effect(() => {
    const text = lockKind === 'password' ? context?.post_text : undefined;
    if (!text) {
      hints = [];
      return;
    }
    let current = true;
    apiPasswordHints(text)
      .then((found) => {
        if (current) hints = found;
      })
      .catch((err) => logger.warn('Password hint extraction failed', err));
    return () => {
      current = false;
    };
  });

  $effect(() => {
    if (scrapingEnabled && url && !lockKind) {
      untrack(() => {
        if (!resolvedData && !loading && !error) void resolveLink(false);
      });
    }
  });
</script>

<article class="cloud-link" data-cloud-url={url}>
  <div class="cloud-link-row">
    <span class="cloud-link-icon" aria-hidden="true"><IconCloud /></span>

    <div class="cloud-link-body">
      {#if resolvedData}
        <span class="cloud-link-title">{resolvedData.title}</span>
        <span class="cloud-link-meta">{host} · {fileSummary(resolvedData.total_files, resolvedData.total_size)}</span>
      {:else}
        <span class="cloud-link-url" use:tooltip={url}>{url}</span>
        {#if !lockKind}
          <span class="cloud-link-meta" class:is-error={Boolean(error)}>
            {scrapingEnabled ? (error ?? i18n.t('post.cloud_inspecting')) : i18n.t('post.cloud_scraping_off')}
          </span>
        {/if}
      {/if}
    </div>

    <div class="cloud-link-actions" data-control-size={layoutState.isMobile ? 'md' : 'sm'}>
      {#if !lockKind && (scrapingEnabled || resolvedData)}
        <Button variant="tonal" disabled={loading} onclick={() => void resolveLink(true)}>
          {#if loading}
            <IconLoading />
          {:else if error}
            <IconRetry />
          {:else}
            <IconFolder />
          {/if}
          {i18n.t(error ? 'common.retry' : 'post.cloud_browse')}
        </Button>
      {/if}
      <Button
        variant="ghost"
        class="btn-icon"
        tooltip={i18n.t('post.open_in_browser')}
        onclick={() => void openExternalUrl(url)}
      >
        <IconOpen />
      </Button>
    </div>
  </div>

  {#if lockKind}
    {#key lockKind}
      <SecretUnlockPanel
        kind={lockKind}
        title={i18n.t(lockKind === 'password' ? 'cloud.link_password_locked' : 'cloud.link_key_required')}
        {hints}
        {unlock}
      />
    {/key}
  {/if}
</article>

<CloudFolderModal
  folder={resolvedData}
  {post}
  open={modalOpen}
  onclose={() => (modalOpen = false)}
/>

<style>
  .cloud-link {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    padding: calc(var(--space-3) * var(--ui-scale, 1)) calc(var(--space-4) * var(--ui-scale, 1));
    border-radius: calc(var(--radius-lg) * var(--ui-scale, 1));
    background: var(--bg-card);
  }

  .cloud-link-row {
    display: flex;
    align-items: center;
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    min-width: 0;
  }

  .cloud-link-icon {
    display: grid;
    flex: none;
    place-items: center;
    width: calc(var(--file-row-thumb) * var(--ui-scale, 1));
    height: calc(var(--file-row-thumb) * var(--ui-scale, 1));
    border-radius: calc(var(--radius-sm) * var(--ui-scale, 1));
    background: var(--bg-card);
    color: var(--text-secondary);
  }

  .cloud-link-icon :global(svg) {
    width: calc(var(--file-row-icon) * var(--ui-scale, 1));
    height: calc(var(--file-row-icon) * var(--ui-scale, 1));
  }

  .cloud-link-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
    min-width: 0;
  }

  .cloud-link-title,
  .cloud-link-url {
    overflow: hidden;
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    font-weight: var(--font-weight-medium);
    color: var(--text-primary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cloud-link-url {
    font-family: var(--font-mono);
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
  }

  .cloud-link-meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .cloud-link-meta.is-error {
    color: var(--status-error);
  }

  .cloud-link-actions {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: calc(var(--space-1) * var(--ui-scale, 1));
  }
</style>
