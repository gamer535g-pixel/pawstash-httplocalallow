<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { onBackButtonPress } from '@tauri-apps/api/app';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n } from '$lib/i18n';
  import { navigationState } from '$lib/state/navigationState.svelte';
  import { configState } from '$lib/state/configState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { downloadState } from '$lib/state/downloadState.svelte';
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { accountState } from '$lib/state/accountState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { layoutState } from '$lib/state/layoutState.svelte';
  import { creatorsState } from '$lib/state/creatorsState.svelte';
  import { providerState } from '$lib/state/providerState.svelte';
  import { apiGetSettings, apiGetPendingDeepLink, apiSetAppHidden, apiShowMainWindow } from '$lib/utils/ipc';
  import BackgroundProvider from '$lib/components/providers/BackgroundProvider.svelte';
  import SidebarNav from '$lib/components/layout/SidebarNav.svelte';
  import MobileBottomNav from '$lib/components/layout/MobileBottomNav.svelte';
  import SettingsPage from '$lib/components/settings/SettingsPage.svelte';
  import SettingsNav from '$lib/components/settings/SettingsNav.svelte';
  import FeedView from '$lib/components/content/FeedView.svelte';
  import PostPage from '$lib/components/content/PostPage.svelte';
  import CreatorPage from '$lib/components/content/CreatorPage.svelte';
  import LibraryView from '$lib/components/library/LibraryView.svelte';
  import DownloadQueueList from '$lib/components/downloads/DownloadQueueList.svelte';
  import CreatorsView from '$lib/components/content/CreatorsView.svelte';
  import FavoritesView from '$lib/components/content/FavoritesView.svelte';
  import ProfilePage from '$lib/components/profile/ProfilePage.svelte';
  import UpdateModal from '$lib/components/ui/UpdateModal.svelte';
  import NoticeModal from '$lib/components/ui/NoticeModal.svelte';
  import { noticeState } from '$lib/state/noticeState.svelte';
  import { innerFormatsState } from '$lib/state/innerFormatsState.svelte';
  import { uiScaleState } from '$lib/state/uiScaleState.svelte';
  import { scaleIndicator } from '$lib/state/scaleState.svelte';
  import IconGrid from '~icons/fluent/grid-24-regular';
  import IconWindow from '~icons/fluent/window-24-regular';
  import SubscriptionModal from '$lib/components/content/SubscriptionModal.svelte';
  import UnsubscribeDialog from '$lib/components/content/UnsubscribeDialog.svelte';
  import { updateState } from '$lib/state/updateState.svelte';
  import { selectionState } from '$lib/state/selectionState.svelte';
  import { onboardingState } from '$lib/state/onboardingState.svelte';
  import { initFrontendLogging, logger } from '$lib/utils/logger';
  import { initDeepLinkListener, handleDeepLinkUrl } from '$lib/utils/deepLink';
  import { initRestartNotice } from '$lib/utils/restartNotice';
  import Toaster from '$lib/components/ui/Toaster.svelte';
  import ContextMenu from '$lib/components/ui/ContextMenu.svelte';

  navigationState.init();

  onMount(() => {
    initFrontendLogging();
    themeState.init();
    i18n.init();
    document.documentElement.classList.toggle('webkitgtk', layoutState.isLinux);

    const preventPinchZoom = (e: TouchEvent) => {
      if (e.touches.length > 1) {
        e.preventDefault();
      }
    };
    document.addEventListener('touchstart', preventPinchZoom, { passive: false });
    document.addEventListener('touchmove', preventPinchZoom, { passive: false });

    function handleDeepLinkPayload(payloadStr: string | null) {
      if (!payloadStr) return;
      if (
        payloadStr.startsWith('pawstash://') ||
        payloadStr.startsWith('http://') ||
        payloadStr.startsWith('https://')
      ) {
        void handleDeepLinkUrl(payloadStr);
        return;
      }
      try {
        const payload = JSON.parse(payloadStr);
        if (payload.action === 'open_post' && payload.service && payload.creatorId && payload.postId) {
          navigationState.openPost(payload.service, payload.creatorId, payload.postId, undefined, true);
        } else if (payload.action === 'open_downloads') {
          navigationState.navigateRoot('downloads');
        }
      } catch (e) {
        logger.warn('Failed to parse deep link payload', e);
      }
    }

    const cleanupDeepLink = initDeepLinkListener();
    const cleanupRestartNotice = initRestartNotice();
    const reportVisibility = () =>
      void apiSetAppHidden(document.hidden).catch((e) => logger.debug('Failed to report window visibility', e));
    document.addEventListener('visibilitychange', reportVisibility);
    void apiGetPendingDeepLink()
      .then(handleDeepLinkPayload)
      .catch((e) => logger.warn('Failed to read pending deep link', e));
    const unlistenDeepLink = listen<string>('open-post-deep-link', (event) => {
      handleDeepLinkPayload(event.payload);
    });
    const unlistenPanic = listen('panic-mode', () => {
      document.querySelectorAll('video, audio').forEach((el) => {
        try {
          (el as HTMLMediaElement).pause();
        } catch {}
      });
    });

    void apiGetSettings()
      .then((settings) => {
        configState.updateSettings(settings);
        // Read once: transparency is fixed when the window is created.
        document.documentElement.classList.toggle(
          'opaque-window',
          layoutState.isLinux && settings.linux_transparent_window !== true
        );
        if (settings.auto_check_updates ?? true) {
          setTimeout(async () => {
            await noticeState.check();
            await noticeState.whenClosed();
            void updateState.check(true);
          }, 3000);
        }
      })
      .catch((error) => logger.warn('Failed to load application settings', error));
    void libraryState.init().catch((error) => logger.warn('Failed to initialize library', error));
    void downloadState.init().catch((error) => logger.warn('Failed to initialize downloads', error));
    void subscriptionState.init().catch((error) => logger.warn('Failed to initialize subscriptions', error));
    void accountState.refresh().catch((error) => logger.warn('Failed to check Pawchive session', error));
    void syncState.init().catch((error) => logger.warn('Failed to initialize encrypted sync', error));
    void innerFormatsState.init().catch((error) => logger.warn('Failed to load indexed post contents', error));
    void uiScaleState.init().then(() => {
      const legacyScale = themeState.takeLegacyUiScale();
      if (legacyScale && !uiScaleState.percent) uiScaleState.set(legacyScale);
    });
    void providerState.loadProviders().catch((error) => logger.warn('Failed to load providers', error));
    void creatorsState.init().catch((error) => logger.warn('Failed to initialize creators list', error));

    requestAnimationFrame(() => {
      void emit('frontend-ready');
      void apiShowMainWindow().catch((e) => logger.warn('Failed to show main window', e));
    });

    return () => {
      document.removeEventListener('touchstart', preventPinchZoom);
      document.removeEventListener('touchmove', preventPinchZoom);
      cleanupDeepLink();
      cleanupRestartNotice();
      document.removeEventListener('visibilitychange', reportVisibility);
      void unlistenDeepLink.then((u) => u());
      void unlistenPanic.then((u) => u());
    };
  });

  $effect(() => {
    document.documentElement.dataset.layout = layoutState.isMobile ? 'mobile' : 'desktop';
    document.documentElement.dataset.platform = layoutState.isMobileDevice ? 'touch' : 'desktop';
  });

  onMount(() => {
    if (!layoutState.isAndroid) return;
    let listener: { unregister: () => Promise<void> } | undefined;
    void onBackButtonPress(() => {
      if (!navigationState.handleSystemBack()) void invoke('android_minimize');
    }).then((registered) => (listener = registered));
    return () => void listener?.unregister();
  });

  $effect(() => {
    if (!libraryState.initialized || !syncState.statusLoaded) return;
    onboardingState.observeLibrary(libraryState.savedCount, libraryState.stashCount, syncState.status.configured);
  });

  $effect(() => {
    if (!selectionState.active) return;
    return untrack(() =>
      navigationState.registerBackHandler(() => {
        selectionState.exit();
        return true;
      })
    );
  });
</script>

<div
  class="app-shell h-screen w-screen flex overflow-hidden select-none text-[var(--text-primary)] font-sans relative"
>
  <BackgroundProvider />
  <Toaster />
  <ContextMenu />

  {#if !layoutState.isMobile}
    {#if navigationState.route.name === 'settings'}
      <SettingsNav />
    {:else}
      <SidebarNav />
    {/if}
  {/if}

  <div class="app-content flex-1 flex overflow-hidden relative min-w-0">
    <main class="flex-1 flex overflow-hidden w-full h-full">
      {#key navigationState.entryKey}
        {#if navigationState.route.name === 'feed'}
          <FeedView />
        {:else if navigationState.route.name === 'favorites'}
          <FavoritesView />
        {:else if navigationState.route.name === 'library'}
          <LibraryView />
        {:else if navigationState.route.name === 'downloads'}
          <DownloadQueueList />
        {:else if navigationState.route.name === 'creators'}
          <CreatorsView />
        {:else if navigationState.route.name === 'profile'}
          <ProfilePage />
        {:else if navigationState.route.name === 'post'}
          <PostPage
            service={navigationState.route.service}
            creatorId={navigationState.route.creatorId}
            postId={navigationState.route.postId}
            initialMedia={navigationState.route.initialMedia}
            openViewer={navigationState.route.openViewer}
          />
        {:else if navigationState.route.name === 'creator'}
          <CreatorPage
            service={navigationState.route.service}
            creatorId={navigationState.route.creatorId}
            initialTag={navigationState.route.initialTag}
          />
        {:else if navigationState.route.name === 'settings'}
          <SettingsPage />
        {/if}
      {/key}
    </main>
  </div>

  {#if layoutState.isMobile}
    <MobileBottomNav />
  {/if}

  <UpdateModal />
  <NoticeModal />
  {#if scaleIndicator.kind}
    <div class="scale-indicator" role="status">
      {#if scaleIndicator.kind === 'grid'}<IconGrid />{:else}<IconWindow />{/if}
      {i18n.t(scaleIndicator.kind === 'grid' ? 'settings.grid_scale_indicator' : 'settings.ui_scale_indicator', { percent: scaleIndicator.percent })}
    </div>
  {/if}
  <SubscriptionModal />
  <UnsubscribeDialog />
</div>
