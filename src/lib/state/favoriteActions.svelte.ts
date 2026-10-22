import type { Creator, Post } from '$lib/types/content';
import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';
import { apiSetCreatorFavorite, apiSetPostFavorite } from '$lib/utils/ipc';
import { accountState } from '$lib/state/accountState.svelte';

const postKey = (post: Pick<Post, 'service' | 'user' | 'id'>) => `post:${post.service}:${post.user}:${post.id}`;
const creatorKey = (creator: Pick<Creator, 'service' | 'id'>) => `creator:${creator.service}:${creator.id}`;

function announce(added: boolean, description?: string) {
  const key = accountState.session.authenticated
    ? added ? 'post.added_to_favorites' : 'post.removed_from_favorites'
    : added ? 'favorites.saved_locally' : 'favorites.removed_locally';
  notify.success(i18n.t(key), { description, glyph: added ? 'favorited' : 'unfavorited' });
}

class FavoriteActions {
  pending = $state<Record<string, boolean>>({});

  isPostPending(post: Pick<Post, 'service' | 'user' | 'id'>) {
    return Boolean(this.pending[postKey(post)]);
  }

  isCreatorPending(creator: Pick<Creator, 'service' | 'id'>) {
    return Boolean(this.pending[creatorKey(creator)]);
  }

  async togglePost(post: Post) {
    const key = postKey(post);
    if (this.pending[key]) return;
    this.pending[key] = true;
    const target = !accountState.isPostFavorite(post.service, post.user, post.id);
    if (target) accountState.addPostFavoriteOptimistic(post);
    else accountState.removePostFavoriteOptimistic(post.service, post.user, post.id);
    announce(target);
    try {
      await apiSetPostFavorite(post.service, post.user, post.id, target);
    } catch (error) {
      if (target) accountState.removePostFavoriteOptimistic(post.service, post.user, post.id);
      else accountState.addPostFavoriteOptimistic(post);
      notify.error(i18n.t('post.favorite_failed'), error);
    } finally {
      this.pending[key] = false;
    }
  }

  async toggleCreator(creator: Pick<Creator, 'id' | 'service' | 'name'>) {
    const key = creatorKey(creator);
    if (this.pending[key]) return;
    this.pending[key] = true;
    const target = !accountState.isCreatorFavorite(creator.service, creator.id);
    try {
      await apiSetCreatorFavorite(creator.service, creator.id, target);
      if (target) accountState.addCreatorFavoriteOptimistic(creator as Creator);
      else accountState.removeCreatorFavoriteOptimistic(creator.service, creator.id);
      announce(target, creator.name);
    } catch (error) {
      notify.error(i18n.t('post.favorite_failed'), error);
    } finally {
      this.pending[key] = false;
    }
  }
}

export const favoriteActions = new FavoriteActions();
