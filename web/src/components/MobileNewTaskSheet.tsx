import { useState, type CSSProperties } from 'react';
import { addDays, formatLocalDate } from '../taskFormat';

const DUE_CHIPS: { label: string; days: number | null }[] = [
  { label: 'Today', days: 0 },
  { label: 'Tomorrow', days: 1 },
  { label: '+1 week', days: 7 },
  { label: 'None', days: null },
];

interface MobileNewTaskSheetProps {
  today: Date;
  agentNames: string[];
  onSubmit: (input: { title: string; due?: string; agent?: string }) => void;
  onClose: () => void;
}

function chipStyle(active: boolean, mono = false): CSSProperties {
  return {
    flex: 'none', height: 36, padding: '0 14px', borderRadius: 999, cursor: 'pointer',
    border: `1px solid ${active ? 'var(--accent)' : 'var(--hairline)'}`,
    background: active ? 'var(--accent-soft)' : 'transparent',
    color: active ? 'var(--fg-1)' : 'var(--fg-3)',
    fontSize: mono ? 13 : 13.5, fontWeight: 600, fontFamily: mono ? 'var(--font-mono)' : 'inherit',
  };
}

/** Bottom sheet for adding a task on a phone, with due-date and agent chips so it can skip the Inbox. */
export function MobileNewTaskSheet({ today, agentNames, onSubmit, onClose }: MobileNewTaskSheetProps) {
  const [title, setTitle] = useState('');
  const [dueDays, setDueDays] = useState<number | null>(0);
  const [agent, setAgent] = useState('');
  const willInbox = dueDays === null || !agent;

  return (
    <>
      <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 40, background: 'rgba(0, 0, 0, 0.5)' }} />
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const t = title.trim();
          if (!t) return;
          onSubmit({
            title: t,
            due: dueDays === null ? undefined : formatLocalDate(addDays(today, dueDays)),
            agent: agent || undefined,
          });
        }}
        style={{
          position: 'fixed', left: 0, right: 0, bottom: 0, zIndex: 41, background: 'var(--ink-2)',
          borderTop: '1px solid var(--hairline)', borderRadius: '16px 16px 0 0',
          padding: '10px 16px calc(16px + env(safe-area-inset-bottom, 18px))',
          display: 'flex', flexDirection: 'column', gap: 14,
        }}
      >
        <div style={{ alignSelf: 'center', width: 36, height: 5, borderRadius: 999, background: 'var(--fg-faint)' }} />
        <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 17 }}>New task</div>
        <input
          autoFocus
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Escape') onClose(); }}
          placeholder="Task title"
          style={{
            height: 48, padding: '0 14px', borderRadius: 5, border: '1px solid var(--accent)',
            background: 'var(--ink)', color: 'var(--fg-1)', fontSize: 16,
          }}
        />
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>Due</span>
          <div className="no-scrollbar" style={{ display: 'flex', gap: 6, overflowX: 'auto' }}>
            {DUE_CHIPS.map((c) => (
              <button key={c.label} type="button" onClick={() => setDueDays(c.days)} style={chipStyle(dueDays === c.days)}>
                {c.label}
              </button>
            ))}
          </div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>Agent</span>
          <div className="no-scrollbar" style={{ display: 'flex', gap: 6, overflowX: 'auto' }}>
            {agentNames.map((name) => (
              <button key={name} type="button" onClick={() => setAgent(agent === name ? '' : name)} style={chipStyle(agent === name, true)}>
                {name}
              </button>
            ))}
          </div>
        </div>
        <div style={{ fontSize: 12.5, color: willInbox ? 'var(--citrine)' : 'var(--teal)' }}>
          {willInbox ? 'Will land in Inbox until it has a due date and an agent.' : 'Ready to go — skips the Inbox.'}
        </div>
        <div style={{ display: 'flex', gap: 10 }}>
          <button
            type="button"
            onClick={onClose}
            style={{
              height: 48, padding: '0 20px', fontSize: 16, fontWeight: 600, borderRadius: 5, cursor: 'pointer',
              background: 'transparent', color: 'var(--fg-1)', border: '1px solid var(--hairline)',
            }}
          >
            Cancel
          </button>
          <button
            type="submit"
            style={{
              flex: 1, height: 48, padding: '0 24px', fontSize: 16, fontWeight: 600, borderRadius: 5, cursor: 'pointer',
              background: 'var(--accent)', color: 'var(--ink)', border: '1px solid transparent',
            }}
          >
            Add task
          </button>
        </div>
      </form>
    </>
  );
}
