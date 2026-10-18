<script lang="ts">
  import type { CloudLockKind } from '$lib/types/cloud';
  import { i18n } from '$lib/i18n';
  import { notify } from '$lib/utils/toast';
  import { errorCode } from '$lib/utils/cloudLink';
  import Button from '$lib/components/ui/Button.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import IconLock from '~icons/fluent/lock-closed-24-regular';

  interface Props {
    kind: CloudLockKind;
    title: string;
    hints?: string[];
    autofocus?: boolean;
    unlock: (secret: string) => Promise<unknown>;
  }

  let { kind, title, hints = [], autofocus = false, unlock }: Props = $props();

  const titleId = $props.id();
  const errorId = `${titleId}-error`;

  let value = $state('');
  let busy = $state(false);
  let error = $state('');

  function wrongSecretMessage(code: string): string {
    if (code === 'wrong_password') return i18n.t('cloud.wrong_password');
    if (code === 'wrong_key') return i18n.t('cloud.wrong_key');
    return '';
  }

  async function submit(secret: string) {
    const candidate = secret.trim();
    if (!candidate || busy) return;
    busy = true;
    error = '';
    try {
      await unlock(candidate);
      value = '';
    } catch (err) {
      error = wrongSecretMessage(errorCode(err));
      if (!error) {
        notify.error(i18n.t(kind === 'password' ? 'cloud.unlock_failed' : 'cloud.key_check_failed'), err);
      }
    } finally {
      busy = false;
    }
  }

  function useHint(hint: string) {
    value = hint;
    void submit(hint);
  }
</script>

<form
  class="unlock"
  onsubmit={(event) => {
    event.preventDefault();
    void submit(value);
  }}
>
  <p class="unlock-title" id={titleId}><IconLock aria-hidden="true" />{title}</p>
  <Input
    type={kind === 'password' ? 'password' : 'text'}
    class={kind === 'key' ? 'is-key' : ''}
    autocomplete="off"
    {autofocus}
    labelledby={titleId}
    describedby={error ? errorId : undefined}
    bind:value
    placeholder={i18n.t(kind === 'password' ? 'cloud.password_placeholder' : 'cloud.key_placeholder')}
    disabled={busy}
    oninput={() => (error = '')}
  />
  {#if error}
    <p class="unlock-error" id={errorId} role="alert">{error}</p>
  {/if}
  {#if hints.length}
    <div class="unlock-hints">
      <span class="unlock-hints-label">{i18n.t('cloud.password_hints')}</span>
      {#each hints as hint (hint)}
        <Button variant="tonal" size="sm" class="unlock-hint" disabled={busy} onclick={() => useHint(hint)}>
          {hint}
        </Button>
      {/each}
    </div>
  {/if}
  <div class="dialog-actions">
    <Button type="submit" variant="ghost" data-tone="accent" disabled={busy || !value.trim()}>
      {i18n.t('cloud.unlock')}
    </Button>
  </div>
</form>

<style>
  .unlock {
    --unlock-gap: calc(var(--space-2) * var(--ui-scale, 1));
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-3) * var(--ui-scale, 1));
    min-width: 0;
  }

  .unlock-title {
    display: flex;
    align-items: center;
    gap: var(--unlock-gap);
    margin: 0;
    font-size: calc(var(--text-sm) * var(--ui-scale, 1));
    font-weight: var(--font-weight-semibold);
  }

  .unlock-title :global(svg) {
    flex-shrink: 0;
    width: calc(var(--control-icon-base) * var(--ui-scale, 1));
    height: calc(var(--control-icon-base) * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .unlock :global(.is-key input) {
    font-family: var(--font-mono);
  }

  .unlock-error {
    margin: 0;
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    color: var(--status-error);
  }

  .unlock-hints {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--unlock-gap);
  }

  .unlock-hints-label {
    font-size: calc(var(--text-xs) * var(--ui-scale, 1));
    color: var(--text-secondary);
  }

  .unlock-hints :global(.unlock-hint) {
    max-width: 100%;
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
  }

  .unlock .dialog-actions {
    grid-template-columns: 1fr;
  }
</style>
