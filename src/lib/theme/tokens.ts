import type { AccentPalette, ColorMode, ContrastLevel, SchemeVariant } from './palette';

export type RadiusScale = 'sharp' | 'smooth' | 'rounded' | 'pill';
export type SurfaceStyle = 'flat' | 'oled';
export type AccentPreset = '#D69085' | 'rose' | 'violet' | 'cyan' | 'emerald' | 'amber' | 'indigo';
export type AccentColor = 'system' | AccentPreset | (string & {});
export type MotionSpeed = 'instant' | 'snappy' | 'smooth';

export type { AccentPalette, ColorMode, ContrastLevel, SchemeVariant };

export interface ThemeTokens {
  fontFamily?: string;
  radiusScale: RadiusScale;
  surfaceStyle: SurfaceStyle;
  accent: AccentColor;
  colorMode: ColorMode;
  schemeVariant: SchemeVariant;
  contrastLevel: ContrastLevel;
  motionSpeed: MotionSpeed;
  backdropBlurPx: number;
  borderWidthPx: number;
  sidebarWidthPx?: number;
}

export const UI_SCALE_MIN = 80;
export const UI_SCALE_MAX = 150;
export const UI_SCALE_STEP = 5;

const BASE_TEXT_SIZES = { xs: 12, sm: 14, base: 16, lg: 18, xl: 20 };

export function textSizes(factor: number): Record<keyof typeof BASE_TEXT_SIZES, string> {
  const size = (px: number) => `${Math.round(px * factor * 10) / 10}px`;
  return {
    xs: size(BASE_TEXT_SIZES.xs),
    sm: size(BASE_TEXT_SIZES.sm),
    base: size(BASE_TEXT_SIZES.base),
    lg: size(BASE_TEXT_SIZES.lg),
    xl: size(BASE_TEXT_SIZES.xl)
  };
}

export const RADIUS_SCALE_MAP: Record<RadiusScale, { sm: string; md: string; lg: string; xl: string }> = {
  sharp: { sm: '2px', md: '4px', lg: '6px', xl: '8px' },
  smooth: { sm: '8px', md: '12px', lg: '16px', xl: '20px' },
  rounded: { sm: '10px', md: '16px', lg: '22px', xl: '28px' },
  pill: { sm: '9999px', md: '9999px', lg: '9999px', xl: '9999px' },
};

export const MOTION_SPEED_MAP: Record<MotionSpeed, { fast: string; normal: string; slow: string }> = {
  instant: { fast: '0ms', normal: '50ms', slow: '100ms' },
  snappy: { fast: '80ms', normal: '150ms', slow: '250ms' },
  smooth: { fast: '120ms', normal: '220ms', slow: '400ms' },
};
