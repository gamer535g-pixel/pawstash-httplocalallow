import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';
import { navigationState } from '$lib/state/navigationState.svelte';

const STORAGE_KEY = 'pawstash_onboarding';
const SYNC_NUDGE_AT = 10;

interface Stored {
  checklistDismissed: boolean;
  communityJoined: boolean;
  syncNudgeShown: boolean;
}

function load(): Stored {
  const fallback: Stored = { checklistDismissed: false, communityJoined: false, syncNudgeShown: false };
  if (typeof localStorage === 'undefined') return fallback;
  try {
    return { ...fallback, ...JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') };
  } catch {
    return fallback;
  }
}

class OnboardingState {
  private data = $state<Stored>(load());
  pendingProfileView = $state<'create' | null>(null);
  private baseline: { saved: number; stashes: number } | null = null;

  get checklistDismissed() {
    return this.data.checklistDismissed;
  }

  get communityJoined() {
    return this.data.communityJoined;
  }

  private update(patch: Partial<Stored>) {
    this.data = { ...this.data, ...patch };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.data));
    } catch {
    }
  }

  dismissChecklist() {
    this.update({ checklistDismissed: true });
  }

  markCommunityJoined() {
    if (!this.data.communityJoined) this.update({ communityJoined: true });
  }

  openSyncSetup() {
    this.pendingProfileView = 'create';
    navigationState.navigateRoot('profile');
  }

  takeProfileView() {
    const view = this.pendingProfileView;
    this.pendingProfileView = null;
    return view;
  }

  observeLibrary(saved: number, stashes: number, syncConfigured: boolean) {
    if (!this.baseline) {
      this.baseline = { saved, stashes };
      return;
    }
    const grew = saved >= SYNC_NUDGE_AT && this.baseline.saved < SYNC_NUDGE_AT;
    const newStash = stashes > this.baseline.stashes;
    this.baseline = { saved, stashes };
    if (this.data.syncNudgeShown || syncConfigured || !(grew || newStash)) return;
    this.update({ syncNudgeShown: true });
    notify.info(i18n.t('onboarding.sync_nudge_title'), i18n.t('onboarding.sync_nudge_desc'), {
      duration: 20000,
      action: { label: i18n.t('onboarding.sync_nudge_action'), onclick: () => this.openSyncSetup() }
    });
  }
}

export const onboardingState = new OnboardingState();
