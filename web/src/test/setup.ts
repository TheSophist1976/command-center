import '@testing-library/jest-dom/vitest';
import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';

// Recent Node versions ship their own experimental `localStorage` that shadows jsdom's and throws
// without a --localstorage-file. Use a plain in-memory Storage so tests behave the same everywhere.
class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() { return this.data.size; }
  clear() { this.data.clear(); }
  getItem(key: string) { return this.data.get(key) ?? null; }
  key(index: number) { return [...this.data.keys()][index] ?? null; }
  removeItem(key: string) { this.data.delete(key); }
  setItem(key: string, value: string) { this.data.set(key, String(value)); }
}
for (const name of ['localStorage', 'sessionStorage'] as const) {
  Object.defineProperty(window, name, { value: new MemoryStorage(), configurable: true });
  Object.defineProperty(globalThis, name, { value: window[name], configurable: true });
}

afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  vi.restoreAllMocks();
});

// jsdom has no layout engine, so these browser APIs are missing; stub the bits the app touches.
Element.prototype.scrollIntoView = () => {};

/** Pretend the viewport is `mobile` (≤720px) or desktop for every media query the app asks about. */
export function setViewport(mobile: boolean) {
  window.matchMedia = ((query: string) => ({
    matches: /max-width:\s*720px/.test(query) ? mobile : false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  })) as typeof window.matchMedia;
}
setViewport(false);
