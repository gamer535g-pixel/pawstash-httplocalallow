<script lang="ts" generics="T extends string | number">
  import { i18n } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';
  import IconArrowSort from '~icons/fluent/arrow-sort-24-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';

  interface Props {
    options: { value: T; label: string }[];
    value: T;
    onchange: (value: T) => void;
  }

  let { options, value, onchange }: Props = $props();
</script>

<div class="filter-section-title">
  <div class="filter-section-icon"><IconArrowSort class="w-3.5 h-3.5" /></div>
  <span>{i18n.t('favorites.sort_by')}</span>
</div>
<div class="service-options" role="radiogroup" aria-label={i18n.t('favorites.sort_by')}>
  {#each options as option (option.value)}
    {@const selected = option.value === value}
    <Button
      variant="ghost"
      size="sm"
      role="radio"
      aria-checked={selected}
      class="filter-chip {selected ? 'state-include' : ''}"
      onclick={() => onchange(option.value)}
    >
      {#if selected}<IconCheck class="w-4 h-4" />{/if}
      <span>{option.label}</span>
    </Button>
  {/each}
</div>
