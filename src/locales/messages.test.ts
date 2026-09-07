import { describe, it, expect } from 'vitest'
import { createI18n } from 'vue-i18n'
import ja from './ja.json'
import en from './en.json'
import zhTW from './zh-TW.json'
import fr from './fr.json'
import es from './es.json'
import ptBR from './pt-BR.json'
import de from './de.json'
import ko from './ko.json'

// Every message must survive vue-i18n's message compiler. Characters such as `@`, `{`, `}`,
// `|` and `$` are syntax there; a stray one used to throw at render time and silently kept a
// whole dialog from opening. Guard all locales, not just the one the developer runs.
// Inferred from the JSON imports, exactly like src/i18n/index.ts passes them to createI18n.
const locales = {
  ja, en, 'zh-TW': zhTW, fr, es, 'pt-BR': ptBR, de, ko,
}

function flattenKeys(obj: Record<string, unknown>, prefix = ''): string[] {
  return Object.entries(obj).flatMap(([key, value]) => {
    const path = prefix ? `${prefix}.${key}` : key
    return value !== null && typeof value === 'object'
      ? flattenKeys(value as Record<string, unknown>, path)
      : [path]
  })
}

describe('locale messages compile', () => {
  for (const [locale, messages] of Object.entries(locales)) {
    it(`${locale}: every message compiles and renders`, () => {
      const i18n = createI18n({
        legacy: false,
        locale,
        messages: { [locale]: messages },
        missingWarn: false,
        fallbackWarn: false,
        warnHtmlMessage: false,
      })
      const failures: string[] = []
      for (const key of flattenKeys(messages)) {
        try {
          const rendered = i18n.global.t(key)
          if (typeof rendered !== 'string') failures.push(`${key}: not a string`)
        } catch (e) {
          failures.push(`${key}: ${String(e).split('\n')[0]}`)
        }
      }
      expect(failures).toEqual([])
    })
  }
})
