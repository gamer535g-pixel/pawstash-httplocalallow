<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { libraryState } from '$lib/state/libraryState.svelte';
  import { syncState } from '$lib/state/syncState.svelte';
  import { onboardingState } from '$lib/state/onboardingState.svelte';
  import { COMMUNITY_FEATURED, openCommunity } from '$lib/utils/community';
  import Button from '$lib/components/ui/Button.svelte';
  import IconCheck from '~icons/fluent/checkmark-16-filled';
  import IconDismiss from '~icons/fluent/dismiss-24-regular';

  let steps = $derived([
    { id: 'save', done: libraryState.savedCount > 0 },
    { id: 'sync', done: syncState.status.configured },
    { id: 'community', done: onboardingState.communityJoined }
  ]);
  let doneCount = $derived(steps.filter((step) => step.done).length);
  let visible = $derived(
    libraryState.initialized &&
      syncState.statusLoaded &&
      !onboardingState.checklistDismissed &&
      doneCount < steps.length
  );
</script>

{#if visible}
  <section class="getting-started" aria-label={i18n.t('onboarding.title')}>
    <header class="gs-header">
      <div class="gs-heading">
        <h2 class="gs-title">{i18n.t('onboarding.title')}</h2>
        <span class="gs-progress">{i18n.t('onboarding.progress', { done: doneCount, total: steps.length })}</span>
      </div>
      <Button variant="ghost" class="btn-icon" aria-label={i18n.t('onboarding.dismiss')} onclick={() => onboardingState.dismissChecklist()}>
        <IconDismiss />
      </Button>
    </header>

    <ol class="gs-steps">
      {#each steps as step (step.id)}
        <li class="gs-step" class:is-done={step.done}>
          <span class="gs-mark" aria-hidden="true">{#if step.done}<IconCheck />{/if}</span>
          <div class="gs-text">
            <span class="gs-step-title">{i18n.t(`onboarding.${step.id}_title`)}</span>
            {#if !step.done}
              <span class="gs-step-desc">{i18n.t(`onboarding.${step.id}_desc`)}</span>
            {/if}
          </div>
          {#if !step.done && step.id === 'sync'}
            <Button variant="tonal" size="sm" onclick={() => onboardingState.openSyncSetup()}>
              {i18n.t('onboarding.sync_action')}
            </Button>
          {:else if !step.done && step.id === 'community'}
            <div class="gs-chats">
              {#each COMMUNITY_FEATURED as link (link.id)}
                <Button variant="tonal" size="sm" class="btn-icon" aria-label={link.label} onclick={() => openCommunity(link)}>
                  <link.icon />
                </Button>
              {/each}
            </div>
          {/if}
        </li>
      {/each}
    </ol>
  </section>
{/if}

<style>
  .getting-started {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-block-end: 16px;
    padding: 12px 12px 8px 20px;
    border-radius: var(--radius-xl);
    background: var(--surface-container);
  }

  .gs-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .gs-heading {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
  }

  .gs-title {
    margin: 0;
    color: var(--text-primary);
    font-size: var(--type-title-md-size);
    line-height: var(--type-title-md-line);
    font-weight: 500;
  }

  .gs-progress {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
    font-variant-numeric: tabular-nums;
  }

  .gs-steps {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .gs-step {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 56px;
    padding: 8px 8px 8px 0;
  }

  .gs-mark {
    display: grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 2px solid var(--outline-variant);
    border-radius: var(--radius-full);
    box-sizing: border-box;
  }

  .is-done .gs-mark {
    border-color: var(--accent-primary);
    background: var(--accent-primary);
    color: var(--accent-on-primary);
  }

  .gs-mark :global(svg) {
    width: 14px;
    height: 14px;
  }

  .gs-text {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .gs-step-title {
    color: var(--text-primary);
    font-size: var(--type-body-lg-size);
    line-height: var(--type-body-lg-line);
  }

  .is-done .gs-step-title {
    color: var(--text-secondary);
    text-decoration: line-through;
    text-decoration-color: var(--outline-variant);
  }

  .gs-step-desc {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
    line-height: var(--type-body-sm-line);
  }

  .gs-chats {
    display: flex;
    flex: none;
    gap: 6px;
  }

  .gs-chats :global(svg) {
    width: 18px;
    height: 18px;
  }
</style>
