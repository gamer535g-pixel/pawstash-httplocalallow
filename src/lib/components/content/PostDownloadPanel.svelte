<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { DownloadItem } from '$lib/types/download';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import { notify } from '$lib/utils/toast';
  import Button from '$lib/components/ui/Button.svelte';
  import LinearProgress from '$lib/components/ui/LinearProgress.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import IconDownload from '~icons/fluent/arrow-download-24-regular';
  import IconFolder from '~icons/fluent/folder-open-24-regular';
  import IconPause from '~icons/fluent/pause-24-regular';
  import IconPlay from '~icons/fluent/play-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  interface Props {
    jobs: (DownloadItem | undefined)[];
    totalBytes: number;
    starting: boolean;
    ondownload: () => void;
    onopenfolder: () => void;
    trailing?: Snippet;
  }

  let { jobs, totalBytes, starting, ondownload, onopenfolder, trailing }: Props = $props();

  const ACTIVE = ['queued', 'resolving', 'downloading'];

  let tracked = $derived(jobs.filter((job): job is DownloadItem => Boolean(job)));
  let completed = $derived(tracked.filter((job) => job.status === 'completed'));
  let active = $derived(tracked.filter((job) => ACTIVE.includes(job.status)));
  let paused = $derived(tracked.filter((job) => job.status === 'paused'));
  let unfinished = $derived(tracked.filter((job) => job.status !== 'completed'));
  let total = $derived(jobs.length);
  let remaining = $derived(total - completed.length);
  let inFlight = $derived(active.length + paused.length > 0);
  let allDone = $derived(total > 0 && completed.length === total);

  let bytesTotal = $derived.by(() => {
    const known = tracked.reduce((sum, job) => sum + (job.total_bytes || 0), 0);
    return Math.max(known, totalBytes);
  });
  let bytesDone = $derived(
    tracked.reduce((sum, job) => sum + (job.status === 'completed' ? job.total_bytes || job.downloaded_bytes : job.downloaded_bytes), 0)
  );
  let percent = $derived(bytesTotal > 0 ? (bytesDone / bytesTotal) * 100 : (completed.length / Math.max(total, 1)) * 100);

  let confirmDelete = $state(false);
  let busy = $state(false);

  async function each(items: DownloadItem[], operation: (id: string) => Promise<void>) {
    busy = true;
    try {
      await Promise.all(items.map((job) => operation(job.id)));
    } catch (error) {
      notify.error(i18n.t('downloads.action_error'), error);
    } finally {
      busy = false;
    }
  }
</script>

  {#if inFlight}
    <div class="post-download-actions">
      <div class="post-download-progress" role="status">
        <div class="post-download-progress__row">
          <span>{i18n.t(active.length ? 'post.download_progress' : 'post.download_paused', { done: completed.length, total })}</span>
          {#if bytesTotal > 0}
            <span class="post-download-progress__bytes">{formatBytes(bytesDone)} / {formatBytes(bytesTotal)}</span>
          {/if}
        </div>
        <LinearProgress value={percent} />
      </div>
      {#if active.length}
        <Button
          variant="tonal"
          class="btn-icon"
          aria-label={i18n.t('downloads.pause')}
          title={i18n.t('downloads.pause')}
          disabled={busy}
          onclick={() => void each(active, (id) => downloadState.pause(id))}
        >
          <IconPause />
        </Button>
      {:else}
        <Button
          variant="tonal"
          class="btn-icon"
          aria-label={i18n.t('downloads.resume')}
          title={i18n.t('downloads.resume')}
          disabled={busy}
          onclick={() => void each(paused, (id) => downloadState.resume(id))}
        >
          <IconPlay />
        </Button>
      {/if}
      <Button
        variant="ghost"
        class="btn-icon"
        aria-label={i18n.t('downloads.cancel')}
        title={i18n.t('downloads.cancel')}
        disabled={busy}
        onclick={() => void each(unfinished, (id) => downloadState.remove(id))}
      >
        <IconDismiss />
      </Button>
      {@render trailing?.()}
    </div>
  {:else}
    <div class="post-download-actions">
      {#if allDone}
        <Button variant="tonal" onclick={onopenfolder}>
          <IconFolder />
          <span>{i18n.t('downloads.open_post_folder')}</span>
        </Button>
      {:else}
        <Button variant="tonal" disabled={starting} onclick={ondownload}>
          {#if starting}<IconLoading />{:else}<IconDownload />{/if}
          <span>
            {#if completed.length > 0}
              {i18n.t('post.download_remaining', { count: remaining })}
            {:else}
              {i18n.t('post.download_all')}{totalBytes > 0 ? ` · ${formatBytes(totalBytes)}` : ''}
            {/if}
          </span>
        </Button>
        {#if completed.length > 0}
          <Button variant="ghost" onclick={onopenfolder}>
            <IconFolder />
            <span>{i18n.t('downloads.open_post_folder')}</span>
          </Button>
        {/if}
      {/if}
      {@render trailing?.()}
      {#if completed.length > 0}
        <Button
          variant="ghost"
          class="btn-icon post-download-delete"
          aria-label={i18n.t('post.delete_downloaded')}
          title={i18n.t('post.delete_downloaded')}
          onclick={() => (confirmDelete = true)}
        >
          <IconDelete />
        </Button>
      {/if}
    </div>
  {/if}

<ConfirmDialog
  isOpen={confirmDelete}
  title={i18n.t('post.delete_downloaded_title')}
  description={i18n.t('post.delete_downloaded_desc', { count: completed.length })}
  confirmLabel={i18n.t('downloads.remove')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  loading={busy}
  onconfirm={() => void each(completed, (id) => downloadState.remove(id)).then(() => (confirmDelete = false))}
  onclose={() => (confirmDelete = false)}
/>

<style>

  .post-download-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .post-download-progress {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    width: min(320px, 100%);
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
  }

  .post-download-progress__row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-primary);
    font-size: var(--type-label-md-size);
    line-height: var(--type-label-md-line);
    white-space: nowrap;
  }

  .post-download-progress__bytes {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .post-download-actions :global(.post-download-delete) {
    color: var(--text-secondary);
  }

  @media (hover: hover) {
    .post-download-actions :global(.post-download-delete:hover) {
      color: var(--status-error);
    }
  }
</style>
