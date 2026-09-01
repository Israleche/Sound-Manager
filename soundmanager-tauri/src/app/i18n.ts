/**
 * Minimal i18n: loads locale entries from the Rust core (bundled resources)
 * and applies them to [data-i18n] nodes. Falls back to English.
 */
import { ipc } from "./ipc";

export type Lang = "eng" | "fra";

let entries: Record<string, string> = {};
let current: Lang = "eng";

export function t(key: string): string {
  return entries[key] ?? key;
}

export async function loadLanguage(lang: Lang): Promise<void> {
  try {
    const loc = await ipc.getLocale(lang);
    entries = loc.entries;
    current = lang;
    document.documentElement.lang = lang === "fra" ? "fr" : "en";
    apply();
  } catch (e) {
    console.warn("locale load failed", lang, e);
  }
}

export function apply(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n ?? "";
    const value = t(key);
    if (value && value !== key) el.textContent = value;
  });
  document.querySelectorAll<HTMLSelectElement>("#set-lang").forEach((sel) => {
    sel.value = current;
  });
}

export function currentLanguage(): Lang {
  return current;
}
