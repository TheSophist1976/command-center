const SETTINGS_KEY = 'ccSettings';
const DEFERRALS_KEY = 'ccDeferrals';

export interface Settings {
  /** How long a deferred Next task sinks to the bottom of the list. */
  deferMinutes: number;
}

export const DEFAULT_SETTINGS: Settings = { deferMinutes: 60 };

export function loadSettings(): Settings {
  try {
    const parsed = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? '{}');
    const minutes = Number(parsed.deferMinutes);
    return { deferMinutes: Number.isFinite(minutes) && minutes > 0 ? minutes : DEFAULT_SETTINGS.deferMinutes };
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: Settings): void {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

/** Task id → epoch ms until which the task is deferred. */
export type Deferrals = Record<number, number>;

/** Drops entries that have already expired. */
export function loadDeferrals(now: number): Deferrals {
  try {
    const parsed: Deferrals = JSON.parse(localStorage.getItem(DEFERRALS_KEY) ?? '{}');
    return Object.fromEntries(Object.entries(parsed).filter(([, until]) => until > now)) as Deferrals;
  } catch {
    return {};
  }
}

export function saveDeferrals(deferrals: Deferrals): void {
  localStorage.setItem(DEFERRALS_KEY, JSON.stringify(deferrals));
}
