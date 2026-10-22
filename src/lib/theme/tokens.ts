import type { AccentPalette, ColorMode, ContrastLevel, SchemeVariant } from './palette';

export type FontSizeScale = 'compact' | 'standard' | 'large';
export type RadiusScale = 'sharp' | 'smooth' | 'rounded' | 'pill';
export type SurfaceStyle = 'flat' | 'oled';
export type AccentPreset = '#D69085' | 'rose' | 'violet' | 'cyan' | 'emerald' | 'amber' | 'indigo';
export type AccentColor = 'system' | AccentPreset | (string & {});
export type MotionSpeed = 'instant' | 'snappy' | 'smooth';

export type { AccentPalette, ColorMode, ContrastLevel, SchemeVariant };

export interface ThemeTokens {
  fontScale: FontSizeScale;
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

export const FONT_SCALE_MAP: Record<FontSizeScale, { xs: string; sm: string; base: string; lg: string; xl: string }> = {
  compact: { xs: '11px', sm: '13px', base: '14px', lg: '16px', xl: '18px' },
  standard: { xs: '12px', sm: '14px', base: '16px', lg: '18px', xl: '20px' },
  large: { xs: '13px', sm: '15px', base: '17px', lg: '19px', xl: '22px' },
};

export const FONT_SCALE_FACTOR: Record<FontSizeScale, number> = {
  compact: 0.93,
  standard: 1,
  large: 1.07
};

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
