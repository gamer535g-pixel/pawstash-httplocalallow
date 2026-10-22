import { listen } from '@tauri-apps/api/event';
import { apiListPostInnerFormats, type IndexedFormats } from '$lib/utils/ipc';
import type { Post } from '$lib/types/content';

function keyOf(service: string, creatorId: string, postId: string) {
  return `${service}:${creatorId}:${postId}`;
}

class InnerFormatsState {
  private byPost = $state<Record<string, string[]>>({});
  private initialized = false;

  async init() {
    if (this.initialized) return;
    this.initialized = true;
    await listen<IndexedFormats>('post-contents-indexed', ({ payload }) => this.store(payload));
    for (const row of await apiListPostInnerFormats()) this.store(row);
  }

  of(post: Post): string[] {
    return this.byPost[keyOf(post.service, post.user, post.id)] ?? [];
  }

  private store(row: IndexedFormats) {
    this.byPost[keyOf(row.service, row.creator_id, row.post_id)] = row.formats;
  }
}

export const innerFormatsState = new InnerFormatsState();
