import type { Component } from 'svelte';
import type { CloudNode } from '$lib/types/cloud';
import { isAudioUrl, isCompressedUrl, isDocumentUrl, isImageUrl, isVideoUrl } from '$lib/utils/media';
import IconFolder from '~icons/fluent/folder-24-filled';
import IconZip from '~icons/fluent/folder-zip-24-regular';
import IconImage from '~icons/fluent/image-24-regular';
import IconVideo from '~icons/fluent/video-clip-24-regular';
import IconMusic from '~icons/fluent/music-note-2-24-regular';
import IconDocument from '~icons/fluent/document-24-regular';
import IconDocumentText from '~icons/fluent/document-text-24-regular';

export type NodeKind = 'folder' | 'image' | 'video' | 'audio' | 'archive' | 'document' | 'file';

export const NODE_KIND_ICONS: Record<NodeKind, Component> = {
  folder: IconFolder,
  image: IconImage,
  video: IconVideo,
  audio: IconMusic,
  archive: IconZip,
  document: IconDocumentText,
  file: IconDocument
};

export function nodeKindOf(node: CloudNode): NodeKind {
  if (node.is_folder) return 'folder';
  const mime = (node.mime_type || '').toLowerCase();
  if (mime.startsWith('image/') || isImageUrl(node.name)) return 'image';
  if (mime.startsWith('video/') || isVideoUrl(node.name)) return 'video';
  if (mime.startsWith('audio/') || isAudioUrl(node.name)) return 'audio';
  if (isCompressedUrl(node.name) || /zip|rar|7z|compressed/.test(mime)) return 'archive';
  if (isDocumentUrl(node.name) || mime === 'application/pdf') return 'document';
  return 'file';
}

export function cloudNodePath(byId: Map<string, CloudNode>, node: CloudNode): string {
  const parts = [node.name];
  const seen = new Set([node.id]);
  let parent = node.parent_id ?? null;
  while (parent && !seen.has(parent)) {
    seen.add(parent);
    const ancestor = byId.get(parent);
    if (!ancestor) break;
    parts.unshift(ancestor.name);
    parent = ancestor.parent_id ?? null;
  }
  return parts.join('/');
}

export function archiveFolderName(name: string): string {
  return name.replace(/\.(zip|cbz|rar|cbr|7z|cb7)(\.\d+)?$/i, '') || name;
}
