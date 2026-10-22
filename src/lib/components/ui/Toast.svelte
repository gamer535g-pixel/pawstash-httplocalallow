<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { ripple } from '$lib/motion';
  import { i18n } from '$lib/i18n';
  import type { ToastVariant } from '$lib/utils/toast';
  import IconSuccess from '~icons/fluent/checkmark-24-filled';
  import IconError from '~icons/fluent/error-circle-24-filled';
  import IconWarning from '~icons/fluent/warning-24-filled';
  import IconInfo from '~icons/fluent/info-24-filled';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import IconClose from '~icons/fluent/dismiss-20-regular';
  import IconFavorited from '~icons/fluent/heart-24-filled';
  import IconUnfavorited from '~icons/fluent/heart-broken-24-regular';
  import IconRemoved from '~icons/fluent/folder-dismiss-24-regular';
  import IconDeleted from '~icons/fluent/delete-24-filled';
  import IconCopied from '~icons/fluent/copy-24-filled';
  import IconDownload from '~icons/fluent/arrow-download-24-filled';
  import IconCleared from '~icons/fluent/broom-24-regular';

  export type ToastGlyph =
    | 'favorited'
    | 'unfavorited'
    | 'removed'
    | 'deleted'
    | 'copied'
    | 'download'
    | 'cleared';

  interface Props {
    id: number | string;
    variant: ToastVariant;
    title: string;
    description?: string;
    swatch?: string;
    glyph?: ToastGlyph;
    actionLabel?: string;
    onaction?: () => void;
    ondismiss?: () => void;
    dismissible?: boolean;
    duration?: number;
    repeat?: number;
  }

  let {
    id,
    variant,
    title,
    description,
    swatch,
    glyph,
    actionLabel,
    onaction,
    ondismiss,
    dismissible = true,
    duration = Number.POSITIVE_INFINITY,
    repeat = 1
  }: Props = $props();

  const countsDown = $derived(Number.isFinite(duration) && duration > 0);
  const runKey = $derived(`${variant}|${title}|${description ?? ''}|${duration}`);

  let mounted = $state(false);
  onMount(() => {
    mounted = true;
  });

  const GLYPHS = {
    success: IconSuccess,
    error: IconError,
    warning: IconWarning,
    info: IconInfo,
    loading: IconLoading
  };

  const ACTION_GLYPHS: Record<ToastGlyph, typeof IconSuccess> = {
    favorited: IconFavorited,
    unfavorited: IconUnfavorited,
    removed: IconRemoved,
    deleted: IconDeleted,
    copied: IconCopied,
    download: IconDownload,
    cleared: IconCleared
  };

  const Glyph = $derived(glyph ? ACTION_GLYPHS[glyph] : GLYPHS[variant]);

  function dismiss() {
    if (ondismiss) ondismiss();
    else toast.dismiss(id);
  }
</script>

<div
  class="toast"
  class:has-desc={Boolean(description)}
  class:is-stacked={Boolean(actionLabel) && actionLabel!.length > 8}
  class:is-swapped={mounted}
  data-variant={variant}
  style:--toast-duration={countsDown ? `${duration}ms` : undefined}
>
  {#key runKey}
    <span class="toast__glyph" aria-hidden="true">
      <Glyph />
    </span>

    <div class="toast__text">
      <p class="toast__title">
        {title}
        {#if repeat > 1}
          <span class="toast__repeat">&times;{repeat}</span>
        {/if}
      </p>
      {#if description}
        <p class="toast__desc">
          {#if swatch}
            <span class="toast__swatch" style:background={swatch} aria-hidden="true"></span>
          {/if}{description}
        </p>
      {/if}
    </div>

    {#if actionLabel}
      <button
        type="button"
        class="toast__action"
        use:ripple
        onclick={() => {
          onaction?.();
          dismiss();
        }}
      >
        {actionLabel}
      </button>
    {/if}

    {#if dismissible}
      <button
        type="button"
        class="toast__close"
        use:ripple
        aria-label={i18n.t('common.close')}
        onclick={dismiss}
      >
        <IconClose />
        {#if countsDown}
          <svg class="toast__ring" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="10.5" />
          </svg>
        {/if}
      </button>
    {/if}
  {/key}
</div>

<style>
  .toast {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    grid-template-areas: "glyph text action close";
    align-items: center;
    column-gap: var(--toast-gap);
    width: 100%;
    min-height: 48px;
    padding: var(--toast-padding-y) 8px var(--toast-padding-y) var(--toast-padding-x);
    border-radius: var(--toast-radius);
    background: var(--toast-surface);
    box-shadow: var(--toast-shadow);
    color: var(--text-primary);
    box-sizing: border-box;
    overflow: hidden;
  }

  .toast.is-stacked {
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-areas:
      "glyph text close"
      ". action action";
    row-gap: 4px;
  }

  .toast:not(:has(.toast__close)):not(:has(.toast__action)) {
    padding-inline-end: var(--toast-padding-x);
  }

  .toast__glyph { grid-area: glyph; }
  .toast__text { grid-area: text; }
  .toast__action { grid-area: action; justify-self: end; }
  .toast__close { grid-area: close; }

  .toast.has-desc {
    align-items: start;
  }

  .toast__glyph {
    display: grid;
    place-items: center;
    flex: none;
    height: var(--type-body-md-line);
    color: var(--text-secondary);
  }

  .toast__glyph :global(svg) {
    width: var(--toast-icon-size);
    height: var(--toast-icon-size);
  }

  .toast[data-variant='error'] .toast__glyph {
    color: var(--status-error);
  }

  .toast[data-variant='warning'] .toast__glyph {
    color: var(--status-warning);
  }

  .toast__text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 auto;
    min-width: 0;
  }

  .toast__title {
    margin: 0;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--type-body-md-size);
    line-height: var(--type-body-md-line);
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .toast__desc {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .toast__action,
  .toast__close {
    flex: none;
    border: 0;
    background: transparent;
    outline: none;
    transition: background-color var(--duration-fast) var(--ease-standard),
                color var(--duration-fast) var(--ease-standard);
  }

  .toast.has-desc .toast__action,
  .toast.has-desc .toast__close {
    margin-block: calc((var(--type-body-md-line) - var(--toast-close-size)) / 2);
  }

  .toast__action {
    height: var(--toast-close-size);
    padding: 0 12px;
    border-radius: var(--radius-full);
    color: var(--accent-on-surface);
    font-family: var(--font-sans);
    font-size: var(--type-label-lg-size);
    line-height: var(--type-label-lg-line);
    font-weight: 500;
    white-space: nowrap;
  }

  .toast__close {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--toast-close-size);
    height: var(--toast-close-size);
    margin-inline-start: -4px;
    padding: 0;
    border-radius: var(--radius-full);
    color: var(--text-secondary);
  }

  .toast__close :global(svg:not(.toast__ring)) {
    width: var(--toast-close-icon);
    height: var(--toast-close-icon);
  }

  @media (hover: hover) {
    .toast__action:hover,
    .toast__close:hover {
      background: var(--state-layer-hover);
    }

    .toast__close:hover {
      color: var(--text-primary);
    }
  }

  .toast__action:focus-visible,
  .toast__close:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .toast__swatch {
    display: inline-block;
    width: var(--toast-swatch-size);
    height: var(--toast-swatch-size);
    margin-inline-end: 6px;
    border-radius: var(--radius-full);
    vertical-align: baseline;
  }

  .toast__repeat {
    margin-inline-start: 4px;
    color: var(--text-secondary);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }

  .toast.is-swapped .toast__glyph,
  .toast.is-swapped .toast__text,
  .toast.is-swapped .toast__action,
  .toast.is-swapped .toast__close {
    animation: toast-swap var(--duration-normal) var(--ease-merge) both;
  }

  @keyframes toast-swap {
    from {
      opacity: 0;
      transform: translateX(calc(var(--toast-merge-shift) * -1)) scale(0.94);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  .toast__ring {
    position: absolute;
    inset: 4px;
    width: calc(100% - 8px);
    height: calc(100% - 8px);
    fill: none;
    stroke: currentColor;
    stroke-width: var(--toast-ring-width);
    stroke-linecap: round;
    opacity: 0.6;
    transform: rotate(-90deg);
    stroke-dasharray: var(--toast-ring-length);
    animation: toast-ring var(--toast-duration) linear forwards;
    pointer-events: none;
  }

  /* Tracks sonner's timer, which holds on toaster hover and swipe; data-expanded is always true under `expand`. */
  :global([data-sonner-toast][data-swiping='true']) .toast__ring {
    animation-play-state: paused;
  }

  @media (hover: hover) {
    :global([data-sonner-toaster]:hover) .toast__ring {
      animation-play-state: paused;
    }
  }

  @keyframes toast-ring {
    from {
      stroke-dashoffset: 0;
    }
    to {
      stroke-dashoffset: var(--toast-ring-length);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast.is-swapped .toast__glyph,
    .toast.is-swapped .toast__text,
    .toast.is-swapped .toast__action,
    .toast.is-swapped .toast__close {
      animation: none;
    }

    .toast__ring {
      display: none;
    }
  }

  .toast.is-stacked .toast__action {
    margin-block: 0;
  }
</style>
