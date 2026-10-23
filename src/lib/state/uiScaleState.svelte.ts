import { apiRefreshUiScale, apiSetUiScale, type UiScale } from '$lib/utils/ipc';
import { UI_SCALE_MAX, UI_SCALE_MIN, UI_SCALE_STEP } from '$lib/theme/tokens';
import { ctrlWheel } from '$lib/utils/ctrlWheel';
import { logger } from '$lib/utils/logger';
import { layoutState } from './layoutState.svelte';
import { scaleIndicator, wheelStepper } from './scaleState.svelte';

const KEY_STEP = 10;

let viewportFactor = 1;

function applyViewportScale(factor = viewportFactor) {
  viewportFactor = factor;
  const viewport = window.visualViewport;
  const deviceWidth = viewport ? viewport.width * viewport.scale : window.innerWidth;
  const width = Math.floor(deviceWidth / factor);
  document
    .querySelector('meta[name="viewport"]')
    ?.setAttribute(
      'content',
      `width=${width}, initial-scale=${factor}, minimum-scale=${factor}, maximum-scale=${factor}, user-scalable=no, viewport-fit=cover`
    );
}

class UiScaleState {
  percent = $state(0);
  systemPercent = $state(100);
  private wheelStep = wheelStepper();
  private queue: Promise<unknown> = Promise.resolve();

  get effective(): number {
    return this.percent || this.systemPercent;
  }

  async init() {
    try {
      this.percent = (await this.enqueue(apiRefreshUiScale)).percent;
    } catch (error) {
      logger.debug('Interface scale unavailable', error);
    }
    window.addEventListener('focus', () => void this.enqueue(apiRefreshUiScale).catch(() => {}));
    window.addEventListener('keydown', this.handleKeydown);
    if (layoutState.isAndroid) window.addEventListener('resize', () => applyViewportScale());
    ctrlWheel(this.handleWheel)(document.documentElement);
  }

  set(percent: number, announce = false) {
    const clamped = Math.min(UI_SCALE_MAX, Math.max(UI_SCALE_MIN, Math.round(percent)));
    const next = percent === 0 || clamped === this.systemPercent ? 0 : clamped;
    if (next !== this.percent) {
      this.percent = next;
      this.enqueue(() => apiSetUiScale(next)).catch((error) => logger.warn('Could not change the interface size', error));
    }
    if (announce) scaleIndicator.show('interface', this.effective);
  }

  private enqueue(call: () => Promise<UiScale>): Promise<UiScale> {
    const result = this.queue.then(call);
    this.queue = result.then(
      (scale) => {
        this.systemPercent = Math.round(scale.system * 100);
        if (layoutState.isAndroid) applyViewportScale(scale.percent ? scale.percent / 100 : scale.system);
      },
      () => {}
    );
    return result;
  }

  private handleKeydown = (event: KeyboardEvent) => {
    if (!(layoutState.isMacOS ? event.metaKey : event.ctrlKey) || event.altKey) return;
    const direction =
      event.key === '+' || event.key === '=' ? 1 : event.key === '-' || event.key === '_' ? -1 : event.key === '0' ? 0 : null;
    if (direction === null) return;
    setTimeout(() => {
      if (event.defaultPrevented) return;
      this.set(direction === 0 ? 0 : this.effective + direction * KEY_STEP, true);
    });
  };

  private handleWheel = (event: WheelEvent) => {
    if (!event.ctrlKey || event.defaultPrevented) return;
    event.preventDefault();
    const direction = this.wheelStep(event);
    if (direction) this.set(this.effective + direction * UI_SCALE_STEP, true);
  };
}

export const uiScaleState = new UiScaleState();
