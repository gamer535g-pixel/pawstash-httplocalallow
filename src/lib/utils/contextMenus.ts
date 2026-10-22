import type { Creator, Post } from '$lib/types/content';
import type { DownloadItem } from '$lib/types/download';
import type { MenuItem } from '$lib/components/ui/MenuList.svelte';
import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';
import { copyText } from '$lib/utils/clipboard';
import { apiOpenDownloadFile, apiShowInFolder, openExternalUrl } from '$lib/utils/ipc';
import { formatProviderName, formatServiceName, getPlatformPostUrl, getPlatformProfileUrl, parentFolder } from '$lib/utils/media';
import { accountState } from '$lib/state/accountState.svelte';
import { contentState } from '$lib/state/contentState.svelte';
import { downloadState } from '$lib/state/downloadState.svelte';
import { favoriteActions } from '$lib/state/favoriteActions.svelte';
import { libraryState } from '$lib/state/libraryState.svelte';
import { navigationState } from '$lib/state/navigationState.svelte';
import { providerState } from '$lib/state/providerState.svelte';
import { subscriptionState } from '$lib/state/subscriptionState.svelte';
import IconSelect from '~icons/fluent/checkbox-checked-24-regular';
import IconOpenPost from '~icons/fluent/document-arrow-right-24-regular';
import IconPersonOpen from '~icons/fluent/person-arrow-right-24-regular';
import IconOpen from '~icons/fluent/open-24-regular';
import IconGlobe from '~icons/fluent/globe-24-regular';
import IconHeart from '~icons/fluent/heart-24-regular';
import IconHeartOff from '~icons/fluent/heart-off-24-regular';
import IconBookmarkAdd from '~icons/fluent/bookmark-add-24-regular';
import IconBookmarkOff from '~icons/fluent/bookmark-off-24-regular';
import IconPersonAdd from '~icons/fluent/person-add-24-regular';
import IconPersonSubscribed from '~icons/fluent/person-available-24-regular';
import IconLink from '~icons/fluent/link-24-regular';
import IconLinkSquare from '~icons/fluent/link-square-24-regular';
import IconNumberSymbol from '~icons/fluent/number-symbol-24-regular';
import IconEye from '~icons/fluent/eye-24-regular';
import IconAppOpen from '~icons/fluent/window-arrow-up-24-regular';
import IconFolderOpen from '~icons/fluent/folder-open-24-regular';
import IconPause from '~icons/fluent/pause-24-regular';
import IconPlay from '~icons/fluent/play-24-regular';
import IconRetry from '~icons/fluent/arrow-counterclockwise-24-regular';
import IconDelete from '~icons/fluent/delete-24-regular';
import IconDismiss from '~icons/fluent/dismiss-24-regular';

const selectItem = (onselect: () => void): MenuItem => ({
  group: 'select',
  label: i18n.t('context.select'),
  icon: IconSelect,
  onclick: onselect
});

function run(task: Promise<unknown>, failure: string) {
  task.catch((error) => notify.error(failure, error));
}

export function postMenuItems(post: Post, onselect: () => void): MenuItem[] {
  const providerName = formatProviderName(providerState.getProviderForService(post.service)?.name || '');
  const originalUrl = getPlatformPostUrl(post.service, post.user, post.id);
  const favorited = accountState.isPostFavorite(post.service, post.user, post.id);
  const saved = libraryState.isSaved(post);
  return [
    selectItem(onselect),
    {
      group: 'open',
      label: i18n.t('context.open_post'),
      icon: IconOpenPost,
      onclick: () => {
        contentState.seedPost(post);
        navigationState.openPost(post.service, post.user, post.id);
      }
    },
    { group: 'open', label: i18n.t('feed.open_creator'), icon: IconPersonOpen, onclick: () => navigationState.openCreator(post.service, post.user) },
    ...(originalUrl
      ? [{ group: 'open', label: i18n.t('post.open_original_post'), detail: formatServiceName(post.service), icon: IconOpen, onclick: () => void openExternalUrl(originalUrl) }]
      : []),
    ...(post.page_url
      ? [{ group: 'open', label: i18n.t('post.open_in_provider'), detail: providerName || undefined, icon: IconGlobe, onclick: () => void openExternalUrl(post.page_url!) }]
      : []),
    {
      group: 'keep',
      label: i18n.t(favorited ? 'context.remove_favorite' : 'context.add_favorite'),
      icon: favorited ? IconHeartOff : IconHeart,
      onclick: () => void favoriteActions.togglePost(post)
    },
    {
      group: 'keep',
      label: i18n.t(saved ? 'library.remove' : 'library.save'),
      icon: saved ? IconBookmarkOff : IconBookmarkAdd,
      onclick: () =>
        libraryState
          .toggle(post)
          .then(() => notify.success(i18n.t(saved ? 'library.removed' : 'library.saved'), post.title || undefined))
          .catch((error) => notify.error(i18n.t('library.save_error'), error))
    },
    ...(post.page_url
      ? [{ group: 'copy', label: i18n.t('post.copy_link'), detail: providerName || undefined, icon: IconLink, onclick: () => copyText(post.page_url!, i18n.t('post.link_copied')) }]
      : []),
    ...(originalUrl
      ? [{ group: 'copy', label: i18n.t('post.copy_link'), detail: formatServiceName(post.service), icon: IconLinkSquare, onclick: () => copyText(originalUrl, i18n.t('post.link_copied')) }]
      : []),
    { group: 'copy', label: i18n.t('post.copy_id'), detail: `#${post.id}`, detailMono: true, icon: IconNumberSymbol, onclick: () => copyText(post.id) }
  ];
}

export function creatorMenuItems(creator: Creator, onselect: () => void): MenuItem[] {
  const profileUrl = getPlatformProfileUrl(creator.service, creator.id, creator.public_id);
  const favorited = accountState.isCreatorFavorite(creator.service, creator.id);
  const subscribed = Boolean(subscriptionState.forCreator(creator.service, creator.id));
  return [
    selectItem(onselect),
    { group: 'open', label: i18n.t('feed.open_creator'), icon: IconPersonOpen, onclick: () => navigationState.openCreator(creator.service, creator.id) },
    ...(profileUrl
      ? [{ group: 'open', label: i18n.t('creator.open_original_profile'), detail: formatServiceName(creator.service), icon: IconOpen, onclick: () => void openExternalUrl(profileUrl) }]
      : []),
    {
      group: 'keep',
      label: i18n.t(favorited ? 'context.remove_favorite' : 'context.add_favorite'),
      icon: favorited ? IconHeartOff : IconHeart,
      onclick: () => void favoriteActions.toggleCreator(creator)
    },
    {
      group: 'keep',
      label: i18n.t(subscribed ? 'subscriptions.subscription' : 'subscriptions.subscribe'),
      icon: subscribed ? IconPersonSubscribed : IconPersonAdd,
      onclick: () =>
        subscriptionState.openDialog({
          service: creator.service,
          creatorId: creator.id,
          creatorName: creator.name || creator.id,
          avatarUrl: creator.avatar_url
        })
    },
    ...(profileUrl
      ? [{ group: 'copy', label: i18n.t('post.copy_link'), detail: formatServiceName(creator.service), icon: IconLinkSquare, onclick: () => copyText(profileUrl, i18n.t('post.link_copied')) }]
      : []),
    { group: 'copy', label: i18n.t('post.copy_id'), detail: creator.id, detailMono: true, icon: IconNumberSymbol, onclick: () => copyText(creator.id) }
  ];
}

function downloadControlItems(items: DownloadItem[]): MenuItem[] {
  const active = items.filter((item) => ['queued', 'resolving', 'downloading'].includes(item.status));
  const paused = items.filter((item) => item.status === 'paused');
  const failed = items.filter((item) => ['failed', 'cancelled', 'missing'].includes(item.status));
  const each = (jobs: DownloadItem[], operation: (id: string) => Promise<void>) => () =>
    run(Promise.all(jobs.map((job) => operation(job.id))), i18n.t('downloads.action_error'));
  return [
    ...(active.length ? [{ group: 'control', label: i18n.t('downloads.pause'), icon: IconPause, onclick: each(active, (id) => downloadState.pause(id)) }] : []),
    ...(paused.length ? [{ group: 'control', label: i18n.t('downloads.resume'), icon: IconPlay, onclick: each(paused, (id) => downloadState.resume(id)) }] : []),
    ...(failed.length ? [{ group: 'control', label: i18n.t('downloads.retry'), icon: IconRetry, onclick: each(failed, (id) => downloadState.retry(id)) }] : []),
    {
      group: 'remove',
      label: i18n.t(active.length ? 'downloads.cancel' : 'downloads.remove'),
      icon: active.length ? IconDismiss : IconDelete,
      danger: true,
      onclick: each(items, (id) => downloadState.remove(id))
    }
  ];
}

interface DownloadMenuOptions {
  onselect: () => void;
  onopen?: (inPost: boolean) => void;
  hasPost: boolean;
}

export function downloadMenuItems(item: DownloadItem, { onselect, onopen, hasPost }: DownloadMenuOptions): MenuItem[] {
  const completed = item.status === 'completed';
  return [
    selectItem(onselect),
    ...(completed && onopen ? [{ group: 'open', label: i18n.t('context.view'), icon: IconEye, onclick: () => onopen(false) }] : []),
    ...(hasPost && onopen ? [{ group: 'open', label: i18n.t('context.open_in_post'), icon: IconOpenPost, onclick: () => onopen(true) }] : []),
    ...(completed
      ? [
          { group: 'open', label: i18n.t('context.open_in_app'), icon: IconAppOpen, onclick: () => run(apiOpenDownloadFile(item.final_path), i18n.t('downloads.open_file_failed')) },
          { group: 'open', label: i18n.t('downloads.show_in_folder'), icon: IconFolderOpen, onclick: () => run(apiShowInFolder(item.final_path), i18n.t('downloads.show_in_folder_failed')) }
        ]
      : []),
    ...downloadControlItems([item])
  ];
}

interface DownloadGroupMenuOptions {
  onselect: () => void;
  onopen?: () => void;
  oncreator?: () => void;
}

export function downloadGroupMenuItems(items: DownloadItem[], { onselect, onopen, oncreator }: DownloadGroupMenuOptions): MenuItem[] {
  const first = items.find((item) => item.status === 'completed');
  return [
    selectItem(onselect),
    ...(onopen ? [{ group: 'open', label: i18n.t('context.open_post'), icon: IconOpenPost, onclick: onopen }] : []),
    ...(oncreator ? [{ group: 'open', label: i18n.t('feed.open_creator'), detail: items[0]?.creator_name || undefined, icon: IconPersonOpen, onclick: oncreator }] : []),
    ...(first
      ? [{ group: 'open', label: i18n.t('downloads.open_post_folder'), icon: IconFolderOpen, onclick: () => run(apiShowInFolder(parentFolder(first.final_path)), i18n.t('downloads.show_in_folder_failed')) }]
      : []),
    ...downloadControlItems(items)
  ];
}
