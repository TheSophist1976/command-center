import { useState } from 'react';
import type { Task } from '../types';
import { Button } from './Button';
import { sectionLabelStyle } from '../styles';
import { formatLocalDate } from '../taskFormat';

const input = {
  background: 'var(--ink-1, transparent)', color: 'var(--fg-1)', border: '1px solid var(--hairline)',
  borderRadius: 4, padding: '4px 6px', fontSize: 13,
};

// Turns an existing task into a service (same id, notes and instructions).
// A service must have a schedule and a next run date, so both are asked for
// up front, prefilled from the task's own recurrence and due date.
export function ConvertToService({ task, today, onConvert }: {
  task: Task;
  today: Date;
  onConvert: (schedule: { recurrence: string; due: string }) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [recurrence, setRecurrence] = useState(task.recurrence ?? 'daily');
  // An overdue date would be refused (no past dates), so start from today then.
  const todayStr = formatLocalDate(today);
  const [due, setDue] = useState(task.due_date && task.due_date > todayStr ? task.due_date : todayStr);
  const [saving, setSaving] = useState(false);

  if (!open) {
    return (
      <div>
        <Button variant="secondary" onClick={() => setOpen(true)}>Convert to service…</Button>
      </div>
    );
  }

  function submit() {
    setSaving(true);
    onConvert({ recurrence: recurrence.trim(), due }).finally(() => setSaving(false));
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: 12, border: '1px solid var(--hairline)', borderRadius: 6 }}>
      <span style={sectionLabelStyle}>Convert to service</span>
      <span style={{ fontSize: 12.5, color: 'var(--fg-3)', lineHeight: 1.5 }}>
        The task leaves the task lists and runs on this schedule from the Services screen.
        {!task.agent && ' It has no agent yet, so nothing will run it until you assign one.'}
      </span>
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <input
          style={{ ...input, flex: 1, minWidth: 140 }}
          placeholder="Schedule (daily, weekly:fri, monthly:2:mon…)"
          value={recurrence}
          onChange={(e) => setRecurrence(e.target.value)}
        />
        <input style={input} type="date" value={due} onChange={(e) => setDue(e.target.value)} title="Next run" />
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        <Button onClick={submit} disabled={saving || !recurrence.trim() || !due}>Convert</Button>
        <Button variant="secondary" onClick={() => setOpen(false)} disabled={saving}>Cancel</Button>
      </div>
    </div>
  );
}
