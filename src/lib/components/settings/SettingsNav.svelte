<script lang="ts">
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { i18n } from '$lib/i18n';
  import { scrollable } from '$lib/actions/scrollable';
  import Button from '$lib/components/ui/Button.svelte';
  import WindowControls from '$lib/components/layout/WindowControls.svelte';
  import NavItem from '$lib/components/layout/NavItem.svelte';
  import IconArrowLeft from '~icons/fluent/arrow-left-24-regular';
  import { SECTION_GROUPS, navSectionOf, sectionMeta } from './sections';
  import VersionLine from './VersionLine.svelte';

  let active = $derived(
    navigationState.route.name === 'settings' ? navSectionOf(navigationState.route.section) : null
  );
  let isMacStyle = $derived(layoutState.effectiveTitlebarStyle === 'macos');
</script>

<nav class="settings-nav" class:is-mac={isMacStyle} aria-label={i18n.t('nav.settings')} data-tauri-drag-region>
  <div class="settings-nav__header" data-tauri-drag-region="deep">
    {#if isMacStyle}
      <WindowControls variant="macos" />
    {/if}
    <Button variant="ghost" class="btn-icon" aria-label={i18n.t('nav.back')} onclick={() => navigationState.exitSettings()}>
      <IconArrowLeft />
    </Button>
    <h1 class="settings-nav__title">{i18n.t('nav.settings')}</h1>
  </div>

  <div
    class="settings-nav__scroll"
    data-overlayscrollbars-initialize
    use:scrollable={{ overflowX: 'hidden' }}
    data-tauri-drag-region="deep"
  >
    <div class="settings-nav__list">
      {#each SECTION_GROUPS as group}
        <div class="settings-nav__group">
          {#each group as id (id)}
            {@const meta = sectionMeta(id)}
            <NavItem
              label={i18n.t(meta.labelKey)}
              icon={meta.icon}
              iconActive={meta.iconActive}
              active={active === id}
              onclick={() => {
                if (navigationState.route.name !== 'settings' || navigationState.route.section !== id) {
                  navigationState.openSettings(id);
                }
              }}
            />
          {/each}
        </div>
      {/each}
    </div>
  </div>

  <VersionLine />
</nav>

<style>
  .settings-nav {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    width: max-content;
    min-width: calc(var(--nav-min-width) + var(--edge-left, 0px));
    max-width: calc(var(--nav-max-width) + var(--edge-left, 0px));
    height: 100%;
    padding: var(--edge-top, 0px) 0 var(--edge-bottom, 0px) var(--edge-left, 0px);
    background: var(--chrome-surface);
  }

  .settings-nav__header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    height: var(--toolbar-height);
    padding-inline: 16px 12px;
    --control-height: var(--control-height-base);
    --control-icon-size: var(--control-icon-base);
  }

  .settings-nav.is-mac .settings-nav__header {
    gap: 12px;
    padding-inline-start: 20px;
  }

  .settings-nav__title {
    margin: 0;
    color: var(--text-primary);
    font-size: var(--type-title-lg-size);
    line-height: var(--type-title-lg-line);
    font-weight: 400;
  }

  .settings-nav__scroll {
    flex: 1 1 0;
    min-height: 0;
  }

  .settings-nav__list {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 8px 12px 16px;
  }

  .settings-nav__group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
