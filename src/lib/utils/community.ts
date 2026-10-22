import type { Component } from 'svelte';
import { openExternalUrl } from '$lib/utils/ipc';
import { onboardingState } from '$lib/state/onboardingState.svelte';
import IconTelegram from '~icons/simple-icons/telegram';
import IconDiscord from '~icons/simple-icons/discord';
import IconReddit from '~icons/simple-icons/reddit';
import IconGithub from '~icons/simple-icons/github';

export interface CommunityLink {
  id: 'telegram' | 'discord' | 'reddit' | 'github';
  label: string;
  url: string;
  icon: Component;
}

export const COMMUNITY_LINKS: CommunityLink[] = [
  { id: 'telegram', label: 'Telegram', url: 'https://t.me/pawstashapp', icon: IconTelegram },
  { id: 'reddit', label: 'Reddit', url: 'https://reddit.com/r/pawstash', icon: IconReddit },
  { id: 'discord', label: 'Discord', url: 'https://discord.gg/ahcx8ub5Ck', icon: IconDiscord },
  { id: 'github', label: 'GitHub', url: 'https://github.com/pawstash', icon: IconGithub }
];

export const COMMUNITY_FEATURED = COMMUNITY_LINKS.slice(0, 2);

export const COMMUNITY_SETTINGS = COMMUNITY_LINKS.filter((link) => link.id !== 'discord');

export const TELEGRAM = COMMUNITY_LINKS[0];

export function openCommunity(link: CommunityLink) {
  onboardingState.markCommunityJoined();
  void openExternalUrl(link.url);
}
