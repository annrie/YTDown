import { createI18n } from 'vue-i18n'
import ja from '../locales/ja.json'
import en from '../locales/en.json'
import zhTW from '../locales/zh-TW.json'
import fr from '../locales/fr.json'
import es from '../locales/es.json'
import ptBR from '../locales/pt-BR.json'
import de from '../locales/de.json'
import ko from '../locales/ko.json'

export type SupportedLocale = 'ja' | 'en' | 'zh-TW' | 'fr' | 'es' | 'pt-BR' | 'de' | 'ko'

export const SUPPORTED_LOCALES: { value: SupportedLocale; label: string }[] = [
  { value: 'ja', label: '日本語' },
  { value: 'en', label: 'English' },
  { value: 'zh-TW', label: '繁體中文' },
  { value: 'fr', label: 'Français' },
  { value: 'es', label: 'Español' },
  { value: 'pt-BR', label: 'Português (BR)' },
  { value: 'de', label: 'Deutsch' },
  { value: 'ko', label: '한국어' },
]

function detectLocale(): SupportedLocale {
  const saved = localStorage.getItem('ytdown_language') as SupportedLocale | null
  if (saved && SUPPORTED_LOCALES.some(l => l.value === saved)) return saved

  const browserLang = navigator.language
  if (browserLang.startsWith('ja')) return 'ja'
  if (browserLang.startsWith('zh')) return 'zh-TW'
  if (browserLang.startsWith('fr')) return 'fr'
  if (browserLang.startsWith('es')) return 'es'
  if (browserLang.startsWith('pt')) return 'pt-BR'
  if (browserLang.startsWith('de')) return 'de'
  if (browserLang.startsWith('ko')) return 'ko'
  return 'ja'
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'ja',
  messages: {
    ja,
    en,
    'zh-TW': zhTW,
    fr,
    es,
    'pt-BR': ptBR,
    de,
    ko,
  },
})

export function setLocale(locale: SupportedLocale) {
  i18n.global.locale.value = locale
  localStorage.setItem('ytdown_language', locale)
  document.documentElement.lang = locale
}
