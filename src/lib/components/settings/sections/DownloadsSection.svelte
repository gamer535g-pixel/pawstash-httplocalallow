<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { copyText } from '$lib/utils/clipboard';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import NumberStepper from '$lib/components/ui/NumberStepper.svelte';
  import TemplateInput, { type TemplateTag } from '$lib/components/ui/TemplateInput.svelte';
  import DownloadsStatsBar from '$lib/components/downloads/DownloadsStatsBar.svelte';
  import SettingsGroup from '../SettingsGroup.svelte';
  import SettingChoice from '../SettingChoice.svelte';
  import { settingsController as ctl } from '../settingsController.svelte';
  import { downloadPathPreview } from '$lib/utils/templatePreview';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';

  let settings = $derived(ctl.settings);
  let preview = $derived(downloadPathPreview(settings));
  let copied = $state(false);

  let creatorTags = $derived<TemplateTag[]>([
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' },
    { tag: '{creator_id}', label: i18n.t('settings.tag_creator_id'), example: '12345' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' }
  ]);

  let postTags = $derived<TemplateTag[]>([
    { tag: '{post_title}', label: i18n.t('settings.tag_post_title'), example: 'PostTitle' },
    { tag: '{post_id}', label: i18n.t('settings.tag_post_id'), example: '67890' },
    { tag: '{date}', label: i18n.t('settings.tag_date'), example: '2024-08-20' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' },
    { tag: '{day}', label: i18n.t('settings.tag_day'), example: '20' },
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' }
  ]);

  let fileTags = $derived<TemplateTag[]>([
    { tag: '{post_title}', label: i18n.t('settings.tag_post_title'), example: 'PostTitle' },
    { tag: '{filename}', label: i18n.t('settings.tag_filename'), example: 'OriginalFilename.png' },
    { tag: '{name}', label: i18n.t('settings.tag_name'), example: 'OriginalFilename' },
    { tag: '{ext}', label: i18n.t('settings.tag_ext'), example: 'png' },
    { tag: '{index}', label: i18n.t('settings.tag_index'), example: '1' },
    { tag: '{date}', label: i18n.t('settings.tag_date'), example: '2024-08-20' },
    { tag: '{year}', label: i18n.t('settings.tag_year'), example: '2024' },
    { tag: '{month}', label: i18n.t('settings.tag_month'), example: '08' },
    { tag: '{day}', label: i18n.t('settings.tag_day'), example: '20' },
    { tag: '{creator}', label: i18n.t('settings.tag_creator'), example: 'AuthorName' },
    { tag: '{service}', label: i18n.t('settings.tag_service'), example: 'Platform' },
    { tag: '{post_id}', label: i18n.t('settings.tag_post_id'), example: '67890' }
  ]);

  async function chooseFolder() {
    try {
      const selected = layoutState.isMobileDevice
        ? await invoke<string | null>('pick_folder')
        : await open({ directory: true, multiple: false, title: i18n.t('settings.download_dir') });
      if (typeof selected !== 'string' || !selected) return;
      if (await ctl.save('download_dir', selected)) notify.success(i18n.t('settings.download_path_updated'), selected);
    } catch (error) {
      notify.error(i18n.t('settings.download_dir_failed'), error);
    }
  }

  async function copyPreview() {
    if (!(await copyText(preview, i18n.t('settings.copied_to_clipboard')))) return;
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<SettingsGroup>
  <SettingItem title={i18n.t('settings.downloads_storage')} description={i18n.t('settings.downloads_storage_desc')} stacked>
    <DownloadsStatsBar downloads={downloadState.downloads} />
  </SettingItem>

  <SettingItem title={i18n.t('settings.download_dir')} description={i18n.t('settings.download_dir_desc')}>
    <Input
      placeholder="Downloads/Pawstash"
      bind:value={ctl.settings.download_dir}
      onblur={() => void ctl.save('download_dir', ctl.settings.download_dir)}
      onBrowse={chooseFolder}
    />
  </SettingItem>
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_naming')}>
  <SettingItem
    title={i18n.t('settings.download_group_by_creator')}
    description={i18n.t('settings.download_group_by_creator_desc')}
    checked={settings.download_group_by_creator}
    onCheckedChange={(value) => void ctl.save('download_group_by_creator', value)}
  />

  {#if settings.download_group_by_creator}
    <SettingItem title={i18n.t('settings.download_creator_template')} description={i18n.t('settings.download_creator_template_desc')}>
      <TemplateInput
        placeholder="&#123;creator&#125;"
        bind:value={ctl.settings.download_creator_folder_template}
        tags={creatorTags}
        previewType="creator"
        onchange={(value) => void ctl.save('download_creator_folder_template', value)}
      />
    </SettingItem>
  {/if}

  <SettingItem
    title={i18n.t('settings.download_group_by_post')}
    description={i18n.t('settings.download_group_by_post_desc')}
    checked={settings.download_group_by_post}
    onCheckedChange={(value) => void ctl.save('download_group_by_post', value)}
  />

  {#if settings.download_group_by_post}
    <SettingItem title={i18n.t('settings.download_post_template')} description={i18n.t('settings.download_post_template_desc')}>
      <TemplateInput
        placeholder="&#123;post_title&#125;"
        bind:value={ctl.settings.download_post_folder_template}
        tags={postTags}
        previewType="post"
        onchange={(value) => void ctl.save('download_post_folder_template', value)}
      />
    </SettingItem>
  {/if}

  <SettingItem title={i18n.t('settings.download_filename_template')} description={i18n.t('settings.download_filename_template_desc')}>
    <TemplateInput
      placeholder="&#123;post_title&#125; - &#123;filename&#125;"
      bind:value={ctl.settings.download_filename_template}
      tags={fileTags}
      previewType="file"
      onchange={(value) => void ctl.save('download_filename_template', value)}
    />
  </SettingItem>

  <SettingItem title={i18n.t('settings.template_preview')} description={i18n.t('settings.download_preview_desc')} stacked>
    <Input
      value={preview}
      readonly={true}
      class="font-mono"
      actionIcon={copied ? IconCheck : IconCopy}
      actionTooltip={copied ? i18n.t('common.copied') : i18n.t('common.copy')}
      onAction={copyPreview}
    />
  </SettingItem>
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.group_post_details')}>
  <SettingItem
    title={i18n.t('settings.download_save_metadata')}
    description={i18n.t('settings.download_save_metadata_desc')}
    checked={settings.download_save_metadata ?? false}
    onCheckedChange={(value) => void ctl.save('download_save_metadata', value)}
  />

  {#if settings.download_save_metadata}
    <SettingChoice
      title={i18n.t('settings.download_metadata_format')}
      description={i18n.t('settings.download_metadata_format_desc')}
      options={[
        { value: 'txt', label: '.txt' },
        { value: 'json', label: '.json' },
        { value: 'both', label: '.txt + .json' }
      ] as { value: NonNullable<typeof settings.download_metadata_format>; label: string }[]}
      value={settings.download_metadata_format || 'txt'}
      onchange={(format) => void ctl.save('download_metadata_format', format)}
    />
  {/if}
</SettingsGroup>

<SettingsGroup label={i18n.t('settings.download_section')}>
  <SettingItem title={i18n.t('settings.download_max_concurrent')} description={i18n.t('settings.download_max_concurrent_desc')}>
    <NumberStepper
      min={1}
      max={10}
      value={settings.download_max_concurrent ?? 3}
      ariaLabel={i18n.t('settings.download_max_concurrent')}
      onchange={(value) => void ctl.save('download_max_concurrent', value)}
    />
  </SettingItem>

  <SettingItem
    title={i18n.t('settings.download_auto_retry')}
    description={i18n.t('settings.download_auto_retry_desc')}
    checked={settings.download_auto_retry ?? true}
    onCheckedChange={(value) => void ctl.save('download_auto_retry', value)}
  />

  {#if settings.download_auto_retry ?? true}
    <SettingItem title={i18n.t('settings.download_auto_retry_max')} description={i18n.t('settings.download_auto_retry_max_desc')}>
      <NumberStepper
        min={1}
        max={10}
        value={settings.download_auto_retry_max ?? 3}
        ariaLabel={i18n.t('settings.download_auto_retry_max')}
        onchange={(value) => void ctl.save('download_auto_retry_max', value)}
      />
    </SettingItem>
  {/if}

  <!-- aria2c is a desktop binary; phones always use the built-in downloader. -->
  {#if !layoutState.isMobileDevice}
    <SettingItem
      title={i18n.t('settings.aria2c_engine')}
      description={i18n.t('settings.aria2c_engine_desc')}
      checked={settings.use_aria2c}
      onCheckedChange={(value) => void ctl.save('use_aria2c', value)}
    />

    {#if settings.use_aria2c}
      <SettingItem title={i18n.t('settings.parallel_connections')} description={i18n.t('settings.parallel_connections_desc')}>
        <NumberStepper
          min={1}
          max={32}
          value={settings.aria2_connections}
          ariaLabel={i18n.t('settings.parallel_connections')}
          onchange={(value) => void ctl.save('aria2_connections', value)}
        />
      </SettingItem>
    {/if}
  {/if}
</SettingsGroup>
