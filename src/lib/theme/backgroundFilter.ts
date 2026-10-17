export interface BackgroundFilter {
  blurPx: number;
  brightness: number;
  saturation: number;
}

export interface LayerSize {
  width: number;
  height: number;
}

export const isIdentityFilter = (filter: BackgroundFilter) =>
  filter.blurPx <= 0 && filter.brightness === 1 && filter.saturation === 1;

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = 'anonymous';
    image.decoding = 'async';
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error(`Background image failed to load: ${url}`));
    image.src = url;
  });
}

function createCanvas(width: number, height: number) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) throw new Error('2D canvas context is unavailable');
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  return { canvas, ctx };
}

function drawScaled(image: HTMLImageElement, width: number, height: number) {
  let source: CanvasImageSource = image;
  let w = image.naturalWidth;
  let h = image.naturalHeight;
  while (w / 2 > width && h / 2 > height) {
    w = Math.round(w / 2);
    h = Math.round(h / 2);
    const step = createCanvas(w, h);
    step.ctx.drawImage(source, 0, 0, w, h);
    source = step.canvas;
  }
  const out = createCanvas(width, height);
  out.ctx.drawImage(source, 0, 0, width, height);
  return out;
}

/** Three box blurs approximating a gaussian (Kovesi, "Fast Almost-Gaussian Filtering"). */
function boxRadiiForGaussian(sigma: number): number[] {
  const n = 3;
  const ideal = Math.sqrt((12 * sigma * sigma) / n + 1);
  let lower = Math.floor(ideal);
  if (lower % 2 === 0) lower--;
  const upper = lower + 2;
  const lowerCount = Math.round((12 * sigma * sigma - n * lower * lower - 4 * n * lower - 3 * n) / (-4 * lower - 4));
  return Array.from({ length: n }, (_, i) => ((i < lowerCount ? lower : upper) - 1) / 2);
}

function boxBlurPass(src: Uint8ClampedArray, dst: Uint8ClampedArray, width: number, height: number, radius: number, horizontal: boolean) {
  const length = horizontal ? width : height;
  const lines = horizontal ? height : width;
  const step = horizontal ? 4 : width * 4;
  const lineStep = horizontal ? width * 4 : 4;
  const size = radius * 2 + 1;
  const last = length - 1;
  for (let line = 0; line < lines; line++) {
    const base = line * lineStep;
    for (let channel = 0; channel < 4; channel++) {
      const at = (i: number) => src[base + Math.min(Math.max(i, 0), last) * step + channel];
      let sum = 0;
      for (let i = -radius; i <= radius; i++) sum += at(i);
      for (let i = 0; i < length; i++) {
        dst[base + i * step + channel] = sum / size;
        sum += at(i + radius + 1) - at(i - radius);
      }
    }
  }
}

function gaussianBlur(data: Uint8ClampedArray, width: number, height: number, sigma: number) {
  const scratch = new Uint8ClampedArray(data.length);
  for (const radius of boxRadiiForGaussian(sigma)) {
    if (radius < 1) continue;
    boxBlurPass(data, scratch, width, height, radius, true);
    boxBlurPass(scratch, data, width, height, radius, false);
  }
}

function adjustColor(data: Uint8ClampedArray, brightness: number, saturation: number) {
  if (brightness === 1 && saturation === 1) return;
  const s = saturation;
  const m = [
    0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s,
    0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s,
    0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s
  ];
  for (let i = 0; i < data.length; i += 4) {
    const r = Math.min(255, data[i] * brightness);
    const g = Math.min(255, data[i + 1] * brightness);
    const b = Math.min(255, data[i + 2] * brightness);
    data[i] = m[0] * r + m[1] * g + m[2] * b;
    data[i + 1] = m[3] * r + m[4] * g + m[5] * b;
    data[i + 2] = m[6] * r + m[7] * g + m[8] * b;
  }
}

/** WebKit lacks a reliable `CanvasRenderingContext2D.filter`, so the filter is applied by hand. */
export async function renderFilteredBackground(url: string, filter: BackgroundFilter, layer: LayerSize, dpr: number): Promise<Blob> {
  const image = await loadImage(url);
  const { naturalWidth, naturalHeight } = image;
  if (!naturalWidth || !naturalHeight) throw new Error(`Background image has no size: ${url}`);

  const cover = Math.max(layer.width / naturalWidth, layer.height / naturalHeight);
  const density = filter.blurPx > 0 ? Math.min(dpr, 4 / filter.blurPx) : dpr;
  const width = Math.max(1, Math.min(naturalWidth, Math.round(naturalWidth * cover * density)));
  const height = Math.max(1, Math.min(naturalHeight, Math.round(naturalHeight * cover * density)));

  const { canvas, ctx } = drawScaled(image, width, height);
  const pixels = ctx.getImageData(0, 0, width, height);
  if (filter.blurPx > 0) gaussianBlur(pixels.data, width, height, filter.blurPx * (width / (naturalWidth * cover)));
  adjustColor(pixels.data, filter.brightness, filter.saturation);
  ctx.putImageData(pixels, 0, 0);

  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('Background canvas could not be encoded'))), 'image/png');
  });
}
