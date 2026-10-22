<script lang="ts">
  import { updateState } from '$lib/state/updateState.svelte';
  import { i18n } from '$lib/i18n';
  import { openExternalUrl } from '$lib/utils/ipc';
  import { APP_VERSION } from '$lib/version';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import { notify } from '$lib/utils/toast';
  import { copyText } from '$lib/utils/clipboard';
  import { COMMUNITY_LINKS, TELEGRAM, openCommunity } from '$lib/utils/community';
  import { readRecentLogs, openLogsFolder, clearLogs } from '$lib/utils/logger';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconArrowReset from '~icons/fluent/arrow-reset-24-regular';
  import IconDelete from '~icons/fluent/delete-24-regular';

  let settings = $derived(ctl.settings);
  let confirmReset = $state(false);
  let confirmWipe = $state(false);
  let logsBusy = $state(false);

  let versionSummary = $derived(
    updateState.checking
      ? i18n.t('settings.checking_updates')
      : updateState.info?.available
        ? i18n.t('settings.update_available_summary', { version: updateState.info.latest_version })
        : updateState.info
          ? i18n.t('settings.up_to_date_summary')
          : i18n.t('settings.check_updates_now')
  );

  function openVersion() {
    if (updateState.info?.available) updateState.openModal();
    else void updateState.check(false);
  }

  async function copyLogs() {
    logsBusy = true;
    try {
      copyText((await readRecentLogs(500)).trim(), i18n.t('settings.logs_copied'));
    } catch (error) {
      notify.error(i18n.t('settings.logs_copy_failed'), error);
    } finally {
      logsBusy = false;
    }
  }

  async function openFolder() {
    try {
      await openLogsFolder();
    } catch (error) {
      notify.error(i18n.t('settings.open_logs_folder_failed'), error);
    }
  }

  async function removeLogs() {
    logsBusy = true;
    try {
      await clearLogs();
      notify.success(i18n.t('settings.logs_cleared'), { glyph: 'cleared' });
    } catch (error) {
      notify.error(i18n.t('settings.logs_clear_failed'), error);
    } finally {
      logsBusy = false;
    }
  }
</script>

<SettingsGroup label={i18n.t('settings.updates_section')}>
  <SettingItem title={`Pawstash ${APP_VERSION}`} summary={versionSummary} disabled={updateState.checking} onclick={openVersion} />

  <SettingItem
    title={i18n.t('settings.auto_check_updates')}
    description={i18n.t('settings.auto_check_updates_desc')}
    checked={settings.auto_check_updates ?? true}
    onCheckedChange={(value) => void ctl.save('auto_check_updates', value)}
  />

  <SettingChoice
    title={i18n.t('settings.update_channel')}
    description={i18n.t('settings.update_channel_desc')}
    options={[
      { value: false, label: i18n.t('settings.channel_stable') },
      { value: true, label: i18n.t('settings.channel_prerelease') }
    ]}
    value={settings.include_prereleases ?? false}
    onchange={(value) => void ctl.save('include_prereleases', value)}
  />
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.community')}>
  {#each COMMUNITY_LINKS as link (link.id)}
    <SettingItem title={link.label} external onclick={() => openCommunity(link)} />
  {/each}
  <SettingItem
    title={i18n.t('settings.author')}
    summary="nichind.dev"
    external
    onclick={() => void openExternalUrl('https://nichind.dev')}
  />
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_troubleshooting')}>
  <SettingItem
    title={i18n.t('settings.get_help')}
    description={i18n.t('settings.get_help_desc')}
    external
    onclick={() => openCommunity(TELEGRAM)}
  />
  <SettingItem
    title={i18n.t('settings.copy_logs')}
    description={i18n.t('settings.copy_logs_desc')}
    action
    disabled={logsBusy}
    onclick={() => void copyLogs()}
  />
  <SettingItem title={i18n.t('settings.open_logs_folder')} action onclick={() => void openFolder()} />
  <SettingItem title={i18n.t('settings.clear_logs')} action disabled={logsBusy} onclick={() => void removeLogs()} />
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_reset')}>
  <SettingItem
    title={i18n.t('settings.reset_all')}
    description={i18n.t('settings.reset_all_desc')}
    action
    danger
    disabled={ctl.resetPending}
    onclick={() => (confirmReset = true)}
  />
  <SettingItem
    title={i18n.t('settings.wipe_all_data')}
    description={i18n.t('settings.wipe_all_data_desc')}
    action
    danger
    disabled={ctl.wipePending}
    onclick={() => (confirmWipe = true)}
  />
</SettingsGroup>

<ConfirmDialog
  isOpen={confirmReset}
  title={i18n.t('settings.reset_all')}
  description={i18n.t('settings.reset_all_confirm')}
  confirmLabel={i18n.t('common.reset')}
  confirmVariant="danger"
  confirmIcon={IconArrowReset}
  loading={ctl.resetPending}
  onconfirm={async () => {
    if (await ctl.resetAll()) confirmReset = false;
  }}
  onclose={() => (confirmReset = false)}
/>

<ConfirmDialog
  isOpen={confirmWipe}
  title={i18n.t('settings.wipe_all_data')}
  description={i18n.t('settings.wipe_all_data_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  loading={ctl.wipePending}
  onconfirm={async () => {
    if (await ctl.wipeAll()) confirmWipe = false;
  }}
  onclose={() => (confirmWipe = false)}
/>

