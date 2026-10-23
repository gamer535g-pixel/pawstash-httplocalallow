<script lang="ts">
  interface Props {
    value?: number;
    indeterminate?: boolean;
    label?: string;
  }

  let { value = 0, indeterminate = false, label }: Props = $props();

  let percent = $derived(Math.min(100, Math.max(0, value)));
</script>

<div
  class="linear-progress"
  class:is-indeterminate={indeterminate}
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={indeterminate ? undefined : Math.round(percent)}
>
  <div class="linear-progress__bar" style:width={indeterminate ? undefined : `${percent}%`}></div>
</div>

<style>
  .linear-progress {
    position: relative;
    height: 4px;
    overflow: hidden;
    border-radius: var(--radius-full);
    background: var(--accent-container);
  }

  .linear-progress__bar {
    height: 100%;
    border-radius: inherit;
    background: var(--accent-primary);
    transition: width var(--duration-fast) var(--ease-standard);
  }

  .is-indeterminate .linear-progress__bar {
    position: absolute;
    width: 40%;
    animation: linear-progress-indeterminate 1.2s var(--ease-standard) infinite;
  }

  @keyframes linear-progress-indeterminate {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .is-indeterminate .linear-progress__bar {
      left: 0;
      width: 100%;
      animation: none;
      opacity: 0.6;
    }
  }
</style>
