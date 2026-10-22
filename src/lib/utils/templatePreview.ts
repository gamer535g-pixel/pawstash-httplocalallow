export type TemplateKind = 'creator' | 'post' | 'file';

const DATE_TOKENS: Record<string, string> = {
  date: '2024-08-20',
  published: '2024-08-20',
  date_compact: '20240820',
  date_dots: '2024.08.20',
  year: '2024',
  yyyy: '2024',
  year_short: '24',
  yy: '24',
  month: '08',
  mm: '08',
  day: '20',
  dd: '20',
  year_month: '2024-08'
};

// Mirrors the per-segment tokens the backend expands; `name` and `id` mean different things per segment.
const SAMPLE_TOKENS: Record<TemplateKind, Record<string, string>> = {
  creator: {
    ...DATE_TOKENS,
    creator: 'AuthorName',
    author: 'AuthorName',
    name: 'AuthorName',
    service: 'Platform',
    platform: 'Platform',
    creator_id: '12345',
    id: '12345'
  },
  post: {
    ...DATE_TOKENS,
    post_title: 'PostTitle',
    title: 'PostTitle',
    post_id: '67890',
    id: '67890',
    creator: 'AuthorName',
    author: 'AuthorName',
    name: 'AuthorName',
    service: 'Platform',
    platform: 'Platform'
  },
  file: {
    ...DATE_TOKENS,
    post_title: 'PostTitle',
    title: 'PostTitle',
    post_id: '67890',
    creator: 'AuthorName',
    author: 'AuthorName',
    service: 'Platform',
    platform: 'Platform',
    filename: 'OriginalFilename.png',
    original_name: 'OriginalFilename.png',
    name: 'OriginalFilename',
    ext: 'png',
    index: '1',
    media_id: 'MediaID'
  }
};

const EMPTY_TEMPLATE: Record<TemplateKind, string> = {
  creator: '{creator}',
  post: '{post_title}',
  file: '{post_title} - {filename}'
};

export function templatePreview(template: string, kind: TemplateKind): string {
  const source = (template || '').trim() || EMPTY_TEMPLATE[kind];
  const tokens = SAMPLE_TOKENS[kind];
  const result = source.replace(/\{([a-z_]+)\}/g, (match, token: string) => tokens[token] ?? match);
  if (kind !== 'file' || result.toLowerCase().endsWith('.png')) return result;
  return `${result.replace(/\.png/gi, '')}.png`;
}

interface DownloadPathSettings {
  download_dir: string;
  download_group_by_creator: boolean;
  download_creator_folder_template: string;
  download_group_by_post: boolean;
  download_post_folder_template: string;
  download_filename_template: string;
}

export function downloadPathPreview(settings: DownloadPathSettings): string {
  const parts = [settings.download_dir.replace(/\\/g, '/').replace(/\/+$/, '') || 'Downloads/Pawstash'];
  if (settings.download_group_by_creator) parts.push(templatePreview(settings.download_creator_folder_template, 'creator'));
  if (settings.download_group_by_post) parts.push(templatePreview(settings.download_post_folder_template, 'post'));
  parts.push(templatePreview(settings.download_filename_template, 'file'));
  return parts.join('/');
}
