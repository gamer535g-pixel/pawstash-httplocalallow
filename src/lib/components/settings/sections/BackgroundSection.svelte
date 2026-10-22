<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    backgroundState,
    defaultAcrylicOpacity,
    supportedBackgroundTypes,
    type BackgroundType,
    type CustomBackgroundKind
  } from '$lib/theme/backgroundState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Slider from '$lib/components/ui/Slider.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import ColorPicker from '$lib/components/ui/ColorPicker.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { backgroundLabel } from '../sections';
  import IconDelete from '~icons/fluent/delete-24-regular';

  const types = supportedBackgroundTypes();
  let bg = $derived(backgroundState.settings);
  let hasMedia = $derived(
    (bg.customKind === 'image' && Boolean(bg.imageUrl)) || (bg.customKind === 'video' && Boolean(bg.videoUrl))
  );
  let confirmClearMedia = $state(false);

  async function chooseFile(kind: 'image' | 'video') {
    try {
      const selected = layoutState.isMobileDevice
        ? await invoke<string | null>('pick_file', { kind })
        : await open({
            multiple: false,
            title: i18n.t(kind === 'image' ? 'settings.choose_background_image' : 'settings.choose_background_video'),
            filters: [
              {
                name: kind === 'image' ? 'Images' : 'Videos',
                extensions: kind === 'image' ? ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif'] : ['mp4', 'webm', 'm4v', 'mov']
              }
            ]
          });
      if (typeof selected !== 'string' || !selected) return;
      const stored = await invoke<string>('store_custom_background', { sourcePath: selected, kind });
      if (kind === 'image') backgroundState.setImageUrl(stored);
      else backgroundState.setVideoUrl(stored);
      notify.success(
        i18n.t(kind === 'image' ? 'settings.background_saved' : 'settings.background_video_saved'),
        stored.split(/[/\\]/).pop() || stored
      );
    } catch (error) {
      notify.error(i18n.t('settings.background_file_failed'), error);
    }
  }
</script>

<SettingsGroup label={i18n.t('settings.group_background')}>
  <SettingChoice
    title={i18n.t('settings.bg_type')}
    description={i18n.t('settings.bg_type_desc')}
    options={types.map((type) => ({ value: type, label: backgroundLabel(type) }))}
    value={bg.type}
    onchange={(type: BackgroundType) => backgroundState.setType(type)}
  />

  <SettingItem
    title={i18n.t('settings.background_tint')}
    description={i18n.t('settings.background_tint_desc')}
    checked={bg.acrylicTint}
    onCheckedChange={(value) => backgroundState.setAcrylicTint(value)}
  />

  {#if bg.acrylicTint}
    <SettingItem
      title={i18n.t('settings.background_tint_opacity')}
      description={i18n.t('settings.background_tint_opacity_desc')}
      stacked
    >
      <Slider
        min={0.1}
        max={1}
        step={0.05}
        value={bg.acrylicOpacity ?? defaultAcrylicOpacity(themeState.isDark)}
        ariaLabel={i18n.t('settings.background_tint_opacity')}
        formatValue={(value) => `${Math.round(value * 100)}%`}
        oninput={(value) => backgroundState.setAcrylicOpacity(value)}
      />
    </SettingItem>
  {/if}

  {#if bg.type === 'custom'}
    <SettingChoice
      title={i18n.t('settings.background_source')}
      description={i18n.t('settings.background_source_desc')}
      options={[
        { value: 'color', label: i18n.t('settings.background_source_color') },
        { value: 'palette', label: i18n.t('settings.background_source_palette') },
        { value: 'image', label: i18n.t('settings.background_source_image') },
        { value: 'video', label: i18n.t('settings.background_source_video') }
      ]}
      value={bg.customKind}
      onchange={(kind: CustomBackgroundKind) => backgroundState.setCustomKind(kind)}
    />

    {#if bg.customKind === 'color'}
      <SettingItem title={i18n.t('settings.background_primary')} description={i18n.t('settings.background_primary_desc')}>
        <ColorPicker
          value={bg.solidColor}
          label={i18n.t('settings.background_primary')}
          onchange={(color) => backgroundState.setSolidColor(color)}
        />
      </SettingItem>
      <SettingItem title={i18n.t('settings.background_secondary')} description={i18n.t('settings.background_secondary_desc')}>
        <ColorPicker
          value={bg.gradientSecondary}
          label={i18n.t('settings.background_secondary')}
          onchange={(color) => backgroundState.setGradientSecondary(color)}
        />
      </SettingItem>
    {:else if bg.customKind === 'image'}
      <SettingItem title={i18n.t('settings.background_image')} description={i18n.t('settings.background_image_desc')}>
        <Input
          placeholder={i18n.t('settings.background_image_placeholder')}
          value={bg.imageUrl}
          readonly={true}
          onBrowse={() => void chooseFile('image')}
        />
      </SettingItem>
    {:else if bg.customKind === 'video'}
      <SettingItem title={i18n.t('settings.background_video')} description={i18n.t('settings.background_video_desc')}>
        <Input
          placeholder={i18n.t('settings.background_video_placeholder')}
          value={bg.videoUrl}
          readonly={true}
          onBrowse={() => void chooseFile('video')}
        />
      </SettingItem>
    {/if}

    {#if hasMedia}
      <SettingItem
        title={i18n.t('settings.background_media')}
        description={i18n.t('settings.background_media_desc')}
        action
        danger
        onclick={() => (confirmClearMedia = true)}
      />
    {/if}
  {/if}
</SettingsGroup>

{#if bg.type === 'custom'}
  <SettingsGroup label={i18n.t('settings.group_background_effects')}>
    <SettingItem title={i18n.t('settings.background_blur')} description={i18n.t('settings.background_blur_desc')} stacked>
      <Slider
        min={0}
        max={40}
        value={bg.blurPx}
        ariaLabel={i18n.t('settings.background_blur')}
        formatValue={(value) => `${value}px`}
        oninput={(value) => backgroundState.setBlur(value)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.background_opacity')} description={i18n.t('settings.background_opacity_desc')} stacked>
      <Slider
        min={0.1}
        max={1}
        step={0.05}
        value={bg.opacity}
        ariaLabel={i18n.t('settings.background_opacity')}
        formatValue={(value) => `${Math.round(value * 100)}%`}
        oninput={(value) => backgroundState.setOpacity(value)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.background_brightness')} description={i18n.t('settings.background_brightness_desc')} stacked>
      <Slider
        min={0.2}
        max={1.5}
        step={0.05}
        value={bg.brightness}
        ariaLabel={i18n.t('settings.background_brightness')}
        formatValue={(value) => `${Math.round(value * 100)}%`}
        oninput={(value) => backgroundState.setBrightness(value)}
      />
    </SettingItem>
    <SettingItem title={i18n.t('settings.background_saturation')} description={i18n.t('settings.background_saturation_desc')} stacked>
      <Slider
        min={0}
        max={2}
        step={0.05}
        value={bg.saturation}
        ariaLabel={i18n.t('settings.background_saturation')}
        formatValue={(value) => `${Math.round(value * 100)}%`}
        oninput={(value) => backgroundState.setSaturation(value)}
      />
    </SettingItem>
  </SettingsGroup>
{/if}

<ConfirmDialog
  isOpen={confirmClearMedia}
  title={i18n.t('settings.background_media_clear')}
  description={i18n.t('settings.background_media_clear_confirm')}
  confirmLabel={i18n.t('common.delete')}
  confirmVariant="danger"
  confirmIcon={IconDelete}
  onconfirm={() => {
    confirmClearMedia = false;
    backgroundState.clearCustomMedia(bg.customKind as 'image' | 'video');
  }}
  onclose={() => (confirmClearMedia = false)}
/>
