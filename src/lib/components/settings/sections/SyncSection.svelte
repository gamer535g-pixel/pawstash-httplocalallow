<script lang="ts">
  import { syncState } from '$lib/state/syncState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import { syncStatusText } from '$lib/utils/syncStatus';

  let settings = $derived(ctl.settings);

  const PUSH_INTERVALS: [number, string][] = [
    [15, 'sync.interval_15s'],
    [30, 'sync.interval_30s'],
    [60, 'sync.interval_1m'],
    [120, 'sync.interval_2m'],
    [300, 'sync.interval_5m'],
    [900, 'sync.interval_15m'],
    [1800, 'sync.interval_30m']
  ];
  const PULL_INTERVALS: [number, string][] = [
    [30, 'sync.interval_30s'],
    [60, 'sync.interval_1m'],
    [120, 'sync.interval_2m'],
    [300, 'sync.interval_5m'],
    [600, 'sync.interval_10m'],
    [900, 'sync.interval_15m'],
    [1800, 'sync.interval_30m']
  ];

  async function setEnabled(enabled: boolean) {
    if (!(await ctl.save('sync_enabled', enabled))) return;
    try {
      await syncState.setEnabled(enabled);
    } catch (error) {
      notify.error(i18n.t('settings.save_failed'), error);
    }
  }

  async function setSessionSync(enabled: boolean) {
    if (!(await ctl.save('sync_pawchive_session', enabled))) return;
    const { configured, enabled: syncOn, unlocked } = syncState.status;
    if (configured && syncOn && unlocked) void syncState.sync();
  }
</script>

<SettingsGroup>
  <SettingItem
    title={i18n.t('profile.title')}
    summary={syncStatusText()}
    onclick={() => navigationState.navigateRoot('profile')}
  />
</SettingsGroup>

<SettingsGroup>
  <SettingItem
    title={i18n.t('sync.enable_sync')}
    description={i18n.t('sync.enable_sync_desc')}
    checked={settings.sync_enabled}
    onCheckedChange={(value) => void setEnabled(value)}
  />
  <SettingItem
    title={i18n.t('sync.sync_on_change')}
    description={i18n.t('sync.sync_on_change_desc')}
    checked={settings.sync_on_change}
    disabled={!settings.sync_enabled}
    onCheckedChange={(value) => void ctl.save('sync_on_change', value)}
  />
  {#if !settings.sync_on_change}
    <SettingChoice
      title={i18n.t('sync.push_interval')}
      description={i18n.t('sync.push_interval_desc')}
      options={PUSH_INTERVALS.map(([value, key]) => ({ value, label: i18n.t(key) }))}
      value={settings.sync_push_interval_seconds}
      disabled={!settings.sync_enabled}
      onchange={(value) => void ctl.save('sync_push_interval_seconds', value)}
    />
  {/if}
  <SettingItem
    title={i18n.t('sync.auto_sync')}
    description={i18n.t('sync.auto_sync_desc')}
    checked={settings.sync_auto}
    disabled={!settings.sync_enabled}
    onCheckedChange={(value) => void ctl.save('sync_auto', value)}
  />
  {#if settings.sync_auto}
    <SettingChoice
      title={i18n.t('sync.pull_interval')}
      description={i18n.t('sync.pull_interval_desc')}
      options={PULL_INTERVALS.map(([value, key]) => ({ value, label: i18n.t(key) }))}
      value={settings.sync_pull_interval_seconds}
      disabled={!settings.sync_enabled}
      onchange={(value) => void ctl.save('sync_pull_interval_seconds', value)}
    />
  {/if}
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_accounts')}>
  <SettingItem
    title={i18n.t('sync.sync_pawchive_session')}
    description={i18n.t('sync.sync_pawchive_session_desc')}
    checked={settings.sync_pawchive_session}
    disabled={!settings.sync_enabled}
    onCheckedChange={(value) => void setSessionSync(value)}
  />
  <SettingItem
    title={i18n.t('sync.persist_in_app_favorites_locally')}
    description={i18n.t('sync.persist_in_app_favorites_locally_desc')}
    checked={settings.persist_in_app_favorites_locally ?? true}
    onCheckedChange={(value) => void ctl.save('persist_in_app_favorites_locally', value)}
  />
</SettingsGroup>
