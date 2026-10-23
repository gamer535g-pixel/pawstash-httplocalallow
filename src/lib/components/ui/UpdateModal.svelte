<script lang="ts">
  import Modal from '$lib/components/ui/Modal.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import LinearProgress from './LinearProgress.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import appIcon from '$lib/assets/app-icon.png';
  import { tooltip } from '$lib/motion';
  import { openExternalUrl } from '$lib/utils/ipc';
  import { COMMUNITY_FEATURED, openCommunity } from '$lib/utils/community';

  type Kind = 'new' | 'fixes' | 'improvements';
  const KIND_LABEL: Record<Kind, string> = {
    new: 'update.section_new',
    fixes: 'update.section_fixes',
    improvements: 'update.section_improvements'
  };

  const COMMIT_TYPES = ['feat', 'fix', 'perf', 'refactor', 'chore', 'docs'];
  const SEGMENT_SPLIT = new RegExp(`;\\s*(?=(?:${COMMIT_TYPES.join('|')})\\b)`, 'i');

  interface Note {
    text: string;
    hash?: string;
  }

  let sections = $derived.by(() => {
    const groups: Record<Kind, Note[]> = { new: [], fixes: [], improvements: [] };
    for (const raw of (updateState.info?.release_notes ?? '').split('\n')) {
      const hash = raw.match(/\(([0-9a-f]{7,40})\)\s*$/i)?.[1]?.slice(0, 7);
      const line = raw
        .trim()
        .replace(/^[-*•]\s*/, '')
        .replace(/\s+by\s+@\S+\s+in\s+https?:\/\/\S+$/i, '')
        .replace(/\s*\((?:[0-9a-f]{7,40}|v\d[^)]*|#\d+)\)/gi, '')
        .trim();
      if (!line || line.startsWith('#') || /^\**full changelog/i.test(line) || /^(pawstash\s+)?v?\d+(\.\d+)+(-[\w.]+)?$/i.test(line)) continue;
      for (const segment of line.split(SEGMENT_SPLIT)) {
        const match = segment.match(/^(\w+)(?:\([^)]*\))?!?:?\s+(.*)$/);
        const type = match?.[1].toLowerCase();
        const known = type && COMMIT_TYPES.includes(type);
        const text = (known ? match![2] : segment).trim();
        if (!text) continue;
        const kind: Kind = type === 'feat' ? 'new' : type === 'fix' ? 'fixes' : 'improvements';
        groups[kind].push({ text: text.charAt(0).toUpperCase() + text.slice(1), hash });
      }
    }
    return (Object.keys(groups) as Kind[]).filter((kind) => groups[kind].length).map((kind) => ({ kind, items: groups[kind] }));
  });

  let repoUrl = $derived(updateState.info?.release_url.split('/releases')[0] ?? '');

  let meta = $derived(
    [
      i18n.t('update.installed', { version: updateState.info?.current_version ?? '' }),
      updateState.info?.asset_size ? formatBytes(updateState.info.asset_size) : '',
      updateState.info?.is_prerelease ? i18n.t('update.prerelease') : ''
    ]
      .filter(Boolean)
      .join(' · ')
  );

  let busy = $derived(updateState.downloading);

  let contributors = $derived(updateState.info?.contributors ?? []);
  let contributorNames = $derived.by(() => {
    const names = contributors.slice(0, 3).map((c) => c.login).join(', ');
    const rest = contributors.length - 3;
    return rest > 0 ? `${names} ${i18n.t('update.and_more', { count: rest })}` : names;
  });
</script>

{#snippet updateBody()}
  <div class="update-body">
    <div class="update-hero">
      <img class="update-icon" src={appIcon} alt="" />
      <div class="update-hero-text">
        <p class="update-name">Pawstash {updateState.info?.latest_version}</p>
        <p class="update-meta">{meta}</p>
      </div>
    </div>

    {#each sections as section (section.kind)}
      <section class="update-section">
        <h3 class="update-section-title">{i18n.t(KIND_LABEL[section.kind])}</h3>
        <ul class="update-list">
          {#each section.items as item, index (index)}
            <li>
              {item.text}
              {#if item.hash && repoUrl}
                <button type="button" class="update-hash" onclick={() => void openExternalUrl(`${repoUrl}/commit/${item.hash}`)}>{item.hash}</button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}

    {#if contributors.length}
      <section class="update-section">
        <h3 class="update-section-title">{i18n.t('update.contributors')}</h3>
        <div class="update-contributors">
          {#each contributors as contributor (contributor.login)}
            <button
              type="button"
              class="update-contributor"
              onclick={() => void openExternalUrl(contributor.html_url)}
              use:tooltip={`${contributor.login} · ${i18n.t('counts.commits', { count: contributor.commits })}`}
              aria-label={contributor.login}
            >
              <img src={`${contributor.avatar_url}&s=80`} alt="" loading="lazy" decoding="async" />
            </button>
          {/each}
        </div>
        <p class="update-contributor-names">{contributorNames}</p>
      </section>
    {/if}

    <div class="update-discuss">
      <span class="update-discuss-label">{i18n.t('update.discuss')}</span>
      {#each COMMUNITY_FEATURED as link (link.id)}
        <button type="button" class="update-chip" onclick={() => openCommunity(link)}>
          <link.icon />
          {link.label}
        </button>
      {/each}
    </div>

    {#if updateState.info?.release_url}
      <button type="button" class="update-link" onclick={() => updateState.openReleasePage()}>
        {i18n.t('update.full_notes')}
      </button>
    {/if}
  </div>
{/snippet}

{#snippet updateActions()}
  {#if busy}
    <div class="update-progress">
      <div class="update-progress-info" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={updateState.downloadProgress}>
        <div class="update-progress-row">
          <span class="update-progress-label">{updateState.installing ? i18n.t('update.installing') : i18n.t('update.downloading')}</span>
          {#if !updateState.installing}
            <span class="update-progress-figures">
              {updateState.downloadProgress}%{updateState.speedText ? ` · ${updateState.speedText}` : ''}
            </span>
          {/if}
        </div>
        <LinearProgress value={updateState.downloadProgress} indeterminate={updateState.installing} />
      </div>
      {#if !updateState.installing}
        <Button variant="ghost" size="md" class="update-cancel" onclick={() => updateState.cancelDownload()}>
          {i18n.t('common.cancel')}
        </Button>
      {/if}
    </div>
  {:else}
    <div class="dialog-actions">
      <Button variant="ghost" size="md" onclick={() => updateState.closeModal()}>
        {i18n.t('update.later')}
      </Button>
      <Button variant="ghost" data-tone="accent" size="md" onclick={() => updateState.startInAppUpdate()}>
        {i18n.t('update.install')}
      </Button>
    </div>
  {/if}
{/snippet}

{#if updateState.modalOpen && updateState.info}
  {#if layoutState.isMobile}
    <BottomSheet
      isOpen={updateState.modalOpen}
      title={i18n.t('update.modal_title')}
      closeLabel={i18n.t('common.close')}
      onclose={() => updateState.closeModal()}
    >
      {@render updateBody()}
      {#snippet footer()}{@render updateActions()}{/snippet}
    </BottomSheet>
  {:else}
    <Modal
      isOpen={updateState.modalOpen}
      title={i18n.t('update.modal_title')}
      size="md"
      borderlessFooter
      onclose={() => updateState.closeModal()}
    >
      {@render updateBody()}
      {#snippet footer()}{@render updateActions()}{/snippet}
    </Modal>
  {/if}
{/if}

<style>
  .update-body {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .update-hero {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .update-icon {
    flex: none;
    width: 56px;
    height: 56px;
    padding: 8px;
    border-radius: var(--radius-lg);
    background: var(--surface-container-high);
    box-sizing: border-box;
  }

  .update-hero-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .update-name,
  .update-meta {
    margin: 0;
  }

  .update-name {
    color: var(--text-primary);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
  }

  .update-meta {
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .update-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .update-section-title {
    margin: 0;
    color: var(--text-primary);
    font-size: var(--type-title-sm-size);
    line-height: var(--type-title-sm-line);
    font-weight: 500;
  }

  .update-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .update-list li {
    position: relative;
    padding-inline-start: 18px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    overflow-wrap: anywhere;
  }

  .update-list li::before {
    content: '';
    position: absolute;
    top: calc(var(--type-body-md-line) / 2 - 2.5px);
    left: 4px;
    width: 5px;
    height: 5px;
    border-radius: var(--radius-full);
    background: var(--text-muted);
  }

  .update-contributors {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .update-contributor {
    width: 36px;
    height: 36px;
    padding: 0;
    overflow: hidden;
    border: 0;
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    cursor: pointer;
  }

  .update-contributor img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  @media (hover: hover) {
    .update-contributor:hover {
      outline: 2px solid var(--outline-variant);
      outline-offset: 1px;
    }
  }

  .update-contributor-names {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .update-hash {
    margin-inline-start: 4px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: var(--type-body-sm-size);
    cursor: pointer;
  }

  @media (hover: hover) {
    .update-hash:hover {
      color: var(--accent-on-surface);
      text-decoration: underline;
    }
  }

  .update-discuss {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .update-discuss-label {
    margin-inline-end: 4px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }

  .update-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-full);
    background: var(--surface-container-highest);
    color: var(--text-primary);
    font: inherit;
    font-size: var(--type-label-lg-size);
    font-weight: 500;
    cursor: pointer;
  }

  .update-chip :global(svg) {
    width: 16px;
    height: 16px;
  }

  @media (hover: hover) {
    .update-chip:hover {
      background: color-mix(in srgb, var(--text-primary) 8%, var(--surface-container-highest));
    }
  }

  .update-link {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-on-surface);
    font: inherit;
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
    font-weight: 500;
    cursor: pointer;
  }

  @media (hover: hover) {
    .update-link:hover {
      text-decoration: underline;
    }
  }

  .update-progress {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: var(--control-height-md);
  }

  .update-progress-info {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .update-progress-label {
    color: var(--text-primary);
  }

  .update-progress :global(.update-cancel) {
    flex: none;
    min-width: 0;
  }

  .update-progress-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .update-progress-figures {
    font-variant-numeric: tabular-nums;
  }

</style>
