import { configState } from './configState.svelte';
import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';

type ScaleKind = 'grid' | 'interface';

export const GRID_SCALE_MIN = 60;
export const GRID_SCALE_MAX = 160;
export const GRID_SCALE_STEP = 5;
const WHEEL_STEP_DELTA = 50;
const INDICATOR_MS = 1200;
const SAVE_DELAY_MS = 300;

class ScaleIndicator {
  kind = $state<ScaleKind | null>(null);
  percent = $state(0);
  private hideTimer: ReturnType<typeof setTimeout> | undefined;

  show(kind: ScaleKind, percent: number) {
    this.kind = kind;
    this.percent = percent;
    clearTimeout(this.hideTimer);
    this.hideTimer = setTimeout(() => (this.kind = null), INDICATOR_MS);
  }
}

export const scaleIndicator = new ScaleIndicator();

export function wheelStepper(): (event: WheelEvent) => -1 | 0 | 1 {
  let accumulated = 0;
  return (event) => {
    accumulated += event.deltaY;
    if (Math.abs(accumulated) < WHEEL_STEP_DELTA) return 0;
    const direction = accumulated < 0 ? 1 : -1;
    accumulated = 0;
    return direction;
  };
}

let gridSaveTimer: ReturnType<typeof setTimeout> | undefined;

function setGridScale(next: number) {
  const value = Math.max(GRID_SCALE_MIN, Math.min(GRID_SCALE_MAX, Math.round(next / GRID_SCALE_STEP) * GRID_SCALE_STEP));
  configState.settings.grid_scale = value;
  scaleIndicator.show('grid', value);
  clearTimeout(gridSaveTimer);
  gridSaveTimer = setTimeout(
    () => configState.save(configState.settings).catch((error) => notify.error(i18n.t('settings.save_failed'), error)),
    SAVE_DELAY_MS
  );
}

const gridStep = wheelStepper();

export function gridScaleWheel(event: WheelEvent) {
  if (!event.ctrlKey) return;
  event.preventDefault();
  const direction = gridStep(event);
  if (direction) setGridScale(configState.settings.grid_scale + direction * GRID_SCALE_STEP);
}
