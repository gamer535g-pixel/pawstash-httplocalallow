<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { backgroundState, defaultAcrylicOpacity, isWindowsPlatform } from '$lib/theme/backgroundState.svelte';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { isIdentityFilter, renderFilteredBackground } from '$lib/theme/backgroundFilter';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { logger } from '$lib/utils/logger';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { resolveLocalMediaUrl, videoPoster } from '$lib/utils/media';

  onMount(() => {
    backgroundState.init();
  });

  $effect(() => {
    const style = backgroundState.settings.type === 'oled' ? 'oled' : 'flat';
    untrack(() => themeState.setSurfaceStyle(style));
  });

  let settings = $derived(backgroundState.settings);

  function toAssetUrl(path: string): string {
    if (!path) return '';
    if (/^(https?:|asset:|blob:|data:)/i.test(path)) return path;
    const local = resolveLocalMediaUrl(path);
    if (local) return local;
    try {
      return convertFileSrc(path);
    } catch {
      return path;
    }
  }

  let filterStyle = $derived(
    `filter: blur(${settings.blurPx}px) brightness(${settings.brightness}) saturate(${settings.saturation}); opacity: ${settings.opacity};`
  );

  let innerWidth = $state(0);
  let innerHeight = $state(0);

  // WebKitGTK re-runs a live filter on this full-window layer on every repaint, so Linux bakes it once.
  let prefiltered = $state<{ source: string; url: string } | null>(null);
  let prefilterFailedFor = $state<string | null>(null);
  let imageSrc = $derived(settings.customKind === 'image' && settings.imageUrl ? toAssetUrl(settings.imageUrl) : '');
  let imageFilter = $derived({ blurPx: settings.blurPx, brightness: settings.brightness, saturation: settings.saturation });
  let bakesImage = $derived(layoutState.isLinux && settings.type === 'custom' && Boolean(imageSrc));
  let imageNeedsFilter = $derived(!isIdentityFilter(imageFilter));

  function releasePrefiltered() {
    if (prefiltered) URL.revokeObjectURL(prefiltered.url);
    prefiltered = null;
  }

  $effect(() => {
    const source = imageSrc;
    const filter = imageFilter;
    const layer = { width: Math.round(innerWidth * 1.12), height: Math.round(innerHeight * 1.12) };
    void backgroundState.version;
    if (!bakesImage || !imageNeedsFilter || layer.width === 0 || prefilterFailedFor === source) {
      untrack(releasePrefiltered);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(async () => {
      try {
        const blob = await renderFilteredBackground(source, filter, layer, window.devicePixelRatio || 1);
        if (cancelled) return;
        releasePrefiltered();
        prefiltered = { source, url: URL.createObjectURL(blob) };
      } catch (error) {
        if (cancelled) return;
        logger.warn('Could not pre-render the background image, using the live CSS filter', { source, error: String(error) });
        releasePrefiltered();
        prefilterFailedFor = source;
      }
    }, 150);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  onDestroy(releasePrefiltered);

  let isNativeEffect = $derived.by(() => {
    if (isWindowsPlatform() && ['acrylic', 'mica-dark', 'tabbed'].includes(settings.type)) return true;
    if (typeof navigator !== 'undefined' && navigator.userAgent.toLowerCase().includes('mac') && settings.type === 'vibrancy') return true;
    return false;
  });
</script>

<svelte:window bind:innerWidth bind:innerHeight />

<div class="fixed inset-0 z-[-2] overflow-hidden pointer-events-none select-none">
  {#if settings.type === 'palette'}
    <div class="absolute inset-0" style="background: var(--surface);"></div>

  {:else if settings.type === 'oled' || (!isNativeEffect && settings.type !== 'custom')}
    <div class="absolute inset-0" style="background: var(--bg-base);"></div>

  {:else if settings.type === 'custom'}
    {#if settings.customKind === 'video' && settings.videoUrl}
      {@const videoSrc = toAssetUrl(settings.videoUrl)}
      {#key `${videoSrc}:${backgroundState.version}`}
        <video
          src={videoSrc}
          poster={videoPoster()}
          autoplay
          loop
          muted
          playsinline
          class="custom-background-media"
          style={filterStyle}
        ></video>
      {/key}
    {:else if settings.customKind === 'image' && settings.imageUrl}
      {@const imgSrc = imageSrc}
      {@const baked = bakesImage && prefiltered?.source === imgSrc ? prefiltered.url : null}
      {@const style = baked
        ? `background-image: url('${baked}'); opacity: ${settings.opacity};`
        : bakesImage && !imageNeedsFilter
          ? `background-image: url('${imgSrc}'); opacity: ${settings.opacity};`
          : `background-image: url('${imgSrc}'); ${filterStyle}`}
      {#key `${imgSrc}:${backgroundState.version}`}
        <div class="custom-background-media bg-cover bg-center" {style}></div>
      {/key}
    {:else if settings.customKind === 'palette'}
      {@const p = themeState.palette}
      <div
        class="custom-background-media"
        style="background: radial-gradient(circle at 18% 22%, {p.quadrants[0]}44 0%, transparent 48%), radial-gradient(circle at 82% 16%, {p.quadrants[1]}38 0%, transparent 44%), radial-gradient(circle at 75% 78%, {p.quadrants[3]}34 0%, transparent 52%), radial-gradient(circle at 22% 82%, {p.quadrants[2]}2e 0%, transparent 46%), linear-gradient(135deg, {p.surface} 0%, {p.surfaceBright} 100%); {filterStyle}"
      ></div>
    {:else}
      <div
        class="custom-background-media"
        style="background: linear-gradient(135deg, {settings.solidColor}, {settings.gradientSecondary}); {filterStyle}"
      ></div>
    {/if}
  {/if}

  {#if settings.acrylicTint}
    <div
      class="absolute inset-0 pointer-events-none transition-colors duration-150"
      style="background: rgba(var(--acrylic-tint-rgb), {settings.acrylicOpacity ?? defaultAcrylicOpacity(themeState.isDark)});"
    ></div>
  {/if}
</div>

<style>
  .custom-background-media {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 112%;
    height: 112%;
    min-width: 112%;
    min-height: 112%;
    max-width: none !important;
    max-height: none !important;
    object-fit: cover;
    transition: filter var(--duration-normal), opacity var(--duration-normal);
  }

  :global(html.webkitgtk) .custom-background-media {
    transition: opacity var(--duration-normal);
  }
</style>
