import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import type { Note } from '../types';
import { Button } from './Button';

interface ReviewPanelProps {
  review: Note | null;
  onSendFeedback: (text: string) => void;
}

export function ReviewPanel({ review, onSendFeedback }: ReviewPanelProps) {
  const [text, setText] = useState('');

  function submit() {
    const trimmed = text.trim();
    if (!trimmed) return;
    onSendFeedback(trimmed);
    setText('');
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
      <div
        style={{
          fontFamily: 'var(--font-display)',
          fontWeight: 700,
          fontSize: 11,
          letterSpacing: '0.08em',
          textTransform: 'uppercase',
          color: 'var(--fg-5)',
        }}
      >
        Review
      </div>

      {review ? (
        <div
          style={{
            border: '1px solid var(--hairline)',
            borderRadius: 6,
            padding: '10px 12px',
            fontSize: 13,
            color: 'var(--fg-2)',
            maxHeight: 320,
            overflowY: 'auto',
          }}
        >
          <ReactMarkdown>{review.body}</ReactMarkdown>
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>No review thread yet.</div>
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
      <Button size="sm" onClick={submit}>Send feedback</Button>
    </div>
  );
}
