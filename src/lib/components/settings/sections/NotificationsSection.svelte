<script lang="ts">
  import { i18n } from '$lib/i18n';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';

  let settings = $derived(ctl.settings);
  // Android's progress notification keeps downloads alive, so one switch drives both the master and completion flags.
  let enabled = $derived(settings.notifications_enabled && settings.notifications_download_completed);

  async function setEnabled(value: boolean) {
    if (!(await ctl.save('notifications_enabled', value))) return;
    if (value && !settings.notifications_download_completed) await ctl.save('notifications_download_completed', true);
  }
</script>

<SettingsGroup>
  <SettingItem
    title={i18n.t('settings.notifications_enabled')}
    description={i18n.t('settings.notifications_enabled_desc')}
    checked={enabled}
    onCheckedChange={(value) => void setEnabled(value)}
  />
</SettingsGroup>

<SettingsGroup>
  <SettingItem
    title={i18n.t('settings.notifications_show_preview')}
    description={i18n.t('settings.notifications_show_preview_desc')}
    checked={settings.notifications_show_preview}
    disabled={!enabled}
    onCheckedChange={(value) => void ctl.save('notifications_show_preview', value)}
  />
  <SettingItem
    title={i18n.t('settings.notifications_sound')}
    description={i18n.t('settings.notifications_sound_desc')}
    checked={settings.notifications_sound}
    disabled={!enabled}
    onCheckedChange={(value) => void ctl.save('notifications_sound', value)}
  />
</SettingsGroup>
