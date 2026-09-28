import ReactMarkdown from 'react-markdown';
import type { Note } from '../types';

interface QuestionPanelProps {
  question: Note | null;
  accentColor?: string;
}

export function QuestionPanel({ question, accentColor }: QuestionPanelProps) {
  const color = accentColor ?? 'var(--danger)';

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
          Needs Input
        </span>
      </div>

      {question ? (
        <div
          style={{
            border: `1px solid ${color}`,
            borderRadius: 8,
            padding: '10px 12px',
            background: 'var(--ink-2)',
            fontSize: 13,
            color: 'var(--fg-2)',
            maxHeight: 320,
            overflowY: 'auto',
          }}
        >
          <ReactMarkdown>{question.body}</ReactMarkdown>
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>
          Waiting on the agent to post its question.
        </div>
      )}

      <div style={{ fontSize: 12, color: 'var(--fg-5)' }}>
        Answer at the terminal where the agent is running — this task will update automatically once it's answered.
      </div>
    </div>
  );
}
