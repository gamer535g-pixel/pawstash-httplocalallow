<script lang="ts">
  import { providerState } from '$lib/state/providerState.svelte';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { formatProviderName } from '$lib/utils/media';
  import { apiOpenAppLinksSettings } from '$lib/utils/ipc';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';

  async function openAppLinks() {
    try {
      await apiOpenAppLinksSettings();
    } catch (error) {
      notify.error(i18n.t('settings.android_open_by_default_error'), error);
    }
  }
</script>

<SettingsGroup>
  {#each providerState.providers as provider (provider.id)}
    <SettingItem
      title={formatProviderName(provider.name || provider.id)}
      summary={provider.enabled
        ? provider.username || provider.api_url
        : i18n.t('settings.provider_disabled')}
      onclick={() => navigationState.openSettings(`provider-${provider.id}`)}
    />
  {/each}
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_loading')}>
  <SettingItem title={i18n.t('settings.provider_deadline')} description={i18n.t('settings.provider_deadline_desc')}>
    <NumberStepper
      value={ctl.settings.provider_deadline_secs || ctl.networkDefaults.provider_deadline_secs}
      min={1}
      max={120}
      step={1}
      onchange={(value) => void ctl.save('provider_deadline_secs', value)}
    />
  </SettingItem>
</SettingsGroup>

{#if layoutState.isAndroid}
  <SettingsGroup>
    <SettingItem
      title={i18n.t('settings.android_open_by_default')}
      description={i18n.t('settings.android_open_by_default_desc')}
      external
      onclick={() => void openAppLinks()}
    />
  </SettingsGroup>
{/if}
