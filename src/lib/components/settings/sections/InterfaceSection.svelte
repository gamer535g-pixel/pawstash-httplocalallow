<script lang="ts">
  import { themeState } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import type { FontSizeScale } from '$lib/theme/tokens';
  import type { AppSettings } from '$lib/types/config';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import ShortcutInput from '$lib/components/ui/ShortcutInput.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import IconFlagUs from '~icons/circle-flags/us';
  import IconFlagRu from '~icons/circle-flags/ru';

  let settings = $derived(ctl.settings);
  let panicEnabled = $derived(settings.panic_button_enabled ?? settings.boss_key_enabled ?? true);
  let panicShortcut = $derived(settings.panic_button_shortcut || settings.boss_key_shortcut || 'H');
</script>

<SettingsGroup label={i18n.t('settings.group_text')}>
  <SettingChoice
    title={i18n.t('settings.language')}
    options={[
      { value: 'en', label: 'English', icon: IconFlagUs },
      { value: 'ru', label: 'Русский', icon: IconFlagRu }
    ]}
    value={i18n.currentLocale}
    onchange={(locale) => i18n.setLocale(locale as typeof i18n.currentLocale)}
  />

  <SettingChoice
    title={i18n.t('settings.text_size')}
    description={i18n.t('settings.text_size_desc')}
    options={(['compact', 'standard', 'large'] as FontSizeScale[]).map((scale) => ({
      value: scale,
      label: i18n.t(`settings.text_size_${scale}`)
    }))}
    value={themeState.tokens.fontScale}
    onchange={(scale) => themeState.setFontScale(scale)}
  />

  <SettingItem title={i18n.t('settings.font_family')} description={i18n.t('settings.font_family_desc')}>
    <Input
      value={themeState.tokens.fontFamily || ''}
      placeholder={i18n.t('settings.font_family_placeholder')}
      clearable={true}
      onchange={(event) => themeState.setFontFamily((event.target as HTMLInputElement).value)}
    />
  </SettingItem>
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_display')}>
  <SettingChoice
    title={i18n.t('settings.layout_mode')}
    description={i18n.t('settings.layout_mode_desc')}
    options={(['auto', 'desktop', 'mobile'] as AppSettings['layout_mode'][]).map((mode) => ({
      value: mode,
      label: i18n.t(`settings.layout_${mode}`)
    }))}
    value={settings.layout_mode || 'auto'}
    onchange={(mode) => void ctl.save('layout_mode', mode)}
  />

  <SettingChoice
    title={i18n.t('settings.toast_position')}
    description={i18n.t('settings.toast_position_desc')}
    options={[
      { value: 'auto', label: i18n.t('settings.toast_auto') },
      { value: 'top-center', label: i18n.t('settings.toast_top_center') },
      { value: 'top-right', label: i18n.t('settings.toast_top_right') },
      { value: 'top-left', label: i18n.t('settings.toast_top_left') },
      { value: 'bottom-center', label: i18n.t('settings.toast_bottom_center') },
      { value: 'bottom-right', label: i18n.t('settings.toast_bottom_right') },
      { value: 'bottom-left', label: i18n.t('settings.toast_bottom_left') }
    ] as { value: NonNullable<AppSettings['toast_position']>; label: string }[]}
    value={settings.toast_position || 'auto'}
    onchange={(position) => void ctl.save('toast_position', position)}
  />
</SettingsGroup>

{#if layoutState.hasKeyboard}
  <SettingsGroup label={i18n.t('settings.group_privacy')}>
    <SettingItem
      title={i18n.t('settings.panic_button')}
      description={i18n.t('settings.panic_button_desc')}
      checked={panicEnabled}
      onCheckedChange={(enabled) => ctl.setPanicKey(panicShortcut, enabled)}
    />
    {#if panicEnabled}
      <SettingItem title={i18n.t('settings.panic_button_shortcut')} description={i18n.t('settings.panic_button_shortcut_desc')}>
        <ShortcutInput
          value={panicShortcut}
          defaultValue="H"
          onchange={(shortcut) => ctl.setPanicKey(shortcut || 'H', panicEnabled)}
        />
      </SettingItem>
    {/if}
  </SettingsGroup>
{/if}
