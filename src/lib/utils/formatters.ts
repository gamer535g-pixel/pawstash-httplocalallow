import { i18n } from '$lib/i18n';
import { extraField } from './fields';

export function formatBytes(bytes: number, decimals: number = 2): string {
  return i18n.bytes(bytes, Math.max(0, decimals));
}

export function formatSpeed(bytesPerSecond: number): string {
  return i18n.t('common.per_second', { value: formatBytes(bytesPerSecond) });
}

export function formatPercent(percent: number): string {
  return i18n.number(percent / 100, { style: 'percent', maximumFractionDigits: 1 });
}

export function parseDateTimestamp(dateValue?: string | number | null): number {
  if (!dateValue) return 0;
  const num = typeof dateValue === 'number' ? dateValue : Number(dateValue);
  if (!isNaN(num) && num > 0) {
    return num < 10_000_000_000 ? num * 1000 : num;
  }
  const t = new Date(String(dateValue)).getTime();
  return isNaN(t) ? 0 : t;
}

export function formatDate(dateValue?: string | number | null): string {
  if (!dateValue) return '';
  try {
    const num = typeof dateValue === 'number' ? dateValue : Number(dateValue);
    const date = !isNaN(num) && num > 0
      ? new Date(num < 10_000_000_000 ? num * 1000 : num)
      : new Date(String(dateValue));
    if (isNaN(date.getTime())) return String(dateValue);
    return i18n.date(date, { year: 'numeric', month: 'short', day: 'numeric' });
  } catch {
    return String(dateValue);
  }
}

export function cleanPostTitle(title?: string | null): string {
  if (!title) return '';
  let text = String(title);

  text = text
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&apos;/g, "'")
    .replace(/&nbsp;/g, ' ');

  text = text.replace(/<[^>]*>/g, ' ');

  text = text
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/(\*{1,2}|_{1,2})(.*?)\1/g, '$2')
    .replace(/~~(.*?)~~/g, '$1')
    .replace(/`([^`]+)`/g, '$1');

  return text.replace(/\s+/g, ' ').trim();
}

export function parseTags(tagsValue?: any): string[] {
  if (!tagsValue) return [];
  if (Array.isArray(tagsValue)) {
    return tagsValue.map((t) => String(t).replace(/^#+/, '').trim()).filter(Boolean);
  }
  if (typeof tagsValue === 'string') {
    const trimmed = tagsValue.trim();
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      return trimmed
        .slice(1, -1)
        .split(',')
        .map((t) => t.trim().replace(/^"(.*)"$/, '$1').replace(/^#+/, '').trim())
        .filter(Boolean);
    }
    if (trimmed.includes(',')) {
      return trimmed.split(',').map((t) => t.trim().replace(/^#+/, '').trim()).filter(Boolean);
    }
    if (trimmed.includes('#')) {
      return trimmed.split(/\s+/).map((t) => t.replace(/^#+/, '').trim()).filter(Boolean);
    }
    if (trimmed) return [trimmed.replace(/^#+/, '').trim()].filter(Boolean);
  }
  return [];
}

export function getPostTags(post?: any): string[] {
  if (!post) return [];
  const direct = parseTags(extraField(post, 'tags') || extraField(post, 'categories'));
  const tagSet = new Set<string>(direct);

  const extractHashtags = (text?: string | null) => {
    if (!text || typeof text !== 'string') return;
    const matches = text.match(/#([\p{L}\p{N}_-]+)/gu);
    if (matches) {
      for (const m of matches) {
        const clean = m.slice(1).trim();
        if (clean.length >= 2 && !/^\d+$/.test(clean)) {
          tagSet.add(clean);
        }
      }
    }
  };

  extractHashtags(post.title);
  if (post.content && typeof post.content === 'string') {
    extractHashtags(post.content.slice(0, 2000));
  }

  return Array.from(tagSet);
}
