<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ripple, tooltip } from '$lib/motion';
  import { parseColorToRgb, ACCENT_PRESETS, getContrastInk } from '$lib/theme/palette';
  import { themeState } from '$lib/theme/themeState.svelte';
  import { i18n } from '$lib/i18n';
  import PopoverMenu from '$lib/components/ui/PopoverMenu.svelte';
  import { copyText } from '$lib/utils/clipboard';
  import IconChevronDown from '~icons/fluent/chevron-down-20-regular';
  import IconCheck from '~icons/fluent/checkmark-24-regular';
  import IconCopy from '~icons/fluent/copy-24-regular';
  import IconEyedropper from '~icons/fluent/eyedropper-24-regular';

  interface Props {
    value?: string;
    label?: string;
    align?: 'left' | 'right';
    size?: 'sm' | 'base' | 'md' | 'lg';
    showPalette?: boolean;
    disabled?: boolean;
    class?: string;
    onchange?: (color: string) => void;
    trigger?: Snippet<[{ toggle: (e: MouseEvent) => void; open: boolean; currentHex: string }]>;
  }

  let {
    value = '#000000',
    label,
    align = 'right',
    size,
    showPalette = true,
    disabled = false,
    class: extraClass = '',
    onchange,
    trigger: customTrigger
  }: Props = $props();

  const M3_TONES = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95];

  const resolvedLabel = $derived(label ?? i18n.t('color_picker.label'));

  const accentPresets = $derived(
    ACCENT_PRESETS.map((preset) => ({
      hex: preset.hex,
      name: i18n.has(`color_picker.accent_${preset.id}`) ? i18n.t(`color_picker.accent_${preset.id}`) : preset.id
    }))
  );

  const activeMaterialRoles = $derived([
    { name: i18n.t('color_picker.role_primary'), hex: themeState.palette.primary },
    { name: i18n.t('color_picker.role_container'), hex: themeState.palette.quadrants[1] },
    { name: i18n.t('color_picker.role_deep'), hex: themeState.palette.quadrants[2] },
    { name: i18n.t('color_picker.role_subtle'), hex: themeState.palette.quadrants[3] }
  ]);

  let hsl = $state({ h: 0, s: 0, l: 0 });
  let hexDisplay = $state('#000000');
  let lastPropValue = $state('');
  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout> | undefined;

  function hslToHex(h: number, s: number, l: number): string {
    l /= 100;
    const a = (s * Math.min(l, 1 - l)) / 100;
    const f = (n: number) => {
      const k = (n + h / 30) % 12;
      const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
      return Math.round(255 * color).toString(16).padStart(2, '0');
    };
    return `#${f(0)}${f(8)}${f(4)}`.toUpperCase();
  }

  function hexToHsl(hex: string): { h: number; s: number; l: number } {
    const rgb = parseColorToRgb(hex);
    const r = rgb.r / 255;
    const g = rgb.g / 255;
    const b = rgb.b / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    let h = hsl?.h ?? 0;
    let s = 0;
    const l = (max + min) / 2;

    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r:
          h = (g - b) / d + (g < b ? 6 : 0);
          break;
        case g:
          h = (b - r) / d + 2;
          break;
        case b:
          h = (r - g) / d + 4;
          break;
      }
      h /= 6;
      h = Math.round(h * 360);
    }

    return {
      h,
      s: Math.round(s * 100),
      l: Math.round(l * 100)
    };
  }

  $effect(() => {
    if (value && value.toUpperCase() !== lastPropValue.toUpperCase()) {
      lastPropValue = value.toUpperCase();
      hexDisplay = value.toUpperCase();
      const parsed = hexToHsl(value);
      if (parsed.s === 0 || parsed.l === 0 || parsed.l === 100) {
        hsl = { h: hsl.h, s: parsed.s, l: parsed.l };
      } else {
        hsl = parsed;
      }
    }
  });

  function applyColor(newHex: string) {
    const formatted = newHex.toUpperCase();
    hexDisplay = formatted;
    lastPropValue = formatted;
    const parsed = hexToHsl(formatted);
    if (parsed.s > 0 && parsed.l > 0 && parsed.l < 100) {
      hsl = parsed;
    } else {
      hsl = { h: hsl.h, s: parsed.s, l: parsed.l };
    }
    onchange?.(formatted);
  }

  function updateHsl(newH: number, newS: number, newL: number) {
    if (newL === 0 && (newH !== hsl.h || (newS !== hsl.s && newS > 0))) {
      newL = 50;
      if (newS === 0) newS = 85;
    }
    if (newS === 0 && newH !== hsl.h) {
      newS = 85;
      if (newL === 0) newL = 50;
    }
    hsl = { h: newH, s: newS, l: newL };
    const hex = hslToHex(hsl.h, hsl.s, hsl.l);
    hexDisplay = hex;
    lastPropValue = hex;
    onchange?.(hex);
  }

  function setTone(tone: number) {
    updateHsl(hsl.h, hsl.s, tone);
  }

  function handleHexInput(e: Event) {
    const raw = (e.target as HTMLInputElement).value.trim();
    const clean = raw.startsWith('#') ? raw : `#${raw}`;
    hexDisplay = clean.toUpperCase();
    if (/^#[0-9A-Fa-f]{6}$/.test(clean)) {
      applyColor(clean);
    }
  }

  async function copyHex() {
    if (!(await copyText(hexDisplay, i18n.t('color_picker.copied_hex', { hex: hexDisplay })))) return;
    isCopied = true;
    clearTimeout(copyTimeout);
    copyTimeout = setTimeout(() => (isCopied = false), 1600);
  }

  function handleHexBlur() {
    if (!/^#[0-9A-Fa-f]{6}$/.test(hexDisplay)) {
      hexDisplay = currentHex;
    }
  }

  const currentHex = $derived(hslToHex(hsl.h, hsl.s, hsl.l));
</script>

<PopoverMenu
  {align}
  width="calc(320px * var(--ui-scale, 1))"
  menuClass="color-picker-popover"
  class="color-picker-root {customTrigger ? 'has-custom-trigger' : ''} {extraClass}"
>
  {#snippet trigger({ toggle, open })}
    {#if customTrigger}
      {@render customTrigger({ toggle, open, currentHex })}
    {:else}
      <button
        type="button"
        onclick={toggle}
        use:ripple
        {disabled}
        class="color-picker-trigger {size ? `size-${size}` : ''}"
        class:is-open={open}
        aria-label={`${resolvedLabel}: ${currentHex}`}
        aria-haspopup="dialog"
        aria-expanded={open}
      >
        <span class="trigger-swatch" style:background-color={currentHex}></span>
        <span class="trigger-label">{currentHex}</span>
        <IconChevronDown class="trigger-chevron" />
      </button>
    {/if}
  {/snippet}

  {#snippet children()}
    <div class="cp" role="group" aria-label={resolvedLabel}>
      <div class="cp-head">
        <span class="cp-swatch" style:background-color={currentHex} aria-hidden="true"></span>
        <div class="cp-hex-field">
          <input
            type="text"
            class="cp-hex-input"
            value={hexDisplay}
            oninput={handleHexInput}
            onblur={handleHexBlur}
            onkeydown={(e) => {
              if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
            }}
            maxlength={7}
            spellcheck="false"
            autocomplete="off"
            aria-label={i18n.t('color_picker.hex_code')}
          />
          <button
            type="button"
            class="field-action"
            class:is-active={isCopied}
            use:ripple
            use:tooltip={i18n.t('color_picker.copy_hex')}
            aria-label={i18n.t('color_picker.copy_hex')}
            onclick={copyHex}
          >
            {#if isCopied}<IconCheck />{:else}<IconCopy />{/if}
          </button>
          <label class="field-action cp-native" use:tooltip={i18n.t('color_picker.system_picker')}>
            <IconEyedropper />
            <input
              type="color"
              value={currentHex}
              oninput={(e) => applyColor(e.currentTarget.value)}
              onchange={(e) => applyColor(e.currentTarget.value)}
              aria-label={i18n.t('color_picker.system_picker')}
            />
          </label>
        </div>
      </div>

      <section class="cp-section">
        <span class="cp-label">{i18n.t('color_picker.tonal')}</span>
        <div class="cp-tones" role="radiogroup" aria-label={i18n.t('color_picker.tonal')}>
          {#each M3_TONES as tone (tone)}
            {@const toneHex = hslToHex(hsl.h, hsl.s, tone)}
            {@const active = Math.abs(hsl.l - tone) <= 4}
            <button
              type="button"
              class="cp-tone ink-{getContrastInk(toneHex)}"
              role="radio"
              aria-checked={active}
              style:background-color={toneHex}
              onclick={() => setTone(tone)}
              aria-label={i18n.t('color_picker.tone_value', { tone })}
            >
              {#if active}<IconCheck />{/if}
            </button>
          {/each}
        </div>
      </section>

      <section class="cp-section cp-sliders">
        {#each [
          { key: 'hue', label: i18n.t('color_picker.hue'), min: 0, max: 360, value: hsl.h, unit: '°', track: 'linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)', set: (v: number) => updateHsl(v, hsl.s, hsl.l) },
          { key: 'tone', label: i18n.t('color_picker.tone'), min: 0, max: 100, value: hsl.l, unit: '%', track: `linear-gradient(to right, #000, ${hslToHex(hsl.h, hsl.s, 50)}, #fff)`, set: (v: number) => updateHsl(hsl.h, hsl.s, v) },
          { key: 'chroma', label: i18n.t('color_picker.chroma'), min: 0, max: 100, value: hsl.s, unit: '%', track: `linear-gradient(to right, ${hslToHex(hsl.h, 0, hsl.l)}, ${hslToHex(hsl.h, 100, hsl.l)})`, set: (v: number) => updateHsl(hsl.h, v, hsl.l) }
        ] as slider (slider.key)}
          <label class="cp-slider">
            <span class="cp-slider-label">{slider.label}</span>
            <input
              type="range"
              min={slider.min}
              max={slider.max}
              step="1"
              value={slider.value}
              oninput={(e) => slider.set(Number(e.currentTarget.value))}
              style:--cp-track={slider.track}
              style:--cp-thumb={currentHex}
            />
            <span class="cp-slider-value">{slider.value}{slider.unit}</span>
          </label>
        {/each}
      </section>

      {#if showPalette}
        <section class="cp-section">
          <span class="cp-label">{i18n.t('color_picker.palette')}</span>
          <div class="cp-swatches" role="radiogroup" aria-label={i18n.t('color_picker.palette')}>
            {#each [...activeMaterialRoles, ...accentPresets] as swatch (swatch.name)}
              <button
                type="button"
                class="color-swatch"
                role="radio"
                aria-checked={currentHex.toUpperCase() === swatch.hex.toUpperCase()}
                style:background-color={swatch.hex}
                onclick={() => applyColor(swatch.hex)}
                use:tooltip={swatch.name}
                aria-label={swatch.name}
              ></button>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/snippet}
</PopoverMenu>

<style>
  :global(.color-picker-root) {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
    max-width: fit-content;
  }

  :global(.color-picker-root:not(.has-custom-trigger)) {
    margin-inline-start: auto;
  }

  .color-picker-trigger {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: calc(128px * var(--ui-scale, 1));
    height: calc(var(--control-height, 40px) * var(--ui-scale, 1));
    padding: 0 10px 0 8px;
    border: 0;
    border-radius: calc(var(--radius-md) * var(--ui-scale, 1));
    background: var(--input-bg);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-body-md-size);
    outline: none;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .color-picker-trigger.size-sm { --control-height: var(--control-height-sm); }
  .color-picker-trigger.size-base { --control-height: var(--control-height-base); }
  .color-picker-trigger.size-md { --control-height: var(--control-height-md); }
  .color-picker-trigger.size-lg { --control-height: var(--control-height-lg); }

  @media (hover: hover) {
    .color-picker-trigger:hover {
      background: var(--input-bg-hover);
    }
  }

  .color-picker-trigger.is-open {
    background: var(--input-bg-hover);
  }

  .color-picker-trigger:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .color-picker-trigger:disabled {
    opacity: var(--opacity-disabled);
    pointer-events: none;
  }

  .trigger-swatch {
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-full);
  }

  .trigger-label {
    flex: 1 1 auto;
    min-width: 0;
    text-align: start;
  }

  .color-picker-trigger :global(.trigger-chevron) {
    flex: none;
    width: 18px;
    height: 18px;
    color: var(--text-secondary);
    transition: transform var(--duration-fast) var(--ease-standard);
  }

  .color-picker-trigger.is-open :global(.trigger-chevron) {
    transform: rotate(180deg);
  }

  .cp {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px;
  }

  .cp-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cp-swatch {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
  }

  .cp-hex-field {
    --control-icon-size: 20px;
    display: flex;
    flex: 1 1 auto;
    align-items: center;
    gap: 4px;
    min-width: 0;
    height: 40px;
    padding: 0 8px 0 12px;
    border-radius: var(--radius-md);
    background: var(--input-bg);
  }

  .cp-hex-field:has(.cp-hex-input:focus-visible) {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .cp-hex-input {
    flex: 1 1 auto;
    min-width: 0;
    padding: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--type-body-lg-size);
    text-transform: uppercase;
  }

  /* The field draws the focus ring; the global input ring would draw a second one inside it. */
  .cp-hex-input:focus-visible {
    outline: none !important;
  }

  .cp-native {
    position: relative;
    cursor: default;
  }

  .cp-native input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: default;
  }

  .cp-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .cp-label {
    color: var(--text-secondary);
    font-size: var(--type-title-sm-size);
    line-height: var(--type-title-sm-line);
    font-weight: 500;
  }

  .cp-tones {
    display: flex;
    height: 32px;
    overflow: hidden;
    border-radius: var(--radius-full);
  }

  .cp-tone {
    display: grid;
    flex: 1 1 0;
    place-items: center;
    min-width: 0;
    padding: 0;
    border: 0;
  }

  .cp-tone :global(svg) {
    width: 16px;
    height: 16px;
  }

  .cp-tone.ink-light {
    color: #fff;
  }

  .cp-tone.ink-dark {
    color: #000;
  }

  .cp-tone:focus-visible {
    outline: 2px solid var(--accent-primary);
    outline-offset: -2px;
  }

  .cp-sliders {
    gap: 4px;
  }

  .cp-slider {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) 40px;
    align-items: center;
    gap: 12px;
    min-height: 32px;
  }

  .cp-slider-label {
    color: var(--text-secondary);
    font-size: var(--type-body-sm-size);
  }

  .cp-slider-value {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--type-body-sm-size);
    font-variant-numeric: tabular-nums;
    text-align: end;
  }

  .cp-slider input {
    width: 100%;
    height: 12px;
    margin: 0;
    border-radius: var(--radius-full);
    background: var(--cp-track);
    outline: none;
    appearance: none;
    -webkit-appearance: none;
  }

  .cp-slider input::-webkit-slider-thumb {
    width: 20px;
    height: 20px;
    border-radius: var(--radius-full);
    background: var(--cp-thumb);
    box-shadow: 0 0 0 3px #fff, 0 1px 4px 3px rgba(0, 0, 0, 0.35);
    appearance: none;
    -webkit-appearance: none;
  }

  .cp-slider input:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px #fff, 0 0 0 5px var(--accent-primary);
  }

  .cp-swatches {
    --swatch-size: 28px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
