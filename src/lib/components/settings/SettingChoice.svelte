<script lang="ts" generics="T">
  import type { Component } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { ripple } from '$lib/motion';
  import SettingItem from '$lib/components/ui/SettingItem.svelte';
  import ChoiceGroup from '$lib/components/ui/ChoiceGroup.svelte';
  import Select from '$lib/components/ui/Select.svelte';
  import BottomSheet from '$lib/components/ui/BottomSheet.svelte';
  import IconCheck from '~icons/fluent/checkmark-24-regular';

  interface Choice {
    value: T;
    label: string;
    icon?: Component;
  }

  interface Props {
    title: string;
    description?: string;
    options: Choice[];
    value: T;
    onchange: (value: T) => void;
    segmented?: boolean;
    disabled?: boolean;
  }

  let { title, description, options, value, onchange, segmented, disabled = false }: Props = $props();

  let sheet = $state<ReturnType<typeof BottomSheet> | null>(null);
  let sheetOpen = $state(false);

  let isSegmented = $derived(segmented ?? options.length <= 3);
  let selected = $derived(options.find((option) => option.value === value));

  function pick(next: T) {
    onchange(next);
    sheet?.close();
  }
</script>

{#if isSegmented}
  <SettingItem {title} {description} {disabled}>
    <ChoiceGroup {options} {value} {disabled} onchange={(next) => onchange(next as T)} />
  </SettingItem>
{:else if !layoutState.isMobile}
  <SettingItem {title} {description} {disabled}>
    <Select
      options={options.map((option) => ({ value: option.value as string | number, label: option.label }))}
      value={value as string | number}
      onchange={(next) => onchange(next as T)}
      ariaLabel={title}
      align="right"
      {disabled}
    />
  </SettingItem>
{:else}
  <SettingItem {title} summary={selected?.label} {disabled} onclick={() => (sheetOpen = true)} />

  <BottomSheet bind:this={sheet} isOpen={sheetOpen} {title} {description} closeLabel={i18n.t('common.close')} onclose={() => (sheetOpen = false)}>
    <div class="choice-sheet__options" role="radiogroup" aria-label={title}>
      {#each options as option (option.value)}
        {@const isSelected = option.value === value}
        <button
          type="button"
          class="floating-item"
          role="radio"
          aria-checked={isSelected}
          use:ripple
          onclick={() => pick(option.value)}
        >
          <span class="floating-item-label choice-sheet__label">{option.label}</span>
          {#if isSelected}<IconCheck />{/if}
        </button>
      {/each}
    </div>
  </BottomSheet>
{/if}

<style>
  .choice-sheet__options {
    display: flex;
    flex-direction: column;
  }

  .choice-sheet__label {
    font-size: var(--type-body-lg-size);
    line-height: var(--type-body-lg-line);
  }
</style>
