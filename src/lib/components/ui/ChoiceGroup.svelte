<script lang="ts">
  import type { Component } from 'svelte';
  import { ripple } from '$lib/motion';
  import StableWeightLabel from './StableWeightLabel.svelte';

  export interface ChoiceOption<T> {
    value: T;
    label: string;
    icon?: Component;
    count?: number | string | null;
    showZero?: boolean;
    title?: string;
  }

  interface Props<T> {
    options: ChoiceOption<T>[];
    value: T;
    onchange: (value: T) => void;
    ariaLabel?: string;
    class?: string;
    align?: 'left' | 'right' | 'center';
    disabled?: boolean;
  }

  let {
    options,
    value,
    onchange,
    ariaLabel,
    class: extraClass = '',
    align = 'right',
    disabled = false
  }: Props<any> = $props();

  let buttons: HTMLButtonElement[] = [];

  let activeIndex = $derived(options.findIndex((option) => option.value === value));
  let accessibleLabel = $derived(ariaLabel || options.map((option) => option.label).join(', '));

  function select(index: number) {
    const option = options[index];
    if (!option) return;
    onchange(option.value);
    buttons[index]?.focus();
  }

  function handleKeydown(event: KeyboardEvent, index: number) {
    let next: number;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (index + 1) % options.length;
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (index - 1 + options.length) % options.length;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = options.length - 1;
    else return;
    event.preventDefault();
    select(next);
  }

  function showCount(option: ChoiceOption<unknown>): boolean {
    if (option.count === undefined || option.count === null || option.count === '') return false;
    return option.showZero || option.count !== 0;
  }
</script>

<div
  class="choice-group align-{align} {extraClass}"
  role="radiogroup"
  aria-label={accessibleLabel}
>
  {#each options as option, index}
    {@const isActive = option.value === value}
    <button
      bind:this={buttons[index]}
      type="button"
      use:ripple
      class="choice-group__option"
      class:is-active={isActive}
      role="radio"
      aria-checked={isActive}
      title={option.title}
      tabindex={isActive || (activeIndex === -1 && index === 0) ? 0 : -1}
      data-choice-value={String(option.value)}
      {disabled}
      onclick={() => select(index)}
      onkeydown={(event) => handleKeydown(event, index)}
    >
      {#if option.icon}
        <option.icon />
      {/if}
      <StableWeightLabel text={option.label} reserveWeight="500" />
      {#if showCount(option)}
        <span class="choice-group__count">{option.count}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .choice-group {
    --choice-inset: 3px;
    --choice-height: var(--control-height, 40px);

    display: inline-flex;
    align-items: stretch;
    flex-shrink: 0;
    gap: 2px;
    min-width: 0;
    height: var(--choice-height);
    padding: var(--choice-inset);
    box-sizing: border-box;
    border-radius: var(--radius-full);
    background: var(--choice-track, var(--surface-container-high));
    user-select: none;
    white-space: nowrap;
  }

  .choice-group__option {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    flex-shrink: 0;
    padding: 0 calc(var(--control-padding-x, 14px) - var(--choice-inset));
    border: 0;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--control-font-size, 14px);
    font-weight: 500;
    line-height: normal;
    overflow: hidden;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }

  @media (hover: hover) {
    .choice-group__option:not(.is-active):hover {
      background: var(--state-layer-hover);
      color: var(--text-primary);
    }
  }

  .choice-group__option:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .choice-group__option.is-active {
    background: var(--choice-active-bg);
    color: var(--choice-active-text);
  }

  .choice-group__option :global(svg) {
    width: var(--control-icon-size, 18px);
    height: var(--control-icon-size, 18px);
    flex: none;
  }

  .choice-group__count {
    font-variant-numeric: tabular-nums;
    font-weight: 400;
    opacity: 0.72;
  }

  .choice-group.align-left {
    align-self: flex-start;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) {
    display: flex;
    width: 100%;
  }

  :global([data-layout='mobile']) .choice-group:not(.align-left) .choice-group__option {
    flex: 1 1 0;
    min-width: 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .choice-group__option {
      transition: none;
    }
  }

  .choice-group__option:disabled {
    cursor: default;
    pointer-events: none;
  }
</style>
