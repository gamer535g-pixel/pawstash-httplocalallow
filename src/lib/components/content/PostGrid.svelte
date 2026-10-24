<script module lang="ts">
  const gridScrollPositions = new Map<string, number>();
</script>

<script lang="ts">
  import { paintSelect } from '$lib/actions/paintSelect';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { getContext, onDestroy } from 'svelte';
  import { SCROLLABLE_CONTEXT, type ScrollableContext } from '$lib/actions/scrollable';
  import type { Post } from '$lib/types/content';
  import { configState } from '$lib/state/configState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { GRID_RATIOS, gridScaleWheel } from '$lib/state/scaleState.svelte';
  import { i18n } from '$lib/i18n';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import PostCard from './PostCard.svelte';
  import SkeletonGrid from '$lib/components/ui/SkeletonGrid.svelte';
  import { GridHandoff } from '$lib/motion/gridHandoff.svelte';
  import IconEmpty from '~icons/fluent/image-off-24-regular';
  import IconLoading from '~icons/svg-spinners/3-dots-fade';
  import { ctrlWheel } from '$lib/utils/ctrlWheel';

  interface Props {
    posts: Post[];
    loading?: boolean;
    hasMore?: boolean;
    onLoadMore?: () => void | Promise<void>;
    stateKey?: string;
    paginationKey?: string | number;
    ariaLabel?: string;
    emptyTitle?: string;
    emptyDescription?: string;
    showCreator?: boolean;
  }

  let {
    posts,
    loading = false,
    hasMore = false,
    onLoadMore,
    stateKey = 'post-grid',
    paginationKey,
    ariaLabel = i18n.t('feed.title'),
    emptyTitle = i18n.t('feed.empty'),
    emptyDescription = i18n.t('feed.empty_desc'),
    showCreator = true
  }: Props = $props();
  let root: HTMLDivElement;
  let loadSentinel: HTMLDivElement;
  const scrollContext = getContext<ScrollableContext | undefined>(SCROLLABLE_CONTEXT);
  let viewport: HTMLElement | null = null;
  let resizeObserver: ResizeObserver | undefined;
  let intersectionObserver: IntersectionObserver | undefined;
  let viewportFrame: number | undefined;
  let gridWidth = $state(0);
  let viewportHeight = $state(0);
  let relativeScroll = $state(0);
  let gridTop = 0;
  let activeStateKey = $state('');
  let pendingScrollRestore = $state<number | null>(null);
  let lastRequestKey = '';
  const ratios = { square: '1 / 1', portrait: '4 / 5', landscape: '3 / 2', widescreen: '16 / 9' } as const;
  const overscanRows = 3;

  let postKeys = $derived(posts.map((p) => `${(p.service || '').toLowerCase()}:${p.user}:${p.id}`));
  let postsMap = $derived(new Map(posts.map((p) => [`${(p.service || '').toLowerCase()}:${p.user}:${p.id}`, p])));

  $effect(() => {
    selectionState.setContext('posts', postKeys, postsMap);
  });

  let baseCardWidth = $derived(layoutState.isMobile ? 155 : 245);
  let scale = $derived(configState.settings.grid_scale / 100);
  let gap = $derived(Math.round((layoutState.isMobile ? 8 : 10) * scale));
  let targetCardWidth = $derived(baseCardWidth * scale);
  let columnCount = $derived(Math.max(1, Math.floor((gridWidth + gap) / (targetCardWidth + gap))));
  let cardWidth = $derived(columnCount > 0 ? (gridWidth - gap * (columnCount - 1)) / columnCount : gridWidth);
  let cardHeight = $derived(Math.round(cardWidth / GRID_RATIOS[configState.settings.grid_aspect_ratio]));
  let rowStride = $derived(cardHeight + gap);
  let rowCount = $derived(Math.ceil(posts.length / columnCount));
  let virtualHeight = $derived(rowCount > 0 ? rowCount * cardHeight + (rowCount - 1) * gap : 0);
  let firstRow = $derived(rowStride > 0 ? Math.max(0, Math.floor(relativeScroll / rowStride) - overscanRows) : 0);
  let lastRow = $derived(gridWidth > 0 && rowStride > 0
    ? Math.min(rowCount, Math.ceil((relativeScroll + viewportHeight) / rowStride) + overscanRows)
    : 0);
  let visibleRows = $derived.by(() => Array.from(
    { length: Math.max(0, lastRow - firstRow) },
    (_, offset) => {
      const index = firstRow + offset;
      return { index, posts: posts.slice(index * columnCount, (index + 1) * columnCount) };
    }
  ));

  const handoff = new GridHandoff();
  $effect(() => handoff.sync(posts.length > 0, loading));
  onDestroy(() => handoff.destroy());

  function updateSizeMetrics() {
    if (!root || !viewport) return;
    const viewportRect = viewport.getBoundingClientRect();
    gridWidth = root.clientWidth;
    viewportHeight = viewport.clientHeight;
    gridTop = root.getBoundingClientRect().top - viewportRect.top + viewport.scrollTop;
  }

  function updateScrollMetrics(savePosition = true) {
    if (!viewport) return;
    relativeScroll = Math.max(0, viewport.scrollTop - gridTop);
    if (savePosition) gridScrollPositions.set(activeStateKey, viewport.scrollTop);
  }

  function handleScroll() {
    updateScrollMetrics(true);
    if (pendingScrollRestore !== null && viewport) {
      if (Math.abs(viewport.scrollTop - pendingScrollRestore) > 5) {
        pendingScrollRestore = null;
      }
    }
    if (viewport && hasMore && !loading) {
      const scrollBottom = viewport.scrollTop + (viewportHeight || viewport.clientHeight);
      const totalHeight = virtualHeight > 0 ? virtualHeight : (viewport.scrollHeight || 0);
      if (scrollBottom >= totalHeight - 1200) {
        requestMore();
      }
    }
  }

  function requestMore() {
    const requestKey = `${stateKey}:${paginationKey ?? posts.length}`;
    if (!hasMore || loading || !onLoadMore || lastRequestKey === requestKey) return;
    lastRequestKey = requestKey;
    queueMicrotask(() => void onLoadMore());
  }

  function attachViewport(nextViewport: HTMLElement | null) {
    if (viewport === nextViewport) return;
    if (viewport) viewport.removeEventListener('scroll', handleScroll);
    resizeObserver?.disconnect();
    intersectionObserver?.disconnect();
    if (viewportFrame) cancelAnimationFrame(viewportFrame);
    
    viewport = nextViewport;
    if (!viewport) return;
    
    viewport.addEventListener('scroll', handleScroll, { passive: true });
    
    resizeObserver = new ResizeObserver(() => {
      updateSizeMetrics();
      updateScrollMetrics(false);
    });
    resizeObserver.observe(viewport);
    
    intersectionObserver = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) requestMore();
      },
      { root: viewport, rootMargin: '1200px 0px' }
    );
    intersectionObserver.observe(loadSentinel);
    
    viewportFrame = requestAnimationFrame(() => {
      if (!viewport) return;
      updateSizeMetrics();
      updateScrollMetrics(true);
    });
  }

  $effect(() => {
    if (root) {
      attachViewport(scrollContext?.viewport ?? null);
    }
    return () => {
      if (viewport) viewport.removeEventListener('scroll', handleScroll);
      resizeObserver?.disconnect();
      intersectionObserver?.disconnect();
      if (viewportFrame) cancelAnimationFrame(viewportFrame);
    };
  });

  $effect(() => {
    const nextStateKey = stateKey;
    activeStateKey = nextStateKey;
    lastRequestKey = '';
    if (viewport) {
      const saved = gridScrollPositions.get(nextStateKey);
      pendingScrollRestore = saved !== undefined ? saved : 0;
    }
  });

  $effect(() => {
    if (!loading) {
      lastRequestKey = '';
    }
  });

  $effect(() => {
    if (posts.length > 0 && hasMore && !loading && viewport) {
      const scrollBottom = viewport.scrollTop + (viewportHeight || viewport.clientHeight);
      const totalHeight = virtualHeight > 0 ? virtualHeight : (viewport.scrollHeight || 0);
      if (scrollBottom >= totalHeight - 600) {
        requestMore();
      }
    }
  });

  $effect(() => {
    if (pendingScrollRestore !== null && viewport && virtualHeight > 0) {
      const target = pendingScrollRestore;
      requestAnimationFrame(() => {
        if (!viewport || pendingScrollRestore === null) return;
        viewport.scrollTop = target;
        updateSizeMetrics();
        updateScrollMetrics(true);
        if (target === 0 || viewport.scrollTop > 0 || Math.abs(viewport.scrollTop - target) < 2) {
          pendingScrollRestore = null;
        }
      });
    }
  });

  $effect(() => {
    configState.settings.grid_scale;
    configState.settings.grid_aspect_ratio;
    if (root) {
      requestAnimationFrame(() => {
        updateSizeMetrics();
        updateScrollMetrics(false);
      });
    }
  });

  $effect(() => {
    const remaining = virtualHeight - (relativeScroll + viewportHeight);
    const nearEnd = viewportHeight > 0 && remaining <= Math.max(viewportHeight * 2, rowStride * 4);
    if (nearEnd && posts.length > 0) requestMore();
  });

  onDestroy(() => {
    if (viewport) {
      gridScrollPositions.set(activeStateKey, viewport.scrollTop);
      viewport.removeEventListener('scroll', handleScroll);
    }
    resizeObserver?.disconnect();
    intersectionObserver?.disconnect();
    if (viewportFrame) cancelAnimationFrame(viewportFrame);
  });
</script>


<div
  bind:this={root}
  class="grid-root"
  use:paintSelect={{ scope: 'posts', keys: postKeys, items: postsMap }}
  role="feed"
  aria-label={ariaLabel}
  {@attach ctrlWheel(gridScaleWheel)}
  style={`--grid-scale: ${scale}; --grid-card-width: ${Math.round(targetCardWidth)}px; --grid-ratio: ${ratios[configState.settings.grid_aspect_ratio]}; --grid-gap: ${gap}px;`}
>

  {#if posts.length > 0}
    <div class="virtual-grid" style:height={`${virtualHeight}px`}>
      {#each visibleRows as row (row.index)}
        <div
          class="virtual-row"
          style={`transform: translateY(${Math.round(row.index * rowStride)}px); grid-template-columns: repeat(${columnCount}, minmax(0, 1fr)); gap: ${gap}px;`}
        >
          {#each row.posts as post, column (`${post.service}:${post.user}:${post.id}`)}
            <PostCard
              {post}
              {showCreator}
              orderedKeys={postKeys}
              itemsMap={postsMap}
              enterDelay={handoff.delayFor(row.index * columnCount + column)}
            />
          {/each}
        </div>
      {/each}
    </div>
  {:else if !handoff.skeletonMounted}
    <EmptyState icon={IconEmpty} title={emptyTitle} description={emptyDescription} />
  {/if}

  {#if handoff.skeletonMounted}
    <SkeletonGrid
      cardWidth={targetCardWidth}
      {gap}
      fading={handoff.skeletonFading}
    />
  {/if}

  <div class="load-sentinel" bind:this={loadSentinel} aria-hidden="true"></div>

  {#if posts.length > 0 && loading}
    <div class="loading-tail"><IconLoading />{i18n.t('feed.loading')}</div>
  {/if}
</div>

<style>
  .grid-root { position: relative; width: 100%; }
  .virtual-grid { position: relative; width: 100%; contain: layout style; }
  .virtual-row { position: absolute; top: 0; left: 0; right: 0; display: grid; align-items: start; will-change: transform; }
  /* Per-row layers cost WebKitGTK more than repainting the moved rows. */
  :global(html.webkitgtk) .virtual-row { will-change: auto; }
  .loading-tail { display: flex; align-items: center; justify-content: center; gap: 8px; min-height: 48px; color: var(--text-muted); font-size: 11px; }
  .load-sentinel { width: 100%; height: 1px; pointer-events: none; }
  .loading-tail :global(svg) { width: 20px; height: 20px; color: var(--text-primary); }
</style>
