<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { ripple, tooltip } from '$lib/motion';

  interface Props extends HTMLButtonAttributes {
    type?: 'button' | 'submit' | 'reset';
    variant?: 'primary' | 'accent' | 'tonal' | 'ghost' | 'danger';
    size?: 'sm' | 'base' | 'md' | 'lg';
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
    class?: string;
    ref?: HTMLButtonElement;
    title?: string;
    tooltip?: string;
  }

  let {
    type = 'button',
    variant = 'primary',
    size,
    disabled = false,
    onclick,
    children,
    class: extraClass = '',
    ref = $bindable(),
    title,
    tooltip: tooltipProp,
    ...restProps
  }: Props = $props();

  let effectiveTooltip = $derived(tooltipProp ?? title);
</script>

<button
  bind:this={ref}
  {type}
  {disabled}
  {onclick}
  use:ripple
  use:tooltip={effectiveTooltip}
  aria-label={restProps['aria-label'] || effectiveTooltip}
  class="btn btn-{variant} {size ? `btn-${size}` : ''} {extraClass}"
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-sans);
    font-weight: 500;
    line-height: normal;
    cursor: pointer;
    border-radius: var(--radius-full);
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard),
                opacity var(--duration-fast) var(--ease-standard);
    border: 0;
    outline: none;
    box-sizing: border-box;
    white-space: nowrap;
    flex-shrink: 0;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-x, 20px) * var(--ui-scale, 1));
    font-size: calc(var(--control-font-size, 14px) * var(--ui-scale, 1));
    gap: calc(var(--control-gap, 8px) * var(--ui-scale, 1));
  }

  .btn :global(svg) {
    width: calc(var(--control-icon-size, 20px) * var(--ui-scale, 1));
    height: calc(var(--control-icon-size, 20px) * var(--ui-scale, 1));
    flex-shrink: 0;
  }

  @media (hover: hover) {
    .btn:hover {
      background-image: linear-gradient(var(--state-layer-hover), var(--state-layer-hover));
    }
  }

  .btn:active {
    background-image: linear-gradient(var(--state-layer-pressed), var(--state-layer-pressed));
  }

  .btn:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: 2px;
  }

  .btn:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
    pointer-events: none;
  }

  .btn-primary {
    background-color: var(--surface-container-highest);
    color: var(--text-primary);
  }

  .btn-accent {
    background-color: var(--accent-primary);
    color: var(--accent-on-primary);
  }

  .btn-tonal {
    background-color: var(--accent-container);
    color: var(--accent-on-container);
  }

  .btn-ghost {
    background-color: transparent;
    color: var(--text-secondary);
  }

  @media (hover: hover) {
    .btn-ghost:hover {
      color: var(--text-primary);
    }
  }

  .btn-ghost[aria-pressed='true'],
  .btn-ghost.is-on {
    color: var(--accent-primary);
  }

  .btn-ghost[aria-expanded='true'] {
    background-color: var(--state-layer-pressed);
  }

  .btn-danger {
    background-color: color-mix(in srgb, var(--status-error) 16%, transparent);
    color: var(--status-error);
  }

  .btn-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12px);
    --control-icon-size: var(--control-icon-sm, 16px);
    --control-padding-x: var(--control-padding-sm, 14px);
    --control-gap: 6px;
  }

  .btn-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
    --control-padding-x: var(--control-padding-base, 16px);
    --control-gap: 7px;
  }

  .btn-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 20px);
    --control-gap: 8px;
  }

  .btn-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 16px);
    --control-icon-size: var(--control-icon-lg, 24px);
    --control-padding-x: var(--control-padding-lg, 24px);
    --control-gap: 10px;
  }

  .btn.btn-icon,
  :global(.btn.btn-icon) {
    width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    min-width: calc(var(--control-height, 46px) * var(--ui-scale, 1)) !important;
    padding: 0 !important;
    border-radius: var(--radius-full) !important;
    flex-shrink: 0;
    display: inline-flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .btn.btn-icon :global(svg),
  :global(.btn.btn-icon svg) {
    width: calc(var(--control-icon-size, 20px) * var(--ui-scale, 1)) !important;
    height: calc(var(--control-icon-size, 20px) * var(--ui-scale, 1)) !important;
    flex-shrink: 0 !important;
  }
</style>
