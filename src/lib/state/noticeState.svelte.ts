import { invoke } from '@tauri-apps/api/core';
import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';
import { logger } from '$lib/utils/logger';
import { openExternalUrl } from '$lib/utils/ipc';

type Localized = Record<string, string>;
type Severity = 'info' | 'warning' | 'critical';

interface RawNotice {
  id: string;
  severity: string;
  title: Localized;
  body: Localized;
  actions: { label: Localized; url: string }[];
}

export interface Notice {
  id: string;
  severity: Severity;
  title: string;
  body: string;
  actions: { label: string; url: string }[];
}

const SEEN_KEY = 'pawstash_seen_notices';
const DEV_SOURCE_KEY = 'pawstash_notices_url';
// A notice can only link where the project lives, so a leaked repository cannot become a phishing channel.
const LINK_HOSTS = ['t.me', 'telegram.me', 'reddit.com', 'github.com', 'nichind.dev'];
const RANK: Record<Severity, number> = { critical: 2, warning: 1, info: 0 };

function linkAllowed(url: string) {
  try {
    const parsed = new URL(url);
    return (
      parsed.protocol === 'https:' &&
      LINK_HOSTS.some((host) => parsed.hostname === host || parsed.hostname.endsWith(`.${host}`))
    );
  } catch {
    return false;
  }
}

function localize(text: Localized) {
  return (text[i18n.currentLocale] ?? text.en ?? Object.values(text)[0] ?? '').trim();
}

function loadSeen(): Set<string> {
  try {
    return new Set(JSON.parse(localStorage.getItem(SEEN_KEY) || '[]'));
  } catch {
    return new Set();
  }
}

class NoticeState {
  current = $state<Notice | null>(null);
  private waiting: (() => void)[] = [];

  whenClosed(): Promise<void> {
    if (!this.current) return Promise.resolve();
    return new Promise((resolve) => this.waiting.push(resolve));
  }

  async check() {
    let raw: RawNotice[];
    try {
      const source = import.meta.env.DEV ? localStorage.getItem(DEV_SOURCE_KEY) : null;
      raw = await invoke<RawNotice[]>('fetch_app_notices', { source });
    } catch (error) {
      logger.warn('Developer notices unavailable', error);
      return;
    }
    const seen = loadSeen();
    const next = raw
      .filter((notice) => !seen.has(notice.id))
      .map(
        (notice): Notice => ({
          id: notice.id,
          severity: (['warning', 'critical'].includes(notice.severity) ? notice.severity : 'info') as Severity,
          title: localize(notice.title),
          body: localize(notice.body),
          actions: notice.actions
            .filter((action) => linkAllowed(action.url))
            .slice(0, 2)
            .map((action) => ({ label: localize(action.label), url: action.url }))
            .filter((action) => action.label)
        })
      )
      .filter((notice) => notice.title)
      .sort((a, b) => RANK[b.severity] - RANK[a.severity])[0];
    if (!next) return;

    if (next.severity === 'info') {
      this.markSeen(next.id);
      const action = next.actions[0];
      notify.info(next.title, next.body || undefined, {
        duration: 20000,
        action: action ? { label: action.label, onclick: () => void openExternalUrl(action.url) } : undefined
      });
    } else {
      this.current = next;
    }
  }

  close() {
    if (this.current) this.markSeen(this.current.id);
    this.current = null;
    for (const resolve of this.waiting.splice(0)) resolve();
  }

  private markSeen(id: string) {
    const seen = loadSeen();
    seen.add(id);
    try {
      localStorage.setItem(SEEN_KEY, JSON.stringify([...seen].slice(-200)));
    } catch {
    }
  }
}

export const noticeState = new NoticeState();
