import { useState } from 'react';
import { X } from 'lucide-react';
import { Button } from './Button';
import { sectionLabelStyle } from '../styles';
import type { Settings } from '../settings';

interface SettingsDialogProps {
  settings: Settings;
  onSave: (settings: Settings) => void;
  onClose: () => void;
}

export function SettingsDialog({ settings, onSave, onClose }: SettingsDialogProps) {
  const [deferMinutes, setDeferMinutes] = useState(String(settings.deferMinutes));
  const minutes = Math.round(Number(deferMinutes));
  const valid = Number.isFinite(minutes) && minutes >= 1;

  function save() {
    if (!valid) return;
    onSave({ ...settings, deferMinutes: minutes });
    onClose();
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
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 24 }}>
          <Button variant="secondary" onClick={onClose}>Cancel</Button>
          <Button onClick={save} disabled={!valid}>Save</Button>
        </div>
      </div>
    </div>
  );
}
