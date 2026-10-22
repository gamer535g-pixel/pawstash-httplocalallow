import { i18n } from '$lib/i18n';
import { notify } from '$lib/utils/toast';

export async function copyText(text: string, done: string = i18n.t('common.copied')): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    notify.success(done, { glyph: 'copied' });
    return true;
  } catch (error) {
    notify.error(i18n.t('common.copy_failed'), error);
    return false;
  }
}
