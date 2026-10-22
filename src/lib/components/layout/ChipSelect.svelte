<script lang="ts">
  import type { Component } from 'svelte';
  import { ripple } from '$lib/motion';
  import Select from '$lib/components/ui/Select.svelte';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';

  interface Props {
    options: { value: string | number; label: string }[];
    value: string | number;
    onchange: (value: any) => void;
    icon: Component;
    label: string;
    align?: 'left' | 'right';
  }

  let { options, value, onchange, icon: Icon, label, align = 'left' }: Props = $props();

  let current = $derived(options.find((option) => option.value === value)?.label ?? String(value));
</script>

<Select {options} {value} {onchange} {align}>
  {#snippet trigger({ toggle, open })}
    <button
      type="button"
      class="view-chip"
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-label={`${label}: ${current}`}
      use:ripple
      onclick={toggle}
    >
      <Icon />
      <span>{current}</span>
      <IconChevronDown />
    </button>
  {/snippet}
</Select>
