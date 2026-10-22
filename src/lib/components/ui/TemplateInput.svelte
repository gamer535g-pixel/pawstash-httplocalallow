<script lang="ts">
  import { tick } from 'svelte';
  import { templatePreview } from '$lib/utils/templatePreview';
  import { computePosition, autoUpdate, flip, shift, offset, size as floatingSize } from '@floating-ui/dom';
  import { FLOATING_EDGE, FLOATING_OFFSET } from '$lib/utils/floating';
  import { portal } from '$lib/actions/portal';
  import { scrollable } from '$lib/actions/scrollable';
  import { i18n } from '$lib/i18n';
  import { ripple, tooltip } from '$lib/motion';
  import { notify } from '$lib/utils/toast';
  import IconCode from '~icons/fluent/code-24-regular';
  import IconEye from '~icons/fluent/eye-24-regular';

  export interface TemplateTag {
    tag: string;
    label: string;
    example?: string;
  }

  interface Props {
    size?: 'sm' | 'base' | 'md' | 'lg';
    value?: string;
    placeholder?: string;
    tags?: TemplateTag[];
    previewType?: 'creator' | 'post' | 'file';
    disabled?: boolean;
    onchange?: (value: string) => void;
    onblur?: () => void;
    class?: string;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    tags = [],
    previewType = 'file',
    disabled = false,
    onchange,
    onblur,
    size,
    class: extraClass = ''
  }: Props = $props();

  let inputEl = $state<HTMLInputElement>();
  let rootEl = $state<HTMLDivElement>();
  let dropdownEl = $state<HTMLDivElement | null>(null);
  let isOpen = $state(false);
  let selectedIndex = $state(0);
  let cleanupAutoUpdate: (() => void) | null = null;
  let closeTimeout: ReturnType<typeof setTimeout> | undefined;

  let filterQuery = $state('');

  let filteredTags = $derived.by(() => {
    if (!filterQuery.trim()) return tags;
    const q = filterQuery.toLowerCase().replace(/[{}]/g, '');
    return tags.filter(
      (t) =>
        t.tag.toLowerCase().includes(q) ||
        t.label.toLowerCase().includes(q) ||
        (t.example && t.example.toLowerCase().includes(q))
    );
  });

  let previewValue = $derived(templatePreview(value, previewType));

  async function updatePosition() {
    if (!rootEl || !dropdownEl) return;
    const { x, y } = await computePosition(rootEl, dropdownEl, {
      placement: 'bottom-start',
      strategy: 'fixed',
      middleware: [
        offset(FLOATING_OFFSET),
        flip({ fallbackPlacements: ['top-start', 'bottom-end', 'top-end'], padding: FLOATING_EDGE }),
        shift({ padding: FLOATING_EDGE }),
        floatingSize({
          padding: FLOATING_EDGE,
          apply({ availableHeight, elements }: { availableHeight: number; elements: any }) {
            Object.assign(elements.floating.style, {
              maxHeight: `${Math.min(availableHeight, 320)}px`
            });
          }
        })
      ]
    });

    if (!dropdownEl) return;
    dropdownEl.style.left = `${x}px`;
    dropdownEl.style.top = `${y}px`;
    dropdownEl.style.visibility = 'visible';
  }

  $effect(() => {
    if (isOpen && rootEl && dropdownEl) {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = autoUpdate(rootEl, dropdownEl, () => {
        void updatePosition();
      });
    } else {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    }
    return () => {
      cleanupAutoUpdate?.();
      cleanupAutoUpdate = null;
    };
  });

  function openDropdown() {
    if (closeTimeout) clearTimeout(closeTimeout);
    filterQuery = '';
    selectedIndex = 0;
    isOpen = true;
  }

  function scheduleClose() {
    closeTimeout = setTimeout(() => {
      isOpen = false;
      onblur?.();
    }, 180);
  }

  function handleFocus() {
    openDropdown();
  }

  function handleInput(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    value = target.value;
    onchange?.(value);

    const cursor = target.selectionStart ?? target.value.length;
    const beforeCursor = target.value.slice(0, cursor);
    const lastOpenBrace = beforeCursor.lastIndexOf('{');
    const lastCloseBrace = beforeCursor.lastIndexOf('}');

    if (lastOpenBrace > lastCloseBrace) {
      filterQuery = beforeCursor.slice(lastOpenBrace);
      selectedIndex = 0;
      isOpen = true;
    } else {
      filterQuery = '';
    }
  }

  function insertTag(tag: string) {
    if (!inputEl) {
      value = value ? `${value}${tag}` : tag;
      onchange?.(value);
      isOpen = false;
      return;
    }

    const start = inputEl.selectionStart ?? value.length;
    const end = inputEl.selectionEnd ?? value.length;

    const before = value.slice(0, start);
    const after = value.slice(end);
    const lastOpen = before.lastIndexOf('{');
    const lastClose = before.lastIndexOf('}');

    let newBefore = before;
    if (lastOpen > lastClose && lastOpen >= 0) {
      newBefore = before.slice(0, lastOpen);
    }

    value = `${newBefore}${tag}${after}`;
    onchange?.(value);

    const newCursor = newBefore.length + tag.length;
    isOpen = false;

    void tick().then(() => {
      inputEl?.focus();
      inputEl?.setSelectionRange(newCursor, newCursor);
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!isOpen) {
      if (e.key === 'ArrowDown' || e.key === 'Enter') {
        openDropdown();
        e.preventDefault();
      }
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, filteredTags.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filteredTags.length) % Math.max(1, filteredTags.length);
    } else if (e.key === 'Enter' || e.key === 'Tab') {
      if (filteredTags.length > 0 && selectedIndex < filteredTags.length) {
        e.preventDefault();
        insertTag(filteredTags[selectedIndex].tag);
      }
    } else if (e.key === 'Escape') {
      isOpen = false;
    }
  }
</script>

<div class="template-input-wrapper {extraClass}" bind:this={rootEl}>
  <div
    class="template-box {size ? `size-${size}` : ''}"
    class:is-disabled={disabled}
    class:is-open={isOpen}
  >
    <input
      bind:this={inputEl}
      type="text"
      {placeholder}
      {disabled}
      bind:value
      onfocus={handleFocus}
      onblur={scheduleClose}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      class="native-input"
      autocomplete="off"
      spellcheck="false"
    />

    <div class="right-actions">
      {#if previewValue}
        <button
          type="button"
          class="field-action"
          use:ripple
          use:tooltip={previewValue}
          onclick={(e) => {
            e.stopPropagation();
            notify.info(previewValue);
          }}
          aria-label={i18n.t('settings.template_preview')}
          tabindex="-1"
        >
          <IconEye />
        </button>
      {/if}

      <button
        type="button"
        class="field-action"
        class:is-active={isOpen}
        use:ripple
        use:tooltip={i18n.t('settings.insert_variable')}
        onclick={(e) => {
          e.stopPropagation();
          if (isOpen) isOpen = false;
          else {
            inputEl?.focus();
            openDropdown();
          }
        }}
        aria-label={i18n.t('settings.insert_variable')}
        tabindex="-1"
      >
        <IconCode />
      </button>
    </div>
  </div>
</div>

{#if isOpen && filteredTags.length > 0}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    use:portal={'body'}
    bind:this={dropdownEl}
    class="template-dropdown floating-surface"
    role="listbox"
    tabindex="-1"
  >
    <div class="dropdown-list" use:scrollable>
      {#each filteredTags as item, i (item.tag)}
        <button
          type="button"
          class="floating-item tag-item"
          use:ripple
          onmousedown={(e) => {
            e.preventDefault();
            insertTag(item.tag);
          }}
          onmouseenter={() => (selectedIndex = i)}
          role="option"
          aria-selected={i === selectedIndex}
        >
          <span class="floating-item-label">{item.label}</span>
          <code class="floating-item-detail is-mono">{item.tag}</code>
        </button>
      {/each}
    </div>
  </div>
{/if}

<style>
  .template-input-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    position: relative;
  }

  .template-box {
    display: flex;
    align-items: center;
    width: 100%;
    height: calc(var(--control-height, 34px) * var(--ui-scale, 1));
    padding: 0 calc(var(--control-padding-x, 12px) * var(--ui-scale, 1));
    gap: calc(8px * var(--ui-scale, 1));
    background: var(--input-bg, rgba(255, 255, 255, 0.06));
    border: none;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    box-sizing: border-box;
    outline: none !important;
    box-shadow: none !important;
    transition: background var(--duration-fast) var(--ease-expo),
                border-color var(--duration-fast) var(--ease-expo);
  }

  .template-box.size-sm {
    --control-height: var(--control-height-sm, 34px);
    --control-font-size: var(--control-font-sm, 12.5px);
    --control-icon-size: var(--control-icon-sm, 16px);
    --control-padding-x: var(--control-padding-sm, 10px);
  }

  .template-box.size-base {
    --control-height: var(--control-height-base, 40px);
    --control-font-size: var(--control-font-base, 13.5px);
    --control-icon-size: var(--control-icon-base, 18px);
    --control-padding-x: var(--control-padding-base, 12px);
  }

  .template-box.size-md {
    --control-height: var(--control-height-md, 46px);
    --control-font-size: var(--control-font-md, 14px);
    --control-icon-size: var(--control-icon-md, 20px);
    --control-padding-x: var(--control-padding-md, 14px);
  }

  .template-box.size-lg {
    --control-height: var(--control-height-lg, 52px);
    --control-font-size: var(--control-font-lg, 15px);
    --control-icon-size: var(--control-icon-lg, 22px);
    --control-padding-x: var(--control-padding-lg, 18px);
  }

  .template-box:hover,
  .template-box:focus-within,
  .template-box.is-open {
    background: var(--input-bg-hover, rgba(255, 255, 255, 0.095));
  }

  .template-box:focus-visible,
  .template-box:has(:focus-visible) {
    outline: calc(1.5px * var(--ui-scale, 1)) solid var(--accent-primary) !important;
    outline-offset: calc(-1.5px * var(--ui-scale, 1)) !important;
    box-shadow: none !important;
  }

  .template-box.is-disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .native-input {
    flex: 1;
    min-width: 0;
    height: 100%;
    background: transparent !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    padding: 0;
    color: var(--text-primary);
    font-size: calc(var(--control-font-size, 13px) * var(--ui-scale, 1));
    font-family: var(--font-sans);
    box-sizing: border-box;
  }

  .native-input:focus,
  .native-input:focus-visible {
    outline: none !important;
    border: none !important;
    box-shadow: none !important;
  }

  .native-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.85;
  }

  .right-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    margin-left: auto;
  }

  .template-dropdown {
    position: fixed;
    z-index: 10000;
    display: flex;
    flex-direction: column;
    width: min(320px, calc(100vw - 32px));
    overflow: hidden;
    visibility: hidden;
    animation: dropdownFadeIn var(--duration-fast) var(--ease-standard);
  }

  @keyframes dropdownFadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .dropdown-list {
    display: flex;
    flex-direction: column;
    width: 100%;
    max-height: 240px;
    box-sizing: border-box;
  }

  .dropdown-list :global(.os-viewport) {
    width: 100%;
    box-sizing: border-box;
  }

  .dropdown-list :global(.os-content) {
    display: flex !important;
    flex-direction: column !important;
    width: 100% !important;
    box-sizing: border-box !important;
  }

  .tag-item[aria-selected='true'] {
    background: var(--state-layer-hover);
    color: var(--text-primary);
  }

  .tag-item .floating-item-detail {
    color: var(--accent-on-surface);
  }
</style>
