import { createI18n } from 'vue-i18n';
import zhSim from './zh-sim';
import zhTra from './zh-tra';
import en from './en';
import eo from './eo';

const LOCALE_KEY = 'guanfu_locale';

function getStoredLocale(): string {
  const stored = localStorage.getItem(LOCALE_KEY);
  if (stored && ['zh-sim', 'zh-tra', 'en', 'eo'].includes(stored)) {
    return stored;
  }
  // Detect from browser/system
  const lang = navigator.language || (navigator as any).userLanguage || 'zh-sim';
  if (lang.startsWith('zh-TW') || lang.startsWith('zh-HK') || lang.startsWith('zh-Hant')) {
    return 'zh-tra';
  }
  if (lang.startsWith('en')) {
    return 'en';
  }
  if (lang.startsWith('eo')) {
    return 'eo';
  }
  return 'zh-sim';
}

const i18n = createI18n({
  legacy: false,
  locale: getStoredLocale(),
  fallbackLocale: 'zh-sim',
  messages: {
    'zh-sim': zhSim,
    'zh-tra': zhTra,
    'en': en,
    'eo': eo,
  },
});

export function setLocale(locale: string) {
  (i18n.global.locale as unknown as { value: string }).value = locale;
  localStorage.setItem(LOCALE_KEY, locale);
  document.documentElement.setAttribute('lang', locale);
}

export function getLocale(): string {
  return i18n.global.locale.value;
}

export const availableLocales = [
  { code: 'zh-sim', name: '简体中文' },
  { code: 'zh-tra', name: '繁體中文' },
  { code: 'en', name: 'English' },
  { code: 'eo', name: 'Esperanto' },
];

export default i18n;
