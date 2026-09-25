import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import type { Note, Task } from '../types';
import { Button } from './Button';

interface ReviewPanelProps {
  review: Note | null;
  workStatus?: Task['work_status'];
  accentColor?: string;
  onSendFeedback: (text: string) => Promise<void> | void;
}

const EMPTY_STATE_COPY: Partial<Record<NonNullable<Task['work_status']>, string>> = {
  'waiting-for-review': 'Waiting on the agent to post their review.',
};

function formatUpdated(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

export function ReviewPanel({ review, workStatus, accentColor, onSendFeedback }: ReviewPanelProps) {
  const [text, setText] = useState('');
  const [sendState, setSendState] = useState<'idle' | 'sending' | 'sent'>('idle');
  const color = accentColor ?? 'var(--fg-5)';
  const trimmed = text.trim();

  async function submit() {
    if (!trimmed || sendState === 'sending') return;
    setSendState('sending');
    try {
      await onSendFeedback(trimmed);
      setText('');
      setSendState('sent');
      setTimeout(() => setSendState('idle'), 1500);
    } catch {
      setSendState('idle');
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ width: 7, height: 7, borderRadius: 999, background: color, flexShrink: 0 }} />
        <span
          style={{
            fontFamily: 'var(--font-display)',
            fontWeight: 700,
            fontSize: 11,
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            color,
          }}
        >
          Review
        </span>
      </div>

      {review ? (
        <div
          style={{
            border: `1px solid ${color}`,
            borderRadius: 6,
            padding: '10px 12px',
            fontSize: 13,
            color: 'var(--fg-2)',
            maxHeight: 320,
            overflowY: 'auto',
          }}
        >
          <ReactMarkdown>{review.body}</ReactMarkdown>
          {review.updated && (
            <div style={{ marginTop: 8, fontSize: 11, color: 'var(--fg-5)' }}>
              Updated {formatUpdated(review.updated)}
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>
          {(workStatus && EMPTY_STATE_COPY[workStatus]) ?? 'No review thread yet.'}
        </div>
      )}

      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Leave feedback for another round…"
        rows={3}
        style={{
          width: '100%',
          background: 'var(--ink-2)',
          border: '1px solid var(--hairline)',
          borderRadius: 4,
          color: 'var(--fg-1)',
          fontSize: 13,
          fontFamily: 'inherit',
          padding: '6px 8px',
          resize: 'vertical',
          boxSizing: 'border-box',
        }}
      />
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <Button size="sm" onClick={submit} disabled={!trimmed || sendState === 'sending'}>
          {sendState === 'sending' ? 'Sending…' : 'Send feedback'}
        </Button>
        {sendState === 'sent' && (
          <span style={{ fontSize: 12, color: 'var(--teal)' }}>Sent</span>
        )}
      </div>
    </div>
  );
}
