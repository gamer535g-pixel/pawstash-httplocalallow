import type { Attachment } from '$lib/types/content';
import type { MenuItem } from '$lib/components/ui/MenuList.svelte';
import { i18n } from '$lib/i18n';
import { cloudProviderName } from '$lib/utils/media';
import IconCloud from '~icons/fluent/cloud-24-regular';

const STORAGE_KEY = 'pawstash_mirror_sources';

function readChoices(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
  } catch {
    return {};
  }
}

export function mirrorsOf(file: Attachment): Attachment[] {
  return (file.mirrors as Attachment[] | undefined) ?? [];
}

export class MirrorSources {
  choices = $state<Record<string, string>>(readChoices());
  failed = $state<Record<string, string[]>>({});
  #scope: () => string;

  constructor(scope: () => string) {
    this.#scope = scope;
  }

  key(file: Attachment): string {
    return `${this.#scope()}|${String(file.name ?? '').trim().toLowerCase()}|${file.size ?? ''}`;
  }

  pick(file: Attachment): Attachment {
    const sources = [file, ...mirrorsOf(file)].map((source): Attachment => ({ ...source, mirrors: undefined }));
    const key = this.key(file);
    const failed = this.failed[key] ?? [];
    const usable = sources.filter((source) => !failed.includes(String(source.cloud_provider)));
    const chosen = usable.find((source) => source.cloud_provider === this.choices[key]) ?? usable[0] ?? sources[0];
    return { ...chosen, mirrors: sources.filter((source) => source !== chosen) };
  }

  choose(file: Attachment, provider: string) {
    const key = this.key(file);
    this.choices = { ...this.choices, [key]: provider };
    this.failed = { ...this.failed, [key]: (this.failed[key] ?? []).filter((name) => name !== provider) };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(this.choices));
  }

  fail(file: Attachment): boolean {
    const key = this.key(file);
    const failed = [...(this.failed[key] ?? []), String(file.cloud_provider)];
    if (!mirrorsOf(file).some((mirror) => !failed.includes(String(mirror.cloud_provider)))) return false;
    this.failed = { ...this.failed, [key]: failed };
    return true;
  }

  menu(file: Attachment): MenuItem[] {
    const failed = this.failed[this.key(file)] ?? [];
    return [file, ...mirrorsOf(file)].map((source) => {
      const provider = String(source.cloud_provider);
      return {
        label: cloudProviderName(provider) ?? provider,
        icon: IconCloud,
        active: source === file,
        detail: failed.includes(provider) ? i18n.t('post.source_failed') : undefined,
        onclick: () => this.choose(file, provider)
      };
    });
  }
}
