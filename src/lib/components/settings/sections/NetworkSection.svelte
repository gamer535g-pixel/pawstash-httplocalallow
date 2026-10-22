<script lang="ts">
  import { i18n } from '$lib/i18n';
  import type { AppSettings } from '$lib/types/config';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import LinksSection from './LinksSection.svelte';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';
  import IconGlobe from '~icons/fluent/globe-24-regular';
  import IconKey from '~icons/fluent/key-24-regular';

  let settings = $derived(ctl.settings);
  let defaults = $derived(ctl.networkDefaults);
</script>

<SettingsGroup label={i18n.t('settings.group_proxy')}>
  <SettingChoice
    title={i18n.t('settings.proxy_mode')}
    description={i18n.t('settings.proxy_mode_desc')}
    options={[
      { value: 'none', label: i18n.t('settings.proxy_none'), icon: IconDismiss },
      { value: 'system', label: i18n.t('settings.proxy_system'), icon: IconGlobe },
      { value: 'custom', label: i18n.t('settings.proxy_custom'), icon: IconKey }
    ] as { value: AppSettings['proxy_mode']; label: string; icon: typeof IconKey }[]}
    value={settings.proxy_mode}
    onchange={(mode) => void ctl.save('proxy_mode', mode)}
  />

  {#if ctl.webviewProxyNotice}
    <SettingItem title={i18n.t('settings.webview_proxy_title')} description={ctl.webviewProxyNotice} />
  {/if}

  {#if settings.proxy_mode === 'custom'}
    <SettingItem title={i18n.t('settings.proxy_url')} description={i18n.t('settings.proxy_url_desc')}>
      <Input
        clearable={true}
        placeholder="http://127.0.0.1:8080"
        bind:value={ctl.settings.proxy_url}
        onblur={() => void ctl.save('proxy_url', ctl.settings.proxy_url)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.proxy_username')} description={i18n.t('settings.proxy_username_desc')}>
      <Input
        clearable={true}
        placeholder={i18n.t('settings.optional')}
        bind:value={ctl.settings.proxy_username}
        onblur={() => void ctl.save('proxy_username', ctl.settings.proxy_username)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.proxy_password')} description={i18n.t('settings.proxy_password_desc')}>
      <Input
        type="password"
        placeholder={i18n.t('settings.optional')}
        bind:value={ctl.settings.proxy_password}
        onblur={() => void ctl.save('proxy_password', ctl.settings.proxy_password)}
      />
    </SettingItem>
    <SettingItem
      title={i18n.t('settings.proxy_bypass_local')}
      description={i18n.t('settings.proxy_bypass_local_desc')}
      checked={settings.proxy_bypass_local}
      onCheckedChange={(value) => void ctl.save('proxy_bypass_local', value)}
    />
  {/if}
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_timeouts')}>
  <SettingItem title={i18n.t('settings.network_timeout')} description={i18n.t('settings.network_timeout_desc')}>
    <NumberStepper
      value={settings.network_timeout_secs || defaults.request_timeout_secs}
      min={5}
      max={600}
      step={5}
      onchange={(value) => void ctl.save('network_timeout_secs', value)}
    />
  </SettingItem>
  <SettingItem title={i18n.t('settings.network_connect_timeout')} description={i18n.t('settings.network_connect_timeout_desc')}>
    <NumberStepper
      value={settings.network_connect_timeout_secs || defaults.connect_timeout_secs}
      min={1}
      max={120}
      step={1}
      onchange={(value) => void ctl.save('network_connect_timeout_secs', value)}
    />
  </SettingItem>
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_user_agent')}>
  <SettingItem title={i18n.t('settings.network_api_user_agent')} description={i18n.t('settings.network_api_user_agent_desc')}>
    <Input
      clearable={true}
      placeholder={defaults.api_user_agent}
      bind:value={ctl.settings.network_api_user_agent}
      onchange={() => void ctl.save('network_api_user_agent', ctl.settings.network_api_user_agent)}
    />
  </SettingItem>
  <SettingItem title={i18n.t('settings.network_browser_user_agent')} description={i18n.t('settings.network_browser_user_agent_desc')}>
    <Input
      clearable={true}
      placeholder={defaults.browser_user_agent}
      bind:value={ctl.settings.network_browser_user_agent}
      onchange={() => void ctl.save('network_browser_user_agent', ctl.settings.network_browser_user_agent)}
    />
  </SettingItem>
</SettingsGroup>

<LinksSection />
