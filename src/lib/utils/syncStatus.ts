import { syncState } from '$lib/state/syncState.svelte';
import { i18n } from '$lib/i18n';

export function syncStatusText(): string {
  if (syncState.isSyncing) {
    const progress = syncState.progress;
    const phase =
      syncState.phase === 'connecting'
        ? 'sync.status_connecting'
        : syncState.phase === 'pushing'
          ? 'sync.status_pushing'
          : syncState.phase === 'pulling'
            ? 'sync.status_pulling'
            : 'sync.status_syncing';
    return progress === null || progress === undefined
      ? i18n.t(phase)
      : `${i18n.t(phase)} ${i18n.number(progress, { style: 'percent', maximumFractionDigits: 0 })}`;
  }
  if (!syncState.status.configured) return i18n.t('profile.offline_session');
  return syncState.status.unlocked ? i18n.t('sync.status_ready') : i18n.t('sync.locked');
}
