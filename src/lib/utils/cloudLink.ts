import type { CloudLinkContext, CloudLockKind } from '$lib/types/cloud';
import type { Post } from '$lib/types/content';

export function errorCode(error: unknown): string {
  return (error instanceof Error ? error.message : String(error)).trim();
}

export function cloudLockKind(error: unknown): CloudLockKind | null {
  const code = errorCode(error);
  if (code === 'password_required') return 'password';
  if (code === 'key_required') return 'key';
  return null;
}

export function postSecretText(post: Pick<Post, 'title' | 'content'>): string {
  return [post.title, post.content].filter(Boolean).join('\n');
}

export function cloudLinkContext(post: Pick<Post, 'service' | 'user' | 'title' | 'content'>): CloudLinkContext {
  return {
    service: post.service,
    creator_id: post.user,
    post_text: postSecretText(post) || undefined
  };
}
