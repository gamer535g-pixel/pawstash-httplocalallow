<script lang="ts">
  import { onMount } from 'svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { formatProviderName } from '$lib/utils/media';
  import { logger } from '$lib/utils/logger';
  import {
    apiGetProviderAuthSchema,
    apiGetProviderNetworkDefaults,
    apiLogoutProviderSession,
    apiSyncProviderFavorites,
    type ProviderNetworkDefaults
  } from '$lib/utils/ipc';
  import type { ProviderAuthSchema, ProviderConfig } from '$lib/types/provider';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import ProviderAuthModal from '$lib/components/providers/ProviderAuthModal.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';

  interface Props {
    providerId: string;
  }

  let { providerId }: Props = $props();

  let provider = $derived(providerState.providers.find((p) => p.id === providerId) ?? null);
  let name = $derived(formatProviderName(provider?.name || providerId));
  let schema = $state<ProviderAuthSchema | null>(null);
  let networkDefaults = $state<ProviderNetworkDefaults | null>(null);
  let authOpen = $state(false);
  let logoutOpen = $state(false);
  let loggingOut = $state(false);
  let syncing = $state(false);
  let isAuthenticated = $derived(Boolean(provider?.session_cookie?.trim()));

  onMount(async () => {
    const [authSchema, defaults] = await Promise.allSettled([
      apiGetProviderAuthSchema(providerId),
      apiGetProviderNetworkDefaults()
    ]);
    if (authSchema.status === 'fulfilled') schema = authSchema.value;
    else logger.warn('Failed to load provider auth schema', { providerId, error: authSchema.reason });
    if (defaults.status === 'fulfilled') networkDefaults = defaults.value[providerId] ?? null;
    else logger.warn('Failed to load provider network defaults', defaults.reason);
  });

  async function update(patch: Partial<ProviderConfig>, reload = false) {
    if (!provider) return;
    try {
      await providerState.updateProvider({ ...provider, ...patch });
      if (reload) ctl.reloadContent();
    } catch (error) {
      notify.error(i18n.t('settings.save_failed'), error);
    }
  }

  function updateApiUrl(raw: string) {
    const url = raw.trim();
    if (url && url !== provider?.api_url) void update({ api_url: url }, true);
  }

  async function syncFavorites() {
    syncing = true;
    try {
      const result = await apiSyncProviderFavorites(providerId, 'both');
      notify.success(i18n.t('settings.auth_sync_success', { pulled: result.pulled_count, pushed: result.pushed_count }));
      ctl.reloadContent();
    } catch (error) {
      notify.error(i18n.t('settings.auth_sync_failed'), error);
    } finally {
      syncing = false;
    }
  }

  async function logout(removeFavorites: boolean) {
    loggingOut = true;
    try {
      await apiLogoutProviderSession(providerId, removeFavorites);
      await providerState.loadProviders();
      notify.success(i18n.t('settings.auth_logout_success', { provider: name }));
      ctl.reloadContent();
      logoutOpen = false;
    } catch (error) {
      notify.error(i18n.t('settings.provider_logout_failed'), error);
    } finally {
      loggingOut = false;
    }
  }

  async function setHideAi(value: boolean) {
    if (await ctl.save('pawchive_hide_ai', value)) ctl.reloadContent();
  }
</script>

{#if provider}
  <SettingsGroup>
    <SettingItem
      title={i18n.t('settings.provider_active')}
      description={i18n.t('settings.provider_enable_desc', { provider: name })}
      checked={provider.enabled}
      onCheckedChange={(enabled) => void update({ enabled }, true)}
    />
    <SettingItem title={i18n.t('settings.provider_api_url')} description={i18n.t('settings.provider_api_url_desc', { provider: name })}>
      <Input
        clearable={true}
        value={provider.api_url}
        placeholder="https://..."
        onblur={(event) => updateApiUrl((event.target as HTMLInputElement).value)}
      />
    </SettingItem>
    {#if provider.id === 'pawchive'}
      <SettingItem
        title={i18n.t('settings.pawchive_hide_ai')}
        description={i18n.t('settings.pawchive_hide_ai_desc')}
        checked={ctl.settings.pawchive_hide_ai ?? false}
        onCheckedChange={(value) => void setHideAi(value)}
      />
    {/if}
  </SettingsGroup>

  {#if schema?.supports_auth}
    <SettingsGroup label={i18n.t('settings.provider_auth')}>
      <SettingItem
        title={isAuthenticated ? provider.username || i18n.t('settings.provider_auth') : i18n.t('settings.provider_auth')}
        description={isAuthenticated
          ? provider.username
            ? i18n.t('settings.provider_logged_in_as', { username: provider.username })
            : i18n.t('settings.provider_signed_in')
          : i18n.t('settings.provider_guest')}
      >
        <div class="settings-action-group">
          {#if isAuthenticated}
            {#if schema.supports_remote_favorites}
              <Button variant="tonal" disabled={syncing} onclick={() => void syncFavorites()}>
                {#if syncing}<IconLoading />{/if}
                {i18n.t('settings.provider_sync_favorites')}
              </Button>
            {/if}
            <Button variant="danger" onclick={() => (logoutOpen = true)}>
              {i18n.t('settings.provider_logout')}
            </Button>
          {:else}
            <Button variant="tonal" onclick={() => (authOpen = true)}>
              {i18n.t('settings.provider_login')}
            </Button>
          {/if}
        </div>
      </SettingItem>
    </SettingsGroup>
  {/if}

  <SettingsGroup label={i18n.t('settings.network_advanced')}>
    <SettingItem
      title={i18n.t('settings.provider_advanced_network')}
      description={i18n.t('settings.provider_advanced_network_desc')}
      checked={provider.advanced_network ?? false}
      onCheckedChange={(value) => void update({ advanced_network: value })}
    />
    {#if provider.advanced_network}
      <SettingItem title={i18n.t('settings.provider_user_agent')} description={i18n.t('settings.provider_user_agent_desc')}>
        <Input
          clearable={true}
          value={provider.user_agent ?? ''}
          placeholder={ctl.settings.network_api_user_agent || ctl.networkDefaults.api_user_agent}
          onchange={(event) => void update({ user_agent: (event.target as HTMLInputElement).value })}
        />
      </SettingItem>
      <SettingItem title={i18n.t('settings.provider_timeout')} description={i18n.t('settings.provider_timeout_desc')}>
        <NumberStepper
          value={provider.timeout_secs || ctl.settings.network_timeout_secs || ctl.networkDefaults.request_timeout_secs}
          min={5}
          max={600}
          step={5}
          onchange={(value) => void update({ timeout_secs: value })}
        />
      </SettingItem>
      <SettingItem title={i18n.t('settings.provider_proxy_url')} description={i18n.t('settings.provider_proxy_url_desc')}>
        <Input
          clearable={true}
          value={provider.proxy_url ?? ''}
          placeholder={i18n.t('settings.inherits_global_proxy')}
          onchange={(event) => void update({ proxy_url: (event.target as HTMLInputElement).value })}
        />
      </SettingItem>
      <SettingItem title={i18n.t('settings.provider_max_retries')} description={i18n.t('settings.provider_max_retries_desc')}>
        <NumberStepper
          value={provider.max_retries || networkDefaults?.max_retries || 3}
          min={1}
          max={10}
          step={1}
          onchange={(value) => void update({ max_retries: value })}
        />
      </SettingItem>
      <SettingItem title={i18n.t('settings.provider_min_interval')} description={i18n.t('settings.provider_min_interval_desc')}>
        <NumberStepper
          value={provider.min_interval_ms || networkDefaults?.min_interval_ms || 100}
          min={10}
          max={5000}
          step={50}
          onchange={(value) => void update({ min_interval_ms: value })}
        />
      </SettingItem>
    {/if}
  </SettingsGroup>

  {#if authOpen}
    <ProviderAuthModal
      isOpen={authOpen}
      {provider}
      onclose={() => (authOpen = false)}
      onsuccess={async () => {
        await providerState.loadProviders();
        ctl.reloadContent();
      }}
    />
  {/if}

  <ConfirmDialog
    isOpen={logoutOpen}
    title={i18n.t('settings.auth_logout_confirm_title', { provider: name })}
    description={i18n.t('settings.auth_logout_confirm_desc')}
    onclose={() => (logoutOpen = false)}
  >
    {#snippet actions()}
      <div class="dialog-actions">
        <Button variant="ghost" disabled={loggingOut} onclick={() => void logout(false)}>
          {i18n.t('settings.auth_logout_keep')}
        </Button>
        <Button variant="ghost" data-tone="danger" disabled={loggingOut} onclick={() => void logout(true)}>
          {i18n.t('settings.auth_logout_remove')}
        </Button>
      </div>
    {/snippet}
  </ConfirmDialog>
{:else}
  <p class="provider-missing">{i18n.t('settings.provider_missing')}</p>
{/if}

<style>
  .provider-missing {
    margin: 0;
    padding-inline: 16px;
    color: var(--text-secondary);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
  }
</style>
