import { useState } from 'react';
import { X } from 'lucide-react';
import { Button } from './Button';
import { sectionLabelStyle } from '../styles';
import type { Settings } from '../settings';
import { notificationPermission, requestNotificationPermission } from '../notifications';

interface SettingsDialogProps {
  settings: Settings;
  onSave: (settings: Settings) => void;
  onClose: () => void;
}

export function SettingsDialog({ settings, onSave, onClose }: SettingsDialogProps) {
  const [deferMinutes, setDeferMinutes] = useState(String(settings.deferMinutes));
  const [notifications, setNotifications] = useState(settings.notifications);
  const [permission, setPermission] = useState(notificationPermission);
  const minutes = Math.round(Number(deferMinutes));
  const valid = Number.isFinite(minutes) && minutes >= 1;

  function save() {
    if (!valid) return;
    onSave({ ...settings, deferMinutes: minutes, notifications });
    onClose();
  }

  async function toggleNotifications(on: boolean) {
    setNotifications(on);
    // Ask for permission here: browsers only allow the prompt from a click.
    if (on) setPermission(await requestNotificationPermission());
  }

  return (
    <div
      onClick={onClose}
      style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{ background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8, padding: 24, width: 420, maxWidth: '90vw', color: 'var(--fg-1)' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', marginBottom: 20 }}>
          <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 18 }}>Settings</span>
          <button onClick={onClose} aria-label="Close" style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 4 }}>
            <X size={18} color="var(--fg-4)" />
          </button>
        </div>
        <label style={{ display: 'block' }}>
          <div style={{ ...sectionLabelStyle, marginBottom: 8 }}>Defer duration (minutes)</div>
          <input
            type="number"
            min={1}
            value={deferMinutes}
            autoFocus
            onChange={(e) => setDeferMinutes(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') save(); }}
            style={{ width: '100%', boxSizing: 'border-box', height: 36, padding: '0 12px', background: 'var(--ink)', border: '1px solid var(--hairline)', borderRadius: 5, color: 'var(--fg-1)', fontSize: 14 }}
          />
          <div style={{ marginTop: 6, fontSize: 12, color: 'var(--fg-4)' }}>
            A deferred task in Next moves to the bottom of the list for this long, then returns to its normal place.
          </div>
        </label>
        <div style={{ marginTop: 20 }}>
          <div style={{ ...sectionLabelStyle, marginBottom: 8 }}>Notifications</div>
          <label style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 14, cursor: 'pointer' }}>
            <input type="checkbox" checked={notifications} onChange={(e) => toggleNotifications(e.target.checked)} />
            Notify me about new tasks, overdue tasks and tasks that need me
          </label>
          <div style={{ marginTop: 6, fontSize: 12, color: notifications && permission !== 'granted' ? 'var(--danger)' : 'var(--fg-4)' }}>
            {notificationHint(notifications, permission)}
          </div>
          {notifications && permission === 'default' && (
            <div style={{ marginTop: 8 }}>
              <Button size="sm" variant="secondary" onClick={async () => setPermission(await requestNotificationPermission())}>Allow notifications</Button>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 24 }}>
          <Button variant="secondary" onClick={onClose}>Cancel</Button>
          <Button onClick={save} disabled={!valid}>Save</Button>
        </div>
      </div>
    </div>
  );
}

function notificationHint(on: boolean, permission: NotificationPermission | 'unsupported'): string {
  if (!on) return 'Off. Turn on to get alerts on this device.';
  switch (permission) {
    case 'granted':
      return 'On for this device while Command Center is open or in the background.';
    case 'denied':
      return 'Blocked by the browser. Allow notifications for this site in the browser settings.';
    case 'unsupported':
      return 'This browser does not support notifications. On iPhone, add Command Center to the Home Screen and open it from there.';
    default:
      return 'The browser has not been allowed to show notifications yet.';
  }
}
