<script lang="ts" module>
  export type { MenuItem } from '$lib/components/ui/MenuList.svelte';
</script>

<script lang="ts">
  import { i18n } from '$lib/i18n';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import MenuList, { type MenuItem } from '$lib/components/ui/MenuList.svelte';
  import IconMoreVertical from '~icons/fluent/more-vertical-24-filled';

  interface Props {
    items: MenuItem[];
  }

  let { items }: Props = $props();

  let open = $state(false);
</script>

{#if items.length > 0}
  <PopoverMenu bind:open title={i18n.t('common.more')} icon={IconMoreVertical} width="auto" menuClass="overflow-menu">
    <div class="overflow-list">
      <MenuList {items} onrun={() => (open = false)} />
    </div>
  </PopoverMenu>
{/if}

<style>
  .overflow-list {
    min-width: 220px;
    max-width: min(340px, calc(100vw - 32px));
  }
</style>
