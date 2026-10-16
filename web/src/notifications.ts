import type { Task } from './types';
import { isOverdue } from './dueWindow';
import { needsHumanAction } from './filters';

const SNAPSHOT_KEY = 'ccNotifySnapshot';

/** What the last check saw, so the next one only notifies about changes. Persisted so a reload does not re-notify. */
export interface NotifySnapshot {
  /** Every task id seen. */
  ids: number[];
  /** Ids of open tasks that needed the human. */
  human: number[];
  /** `id:due_date` of tasks already reported overdue, so a new due date can be reported again. */
  overdue: string[];
}

export interface TaskNotification {
  taskId: number;
  title: string;
  body: string;
}

function overdueKey(t: Task): string {
  return `${t.id}:${t.due_date}`;
}

export function snapshotOf(tasks: Task[], today: Date): NotifySnapshot {
  return {
    ids: tasks.map((t) => t.id),
    human: tasks.filter(needsHumanAction).map((t) => t.id),
    overdue: tasks.filter((t) => isOverdue(t, today)).map(overdueKey),
  };
}

/**
 * Compares the tasks against the previous snapshot and returns at most one
 * notification per task: new task, then newly needs the human, then newly overdue.
 */
export function diffNotifications(prev: NotifySnapshot, tasks: Task[], today: Date): TaskNotification[] {
  const ids = new Set(prev.ids);
  const human = new Set(prev.human);
  const overdue = new Set(prev.overdue);
  const out: TaskNotification[] = [];
  for (const t of tasks) {
    if (t.status !== 'open') continue;
    const needsYou = needsHumanAction(t);
    if (!ids.has(t.id)) {
      out.push({ taskId: t.id, title: needsYou ? 'New task for you' : 'New task', body: `#${t.id} ${t.title}` });
    } else if (needsYou && !human.has(t.id)) {
      out.push({ taskId: t.id, title: humanReason(t), body: `#${t.id} ${t.title}` });
    } else if (isOverdue(t, today) && !overdue.has(overdueKey(t))) {
      out.push({ taskId: t.id, title: 'Task overdue', body: `#${t.id} ${t.title} (due ${t.due_date})` });
    }
  }
  return out;
}

function humanReason(t: Task): string {
  if (t.work_status === 'needs-input') return 'Agent has a question';
  if (t.work_status === 'waiting-for-review') return 'Ready for your review';
  return 'Task assigned to you';
}

export function loadSnapshot(): NotifySnapshot | null {
  try {
    const parsed = JSON.parse(localStorage.getItem(SNAPSHOT_KEY) ?? 'null');
    return parsed && Array.isArray(parsed.ids) && Array.isArray(parsed.human) && Array.isArray(parsed.overdue) ? parsed : null;
  } catch {
    return null;
  }
}

export function saveSnapshot(snapshot: NotifySnapshot): void {
  localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(snapshot));
}

export function notificationsSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window;
}

export function notificationPermission(): NotificationPermission | 'unsupported' {
  return notificationsSupported() ? Notification.permission : 'unsupported';
}

/** Must be called from a user gesture (Safari ignores it otherwise). */
export async function requestNotificationPermission(): Promise<NotificationPermission | 'unsupported'> {
  if (!notificationsSupported()) return 'unsupported';
  if (Notification.permission !== 'default') return Notification.permission;
  return Notification.requestPermission();
}

export function registerServiceWorker(): void {
  if ('serviceWorker' in navigator) navigator.serviceWorker.register('/sw.js').catch(() => {});
}

/** Shows a notification through the service worker when there is one (required on iOS), else directly. */
export async function showNotification(n: TaskNotification): Promise<void> {
  if (notificationPermission() !== 'granted') return;
  const options: NotificationOptions = { body: n.body, tag: `task-${n.taskId}`, icon: '/apple-touch-icon.png' };
  const registration = 'serviceWorker' in navigator ? await navigator.serviceWorker.getRegistration() : undefined;
  if (registration) {
    await registration.showNotification(n.title, options);
    return;
  }
  new Notification(n.title, options);
}
