<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { tooltip } from '$lib/motion';
  import { APP_VERSION, BUILD_TIME, COMMIT_HASH } from '$lib/version';

  let builtOn = $derived.by(() => {
    const date = new Date(BUILD_TIME);
    if (Number.isNaN(date.getTime())) return BUILD_TIME;
    return i18n.date(date, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
  });
</script>

<p class="version-line" use:tooltip={`${i18n.t('settings.built_on')} ${builtOn}`}>
  Pawstash {APP_VERSION} · {COMMIT_HASH}
</p>

<style>
  .version-line {
    flex-shrink: 0;
    margin: 0;
    padding: 12px 24px 16px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--type-label-sm-size);
    line-height: var(--type-label-sm-line);
    user-select: text;
  }
</style>
