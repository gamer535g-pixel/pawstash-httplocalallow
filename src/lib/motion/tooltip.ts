import { computePosition, autoUpdate, flip, shift, offset } from '@floating-ui/dom';
import { FLOATING_EDGE } from '$lib/utils/floating';

export type TooltipPlacement = 'top' | 'bottom' | 'left' | 'right';

export type TooltipOptions = {
  text?: string;
  placement?: TooltipPlacement;
  delay?: number;
  open?: boolean;
  variant?: 'default' | 'accent';
};

export type TooltipParam = string | undefined | null | TooltipOptions;

const GAP = 6;
const HIDE_MS = 120;
const WARM_MS = 600;

const FALLBACKS: Record<TooltipPlacement, TooltipPlacement[]> = {
  bottom: ['top', 'right', 'left'],
  top: ['bottom', 'right', 'left'],
  left: ['right', 'bottom', 'top'],
  right: ['left', 'bottom', 'top']
};

let lastHiddenAt = 0;

const canHover = () => typeof window !== 'undefined' && window.matchMedia('(hover: hover)').matches;

function parseParam(param: TooltipParam, defaultDelay: number) {
  const options: TooltipOptions = typeof param === 'string' ? { text: param } : (param ?? {});
  return {
    text: options.text?.trim() || undefined,
    placement: options.placement ?? ('bottom' as TooltipPlacement),
    delay: options.delay ?? defaultDelay,
    open: options.open,
    variant: options.variant ?? ('default' as const)
  };
}

export function tooltip(node: HTMLElement, param: TooltipParam, defaultDelay = 400) {
  let el: HTMLElement | null = null;
  let showTimer: ReturnType<typeof setTimeout> | undefined;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let stopAutoUpdate: (() => void) | undefined;
  let config = parseParam(param, defaultDelay);

  async function place() {
    if (!el) return;
    const { x, y, placement } = await computePosition(node, el, {
      placement: config.placement,
      strategy: 'fixed',
      middleware: [
        offset(GAP),
        flip({ fallbackPlacements: FALLBACKS[config.placement], padding: FLOATING_EDGE }),
        shift({ padding: FLOATING_EDGE })
      ]
    });
    if (!el) return;
    el.dataset.side = placement.split('-')[0];
    el.style.left = `${Math.round(x)}px`;
    el.style.top = `${Math.round(y)}px`;
  }

  function render() {
    if (!el) {
      el = document.createElement('div');
      el.className = 'app-tooltip';
      el.setAttribute('role', 'tooltip');
      document.body.appendChild(el);
    }
    el.textContent = config.text ?? '';
    el.classList.toggle('is-accent', config.variant === 'accent');
  }

  function show() {
    if (!config.text) return;
    clearTimeout(showTimer);
    const warm = performance.now() - lastHiddenAt < WARM_MS;
    showTimer = setTimeout(async () => {
      if (!config.text) return;
      clearTimeout(hideTimer);
      hideTimer = undefined;
      render();
      await place();
      if (!el) return;
      stopAutoUpdate?.();
      stopAutoUpdate = autoUpdate(node, el, () => void place());
      requestAnimationFrame(() => {
        if (!hideTimer) el?.classList.add('is-visible');
      });
    }, warm || config.open ? 0 : config.delay);
  }

  function hide() {
    clearTimeout(showTimer);
    showTimer = undefined;
    stopAutoUpdate?.();
    stopAutoUpdate = undefined;
    if (!el) return;
    const leaving = el;
    if (leaving.classList.contains('is-visible')) lastHiddenAt = performance.now();
    leaving.classList.remove('is-visible');
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      leaving.remove();
      if (el === leaving) el = null;
      hideTimer = undefined;
    }, HIDE_MS);
  }

  // Touch has no hover: a tap would flash the tooltip. Only explicitly opened ones (slider values) show there.
  const onEnter = () => {
    if (config.open === undefined && canHover()) show();
  };
  const onFocus = () => {
    if (config.open === undefined && node.matches(':focus-visible')) show();
  };
  const onLeave = () => {
    if (config.open === undefined) hide();
  };

  node.addEventListener('mouseenter', onEnter);
  node.addEventListener('mouseleave', onLeave);
  node.addEventListener('focusin', onFocus);
  node.addEventListener('focusout', onLeave);
  node.addEventListener('click', onLeave);
  node.addEventListener('pointerdown', onLeave);

  if (config.open) show();

  return {
    update(next: TooltipParam) {
      config = parseParam(next, defaultDelay);
      if (!config.text) {
        hide();
        return;
      }
      if (el) {
        render();
        void place();
      }
      if (config.open === true && (!el || hideTimer)) show();
      else if (config.open === false) hide();
    },
    destroy() {
      clearTimeout(showTimer);
      clearTimeout(hideTimer);
      stopAutoUpdate?.();
      el?.remove();
      el = null;
      node.removeEventListener('mouseenter', onEnter);
      node.removeEventListener('mouseleave', onLeave);
      node.removeEventListener('focusin', onFocus);
      node.removeEventListener('focusout', onLeave);
      node.removeEventListener('click', onLeave);
      node.removeEventListener('pointerdown', onLeave);
    }
  };
}
