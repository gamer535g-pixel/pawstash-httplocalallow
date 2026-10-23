<script lang="ts">
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes, formatSpeed } from '$lib/utils/formatters';
  import { notify } from '$lib/utils/toast';
  import { tooltip } from '$lib/motion';
  import LinearProgress from '$lib/components/ui/LinearProgress.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import IconPause from '~icons/fluent/pause-20-regular';
  import IconPlay from '~icons/fluent/play-20-regular';
  import IconRetry from '~icons/fluent/arrow-counterclockwise-20-regular';
  import IconDismiss from '~icons/fluent/dismiss-20-regular';
  import IconError from '~icons/fluent/error-circle-20-regular';

  interface Props {
    onshowfailed?: () => void;
  }

  let { onshowfailed }: Props = $props();

  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  let ratio = $derived(ratios[configState.settings.grid_aspect_ratio]);

  let stats = $derived(downloadState.queueStats);
  let running = $derived(stats.downloading + stats.queued > 0);
  let cancellable = $derived(stats.downloading + stats.queued + stats.paused > 0);
  let percent = $derived(
    downloadState.activeProgress === null ? null : Math.floor(downloadState.activeProgress * 100)
  );
  let detail = $derived(
    [
      stats.totalBytes > 0 ? `${formatBytes(stats.downloadedBytes)} / ${formatBytes(stats.totalBytes)}` : '',
      stats.speedBps > 0 ? formatSpeed(stats.speedBps) : ''
    ]
      .filter(Boolean)
      .join(' · ')
  );
  let status = $derived(
    [
      stats.downloading > 0 ? i18n.t('downloads.queue_downloading', { count: stats.downloading }) : '',
      stats.queued > 0 ? i18n.t('downloads.queue_queued', { count: stats.queued }) : '',
      stats.paused > 0 ? i18n.t('downloads.queue_paused', { count: stats.paused }) : ''
    ]
      .filter(Boolean)
      .join(' · ')
  );
  let toggleLabel = $derived(i18n.t(running ? 'downloads.pause_all' : 'downloads.resume_all'));
  let busy = $state(false);
  let confirmCancel = $state(false);

  async function run(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    try {
      await action();
    } catch (error) {
      notify.error(i18n.t('downloads.action_error'), error);
    } finally {
      busy = false;
    }
  }

  async function cancelAll() {
    await run(() => downloadState.cancelAll());
    confirmCancel = false;
  }
</script>

<article class="grid-tile queue-tile" style:aspect-ratio={ratio} aria-label={i18n.t('downloads.queue_title')}>
  <div class="queue-body">
    <h2 class="queue-title">{i18n.t('downloads.queue_title')}</h2>

    {#if cancellable}
      <div class="queue-readout">
        {#if percent !== null}<span class="queue-percent">{percent}%</span>{/if}
        {#if detail}<span class="queue-detail">{detail}</span>{/if}
      </div>
      {#if status}<p class="queue-status">{status}</p>{/if}
      <LinearProgress
        value={percent ?? 0}
        indeterminate={percent === null && stats.downloading > 0}
        label={i18n.t('downloads.queue_title')}
      />
    {/if}

    <div class="queue-actions">
      {#if stats.failed > 0}
        <button type="button" class="queue-failed" onclick={() => onshowfailed?.()}>
          <IconError />
          <span>{i18n.t('downloads.queue_failed', { count: stats.failed })}</span>
        </button>
        <button
          type="button"
          class="grid-tile-action queue-action"
          disabled={busy}
          onclick={() => void run(() => downloadState.retryFailed())}
          use:tooltip={i18n.t('downloads.retry_failed')}
          aria-label={i18n.t('downloads.retry_failed')}
        >
          <IconRetry />
        </button>
      {/if}
      {#if cancellable}
        <button
          type="button"
          class="grid-tile-action queue-action is-tonal"
          disabled={busy}
          onclick={() => void run(() => (running ? downloadState.pauseAll() : downloadState.resumeAll()))}
          use:tooltip={toggleLabel}
          aria-label={toggleLabel}
        >
          {#if running}<IconPause />{:else}<IconPlay />{/if}
        </button>
        <button
          type="button"
          class="grid-tile-action queue-action"
          disabled={busy}
          onclick={() => (confirmCancel = true)}
          use:tooltip={i18n.t('downloads.cancel_all')}
          aria-label={i18n.t('downloads.cancel_all')}
        >
          <IconDismiss />
        </button>
      {/if}
    </div>
  </div>
</article>

<ConfirmDialog
  isOpen={confirmCancel}
  title={i18n.t('downloads.cancel_all_title')}
  description={i18n.t('downloads.cancel_all_desc')}
  confirmLabel={i18n.t('downloads.cancel_all')}
  confirmVariant="danger"
  confirmIcon={IconDismiss}
  loading={busy}
  onconfirm={() => void cancelAll()}
  onclose={() => (confirmCancel = false)}
/>

<style>
  .queue-tile {
    background: var(--surface-container);
  }

  .queue-body {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: calc(8px * var(--grid-scale, 1));
    padding: var(--tile-inset);
    color: var(--text-primary);
  }

  .queue-title {
    margin: 0;
    font-size: calc(var(--type-title-sm-size) * var(--grid-scale, 1));
    line-height: calc(var(--type-title-sm-line) * var(--grid-scale, 1));
    font-weight: 500;
  }

  .queue-readout {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    flex: 1 1 auto;
    min-height: 0;
  }

  .queue-percent {
    font-size: calc(var(--type-headline-size) * var(--grid-scale, 1));
    line-height: calc(var(--type-headline-line) * var(--grid-scale, 1));
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }

  .queue-detail,
  .queue-status {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: calc(var(--type-label-md-size) * var(--grid-scale, 1));
    line-height: calc(var(--type-label-md-line) * var(--grid-scale, 1));
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .queue-status {
    margin: 0;
  }

  .queue-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: calc(4px * var(--grid-scale, 1));
    margin-top: auto;
  }

  .queue-failed {
    display: inline-flex;
    align-items: center;
    gap: calc(6px * var(--grid-scale, 1));
    min-width: 0;
    height: var(--tile-action-size);
    margin-inline-end: auto;
    padding-inline: calc(10px * var(--grid-scale, 1));
    border: none;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--status-error);
    font: inherit;
    font-size: calc(var(--type-label-lg-size) * var(--grid-scale, 1));
    font-weight: 500;
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .queue-failed span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .queue-failed :global(svg) {
    flex-shrink: 0;
    width: var(--tile-action-icon, 18px);
    height: var(--tile-action-icon, 18px);
  }

  .queue-failed:active {
    background: var(--state-layer-pressed);
  }

  @media (hover: hover) {
    .queue-failed:hover {
      background: var(--state-layer-hover);
    }
  }

  .queue-action {
    color: var(--text-secondary);
  }

  .queue-action.is-tonal {
    background: var(--accent-container);
    color: var(--accent-on-container);
  }
</style>
