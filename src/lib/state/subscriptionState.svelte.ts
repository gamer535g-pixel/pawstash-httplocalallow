import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CreatorSubscription, SubscriptionInput } from '$lib/types/subscription';
import {
  apiClearSubscriptionPosts,
  apiCountSubscriptionPosts,
  apiDeleteSubscription,
  apiListSubscriptions,
  apiRefreshSubscription,
  apiSetSubscriptionEnabled,
  apiUpsertSubscription
} from '$lib/utils/ipc';

export interface SubscriptionDialogTarget {
  service: string;
  creatorId: string;
  creatorName: string;
  avatarUrl?: string | null;
  batch?: { service: string; id: string; name: string }[];
}

export interface UnsubscribePromptTarget {
  subscription?: CreatorSubscription;
  creatorName?: string;
  batch?: CreatorSubscription[];
}

class SubscriptionState {
  items = $state<CreatorSubscription[]>([]);
  loading = $state(false);

  dialogOpen = $state(false);
  dialogTarget = $state<SubscriptionDialogTarget | null>(null);

  unsubscribePromptOpen = $state(false);
  unsubscribeTarget = $state<UnsubscribePromptTarget | null>(null);

  private initialized = false;
  private unlisten?: UnlistenFn;

  async init() {
    if (this.initialized) return;
    this.initialized = true;
    this.unlisten = await listen<CreatorSubscription>('subscription-updated', ({ payload }) => this.upsert(payload));
    await this.reload();
  }

  async reload() {
    this.loading = true;
    try { this.items = await apiListSubscriptions(); }
    finally { this.loading = false; }
  }

  private byCreator = $derived(
    new Map(this.items.map((item) => [`${item.service}:${item.creator_id}`, item]))
  );

  forCreator(service: string, creatorId: string) {
    return this.byCreator.get(`${service}:${creatorId}`);
  }

  async save(input: SubscriptionInput) {
    const item = await apiUpsertSubscription(input);
    this.upsert(item);
    return item;
  }

  async setEnabled(id: string, enabled: boolean) { this.upsert(await apiSetSubscriptionEnabled(id, enabled)); }
  async refresh(id: string) { this.upsert(await apiRefreshSubscription(id)); }

  async remove(id: string, removePosts = false) {
    if (await apiDeleteSubscription(id, removePosts)) {
      this.items = this.items.filter((item) => item.id !== id);
      return true;
    }
    return false;
  }

  async clearPosts(id: string) {
    return await apiClearSubscriptionPosts(id);
  }

  async countPosts(id: string) {
    return await apiCountSubscriptionPosts(id);
  }

  openDialog(target: SubscriptionDialogTarget) {
    this.dialogTarget = target;
    this.dialogOpen = true;
  }

  closeDialog() {
    this.dialogOpen = false;
    this.dialogTarget = null;
  }

  promptUnsubscribe(subscription: CreatorSubscription, creatorName: string) {
    this.unsubscribeTarget = { subscription, creatorName };
    this.unsubscribePromptOpen = true;
  }

  promptUnsubscribeBatch(subscriptions: CreatorSubscription[]) {
    this.unsubscribeTarget = { batch: subscriptions };
    this.unsubscribePromptOpen = true;
  }

  closeUnsubscribePrompt() {
    this.unsubscribePromptOpen = false;
    this.unsubscribeTarget = null;
  }

  private upsert(item: CreatorSubscription) {
    const index = this.items.findIndex((current) => current.id === item.id);
    if (index < 0) this.items = [...this.items, item];
    else this.items = this.items.map((current, i) => i === index ? item : current);
  }
}

export const subscriptionState = new SubscriptionState();
