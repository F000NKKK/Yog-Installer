import { useState } from "react";

/** The Installer's own namespace in the webview's storage. */
const PREFIX = "yog-installer.";

/** Settings kept across runs in the webview's storage (`key` under the Installer's prefix); fields added since fall back to `initial`. */
export function usePersisted<T extends object>(key: string, initial: T): [T, (value: T) => void] {
  const [value, setValue] = useState<T>(() => {
    const stored = localStorage.getItem(PREFIX + key);
    if (stored === null) return initial;
    try {
      return { ...initial, ...JSON.parse(stored) } as T;
    } catch {
      return initial;
    }
  });
  const set = (next: T) => {
    setValue(next);
    localStorage.setItem(PREFIX + key, JSON.stringify(next));
  };
  return [value, set];
}
