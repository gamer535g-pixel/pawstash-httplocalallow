<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import type { AppSettings } from '$lib/types/config';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Slider from '$lib/components/ui/Slider.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';

  let settings = $derived(ctl.settings);
</script>

<SettingsGroup>
  <SettingItem
    title={i18n.t('settings.grid_scale')}
    description={layoutState.hasKeyboard
      ? `${i18n.t('settings.grid_scale_desc')} ${i18n.t('settings.grid_scale_desc_keyboard')}`
      : i18n.t('settings.grid_scale_desc')}
    stacked
  >
    <Slider
      min={60}
      max={160}
      step={5}
      ticks
      value={settings.grid_scale}
      ariaLabel={i18n.t('settings.grid_scale')}
      formatValue={(value) => `${value}%`}
      oninput={(value) => ctl.previewValue('grid_scale', value)}
      onchange={(value) => ctl.commit('grid_scale', value)}
    />
  </SettingItem>

  <SettingChoice
    title={i18n.t('settings.grid_ratio')}
    description={i18n.t('settings.grid_ratio_desc')}
    segmented
    options={[
      { value: 'square', label: '1:1' },
      { value: 'portrait', label: '4:5' },
      { value: 'landscape', label: '3:2' },
      { value: 'widescreen', label: '16:9' }
    ] as { value: AppSettings['grid_aspect_ratio']; label: string }[]}
    value={settings.grid_aspect_ratio}
    onchange={(ratio) => void ctl.save('grid_aspect_ratio', ratio)}
  />

  <SettingChoice
    title={i18n.t('settings.card_view_mode')}
    description={i18n.t('settings.card_view_mode_desc')}
    options={[
      { value: 'detailed', label: i18n.t('settings.card_view_mode_detailed') },
      { value: 'lite', label: i18n.t('settings.card_view_mode_lite') }
    ] as { value: NonNullable<AppSettings['card_view_mode']>; label: string }[]}
    value={settings.card_view_mode || 'detailed'}
    onchange={(mode) => void ctl.save('card_view_mode', mode)}
  />

  <SettingItem
    title={i18n.t('settings.blur_previews')}
    description={i18n.t('settings.blur_previews_desc')}
    checked={!(settings.disable_blur_placeholders ?? false)}
    onCheckedChange={(value) => void ctl.save('disable_blur_placeholders', !value)}
  />
</SettingsGroup>
