import { t } from "substrate-platform-ui";

const UNITS = ["B", "KB", "MB", "GB"] as const;
const STEP = 1024;

/** `bytes` as Visual Studio Installer writes sizes: `59 MB`, `2.10 GB`. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= STEP && unit < UNITS.length - 1) {
    value /= STEP;
    unit += 1;
  }
  const digits = unit >= 3 ? 2 : 0;
  return `${value.toFixed(digits)} ${t(UNITS[unit])}`;
}
