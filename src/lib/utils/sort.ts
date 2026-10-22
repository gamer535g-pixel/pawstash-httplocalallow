import type { Component } from 'svelte';
import { i18n } from '$lib/i18n';
import IconDefault from '~icons/fluent/arrow-sort-24-regular';
import IconDescending from '~icons/fluent/arrow-sort-down-lines-24-regular';
import IconAscending from '~icons/fluent/arrow-sort-up-lines-24-regular';
import IconTextAscending from '~icons/fluent/text-sort-ascending-24-regular';
import IconTextDescending from '~icons/fluent/text-sort-descending-24-regular';

type SortField =
  | 'popularity'
  | 'updated'
  | 'indexed'
  | 'published'
  | 'favorited'
  | 'added'
  | 'name'
  | 'title'
  | 'size';

export interface SortOption<T> {
  value: T;
  label: string;
  icon: Component;
  ariaLabel?: string;
}

const TEXT_FIELDS = new Set<SortField>(['name', 'title']);

export function defaultSortOption<T>(value: T): SortOption<T> {
  return { value, label: i18n.t('sort.default'), icon: IconDefault };
}

export function sortOption<T>(value: T, field: SortField, direction: 'asc' | 'desc'): SortOption<T> {
  const label = i18n.t(`sort.${field}`);
  const ascending = direction === 'asc';
  const icon = TEXT_FIELDS.has(field)
    ? ascending ? IconTextAscending : IconTextDescending
    : ascending ? IconAscending : IconDescending;
  return { value, label, icon, ariaLabel: `${label}, ${i18n.t(ascending ? 'sort.ascending' : 'sort.descending')}` };
}
