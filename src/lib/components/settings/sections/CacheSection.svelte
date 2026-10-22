<script lang="ts">
  import { onMount } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { formatBytes } from '$lib/utils/formatters';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import StorageBar from '../StorageBar.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconDelete from '~icons/fluent/delete-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  let stats = $derived(ctl.cacheStats);
  let confirmClearAll = $state(false);

  onMount(() => {
    void ctl.loadCacheStats();
  });
</script>

<SettingsGroup>
  <SettingItem title={i18n.t('settings.cache_usage')} description={i18n.t('settings.cache_usage_desc')} stacked>
    {#if stats}
      <StorageBar {stats} limitMb={ctl.settings.cache_max_mb} {formatBytes} />
    {:else}
      <IconLoading aria-label={i18n.t('settings.cache_section')} />
    {/if}
  </SettingItem>

  <SettingChoice
    title={i18n.t('settings.cache_limit')}
    description={i18n.t('settings.cache_limit_desc')}
    options={[64, 128, 256, 512, 1024, 2048].map((mb) => ({ value: mb, label: formatBytes(mb * 1024 * 1024, 0) }))}
    value={ctl.settings.cache_max_mb}
    onchange={(mb) => void ctl.save('cache_max_mb', mb)}
  />

  <SettingItem
    title={i18n.t('settings.cache_clear')}
    description={i18n.t('settings.cache_clear_desc')}
    action
    disabled={Boolean(ctl.cacheBusy) || !stats || stats.total_bytes === 0}
    onclick={() => void ctl.clearCache('images')}
  />
  <SettingItem
    title={i18n.t('settings.cache_clear_all')}
    description={i18n.t('settings.cache_clear_all_desc')}
    action
    danger
    disabled={Boolean(ctl.cacheBusy) || !stats || stats.total_bytes + stats.metadata_bytes === 0}
    onclick={() => (confirmClearAll = true)}
  />
</SettingsGroup>

<ConfirmDialog
  isOpen={confirmClearAll}
  title={i18n.t('settings.cache_clear_all')}
  description={i18n.t('settings.cache_clear_all_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  loading={ctl.cacheBusy === 'all'}
  onconfirm={() => {
    confirmClearAll = false;
    void ctl.clearCache('all');
  }}
  onclose={() => (confirmClearAll = false)}
/>
