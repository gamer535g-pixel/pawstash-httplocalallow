<script lang="ts">
  import { haptic } from '$lib/utils/haptics';
  import { onMount } from 'svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { logger } from '$lib/utils/logger';
  import ProgressRing from '$lib/components/ui/ProgressRing.svelte';

  interface Props {
    onrefresh?: () => Promise<void> | void;
    scrollContainer?: HTMLElement | null;
    disabled?: boolean;
    class?: string;
  }

  let {
    onrefresh,
    scrollContainer,
    disabled = false,
    class: extraClass = ''
  }: Props = $props();

  const THRESHOLD = 68;
  const MAX_PULL = 104;
  const RESTING_OFFSET = 58;
  const RESISTANCE = 0.44;

  let pullDistance = $state(0);
  let pulling = $state(false);
  let refreshing = $state(false);
  let exiting = $state(false);
  let thresholdPassed = $state(false);

  let startY = 0;
  let startX = 0;
  let isTracking = false;

  let pullProgress = $derived(Math.min(1, pullDistance / THRESHOLD));
  let isVisible = $derived(pullDistance > 4 || refreshing || exiting);

  function getScrollTop(): number {
    if (!scrollContainer) return 0;
    return scrollContainer.scrollTop;
  }

  function handleTouchStart(e: TouchEvent) {
    if (!layoutState.isMobile || disabled || refreshing || !onrefresh || e.touches.length !== 1) {
      return;
    }

    if (getScrollTop() > 2) {
      isTracking = false;
      return;
    }

    startY = e.touches[0].clientY;
    startX = e.touches[0].clientX;
    isTracking = true;
    pulling = false;
    thresholdPassed = false;
  }

  function handleTouchMove(e: TouchEvent) {
    if (!isTracking || refreshing || disabled || !onrefresh || e.touches.length !== 1) {
      return;
    }

    if (getScrollTop() > 1) {
      isTracking = false;
      if (pullDistance > 0) {
        pullDistance = 0;
        pulling = false;
      }
      return;
    }

    const currentY = e.touches[0].clientY;
    const currentX = e.touches[0].clientX;
    const diffY = currentY - startY;
    const diffX = Math.abs(currentX - startX);

    if (diffX > diffY && diffX > 8) {
      isTracking = false;
      return;
    }

    if (diffY > 0 && getScrollTop() <= 0) {
      pulling = true;
      const raw = diffY * RESISTANCE;
      pullDistance = Math.min(MAX_PULL, raw);

      if (pullDistance >= THRESHOLD && !thresholdPassed) {
        thresholdPassed = true;
        haptic(10);
      } else if (pullDistance < THRESHOLD && thresholdPassed) {
        thresholdPassed = false;
      }

      if (pullDistance > 6 && e.cancelable) {
        e.preventDefault();
      }
    }
  }

  function settle() {
    pulling = false;
    exiting = true;
    pullDistance = 0;
    setTimeout(() => {
      exiting = false;
      thresholdPassed = false;
    }, 250);
  }

  async function handleTouchEnd() {
    if (!isTracking && !pulling) return;
    isTracking = false;

    if (pullDistance >= THRESHOLD && onrefresh && !refreshing) {
      refreshing = true;
      pulling = false;
      pullDistance = RESTING_OFFSET;

      try {
        await onrefresh();
      } catch (err) {
        logger.warn('Pull-to-refresh action failed', err);
      } finally {
        refreshing = false;
        settle();
      }
    } else {
      settle();
    }
  }

  onMount(() => {
    window.addEventListener('touchstart', handleTouchStart, { passive: true });
    window.addEventListener('touchmove', handleTouchMove, { passive: false });
    window.addEventListener('touchend', handleTouchEnd, { passive: true });
    window.addEventListener('touchcancel', handleTouchEnd, { passive: true });

    return () => {
      window.removeEventListener('touchstart', handleTouchStart);
      window.removeEventListener('touchmove', handleTouchMove);
      window.removeEventListener('touchend', handleTouchEnd);
      window.removeEventListener('touchcancel', handleTouchEnd);
    };
  });
</script>

{#if isVisible && layoutState.isMobile}
  <div
    class="ptr-indicator-wrapper {extraClass}"
    class:pulling
    class:refreshing
    class:ready={thresholdPassed}
    class:exiting
    style:--ptr-offset="{pullDistance}px"
    style:--ptr-progress={pullProgress}
    aria-hidden="true"
  >
    <div class="ptr-disc" style:transform={refreshing ? undefined : `rotate(${pullProgress * 270}deg)`}>
      <ProgressRing value={refreshing ? null : pullProgress * 0.8} size={24} stroke={2.5} />
    </div>
  </div>
{/if}

<style>
  .ptr-indicator-wrapper {
    position: absolute;
    top: calc(var(--page-bar-space, var(--mobile-status-bar-height, 0px)) + 8px);
    left: 50%;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: var(--ptr-progress, 0);
    transform: translate(-50%, var(--ptr-offset, 0px));
    pointer-events: none;
    transition: transform var(--duration-normal) var(--ease-standard),
                opacity var(--duration-fast) var(--ease-standard);
    will-change: transform, opacity;
  }

  .ptr-indicator-wrapper.pulling {
    transition: none;
  }

  .ptr-indicator-wrapper.refreshing {
    opacity: 1;
  }

  .ptr-indicator-wrapper.exiting {
    opacity: 0;
    transform: translate(-50%, 0) scale(0.6);
  }

  .ptr-disc {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--floating-bg);
    box-shadow: var(--floating-shadow);
    transition: transform var(--duration-fast) var(--ease-standard);
  }

  .ptr-indicator-wrapper.pulling .ptr-disc {
    transition: none;
  }

  .ptr-disc :global(.ring-track) {
    stroke: transparent;
  }

  .ptr-disc :global(.ring-value) {
    transition: none;
  }

  .ptr-indicator-wrapper:not(.ready):not(.refreshing) .ptr-disc :global(.ring-value) {
    opacity: 0.6;
  }
</style>
