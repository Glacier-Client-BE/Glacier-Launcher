/**
 * Reactive convenience layer over `./i18n.ts`'s `t(key, lang)` /
 * `LANGUAGES`, for components that don't already receive `settings` as a
 * prop (most of the panel views only need a translated header title, not
 * the whole settings object). `./i18n.ts` itself stays a plain, rune-free
 * module — a straight port of the original `LocalizationService.T`, with
 * `lang` passed explicitly by every existing call site — so this file adds
 * the reactivity on top rather than changing that module's shape.
 *
 * `App.svelte` calls `setLanguage(settings.Language)` in an `$effect`,
 * writing this module's `$state`; any component that calls `t(key)`
 * directly in a template expression re-renders when that changes, the
 * same "shared reactive module" pattern Svelte's docs use for cross-
 * component state that isn't tied to one component instance.
 */

import { t as tBase, LANGUAGES } from './i18n'

let lang = $state('en')

export function getLanguage(): string {
  return lang
}

/** Falls back to English for an unrecognized code (e.g. a language that
 * existed in an older build's settings file and was since removed) rather
 * than propagating a bad value into every `t()` lookup. */
export function setLanguage(code: string | undefined) {
  lang = code && LANGUAGES.some((l) => l.code === code) ? code : 'en'
}

/** Convenience over `i18n.ts`'s `t(key, lang, params)`, reading the current
 * language from this module's reactive state instead of requiring every
 * caller to thread `settings.Language` through as a prop. `params` fills
 * `{name}`-style placeholders, e.g. `t('toast.created', { name: pack.name })`. */
export function t(key: string, params?: Record<string, string | number>): string {
  return tBase(key, lang, params)
}

export { LANGUAGES }
