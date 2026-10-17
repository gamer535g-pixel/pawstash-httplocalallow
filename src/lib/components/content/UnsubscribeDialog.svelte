<script lang="ts">
  import { subscriptionState } from '$lib/state/subscriptionState.svelte';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Checkbox from '$lib/components/ui/Checkbox.svelte';
  import IconWarning from '~icons/fluent/warning-24-filled';
  import IconPersonDelete from '~icons/fluent/person-delete-24-regular';

  const target = $derived(subscriptionState.unsubscribeTarget);
  const isOpen = $derived(subscriptionState.unsubscribePromptOpen && Boolean(target));

  const isBatch = $derived(Boolean(target?.batch && target.batch.length > 0));

  let removePosts = $state(true);
  let loading = $state(false);
  let postCount = $state<number | null>(null);

  $effect(() => {
    if (isOpen && target) {
      removePosts = true;
      if (!isBatch && target.subscription) {
        subscriptionState
          .countPosts(target.subscription.id)
          .then((count) => {
            postCount = count;
          })
          .catch(() => {
            postCount = null;
          });
      } else {
        postCount = null;
      }
    }
  });

  const stashName = $derived.by(() => {
    if (!target?.subscription) return 'Stash';
    const col = libraryState.collections.find(
      (c) => c.id === target.subscription?.destination_collection_id
    );
    return col ? libraryState.getStashDisplayName(col) : 'Inbox';
  });

  const dialogTitle = $derived.by(() => {
    if (isBatch) {
      return i18n.t('subscriptions.batch_unsubscribe_title', { count: target?.batch?.length ?? 0 });
    }
    return `${i18n.t('subscriptions.unsubscribe')} · ${target?.creatorName ?? ''}`;
  });

  const dialogDescription = $derived.by(() => {
    if (isBatch) {
      return i18n.t('subscriptions.batch_unsubscribe_confirm', { count: target?.batch?.length ?? 0 });
    }
    return i18n.t('subscriptions.unsubscribe_confirm', { creator: target?.creatorName ?? '' });
  });

  function handleClose() {
    if (loading) return;
    subscriptionState.closeUnsubscribePrompt();
  }

  async function handleConfirm() {
    if (!target) return;
    loading = true;
    try {
      if (isBatch && target.batch) {
        for (const sub of target.batch) {
          await subscriptionState.remove(sub.id, removePosts);
        }
        notify.success(
          i18n.t('selection.unsubscribe'),
          `${target.batch.length} ${target.batch.length === 1 ? 'creator' : 'creators'}`
        );
      } else if (target.subscription) {
        await subscriptionState.remove(target.subscription.id, removePosts);
        notify.success(i18n.t('subscriptions.removed'), {
          description: target.creatorName,
          glyph: 'removed'
        });
      }

      if (removePosts) {
        await libraryState.refreshCollections();
        await libraryState.refreshSavedKeys();
        if (libraryState.selectedCollectionId) {
          await libraryState.refresh();
        }
      }

      subscriptionState.closeUnsubscribePrompt();
    } catch (err) {
      notify.error(i18n.t('subscriptions.action_error'), err);
    } finally {
      loading = false;
    }
  }
</script>

{#if isOpen}
  <ConfirmDialog
    {isOpen}
    title={dialogTitle}
    description={dialogDescription}
    confirmLabel={i18n.t('subscriptions.unsubscribe')}
    confirmVariant="danger"
    confirmIcon={IconPersonDelete}
    heroIcon={IconWarning}
    {loading}
    onclose={handleClose}
    onconfirm={handleConfirm}
  >
    <div class="unsubscribe-options flex flex-col gap-3 pt-2">
      <label class="flex items-center gap-3 cursor-pointer select-none group p-2.5 rounded-xl bg-[var(--surface-sunken)] border border-[var(--border-subtle)] hover:border-[var(--border-color)] transition-colors">
        <Checkbox checked={removePosts} onchange={(v) => (removePosts = v)} />
        <span class="text-xs font-medium text-[var(--fg-default)] group-hover:text-[var(--fg-emphasized)] leading-relaxed">
          {#if isBatch}
            {i18n.t('subscriptions.batch_unsubscribe_remove_posts')}
          {:else if postCount !== null && postCount > 0}
            {i18n.t('subscriptions.unsubscribe_remove_posts', {
              count: postCount,
              stash: stashName
            })}
          {:else}
            {i18n.t('subscriptions.unsubscribe_remove_posts_generic', {
              stash: stashName
            })}
          {/if}
        </span>
      </label>
    </div>
  </ConfirmDialog>
{/if}
