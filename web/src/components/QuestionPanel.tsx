import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import type { Note } from '../types';
import { Button } from './Button';

interface QuestionPanelProps {
  question: Note | null;
  awaitingAnswer: boolean;
  accentColor?: string;
  onSendAnswer: (text: string) => Promise<void> | void;
}

export function QuestionPanel({ question, awaitingAnswer, accentColor, onSendAnswer }: QuestionPanelProps) {
  const [text, setText] = useState('');
  const [sendState, setSendState] = useState<'idle' | 'sending' | 'sent'>('idle');
  const color = accentColor ?? 'var(--danger)';
  const trimmed = text.trim();

  async function submit() {
    if (!trimmed || sendState === 'sending') return;
    setSendState('sending');
    try {
      await onSendAnswer(trimmed);
      setText('');
      setSendState('sent');
      setTimeout(() => setSendState('idle'), 1500);
    } catch {
      setSendState('idle');
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
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
          {awaitingAnswer ? 'Needs Input' : 'Question'}
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

      {awaitingAnswer && (
        <>
          <div
            style={{
              display: 'flex',
              alignItems: 'flex-end',
              gap: 8,
              background: 'var(--ink-2)',
              border: '1px solid var(--hairline)',
              borderRadius: 18,
              padding: '6px 6px 6px 12px',
            }}
          >
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Answer the agent… (Enter to send, Shift+Enter for a new line)"
              rows={1}
              style={{
                flex: 1,
                background: 'transparent',
                border: 'none',
                outline: 'none',
                color: 'var(--fg-1)',
                fontSize: 13,
                fontFamily: 'inherit',
                padding: '6px 0',
                resize: 'none',
                boxSizing: 'border-box',
                maxHeight: 120,
              }}
            />
            <Button size="sm" onClick={submit} disabled={!trimmed || sendState === 'sending'}>
              {sendState === 'sending' ? '…' : sendState === 'sent' ? 'Sent' : 'Send'}
            </Button>
          </div>
          <div style={{ fontSize: 12, color: 'var(--fg-5)' }}>
            Sending marks the task changes-requested with your answer in the question note; the agent picks it up on its next run.
            If the agent is still waiting at its terminal, you can also answer there.
          </div>
        </>
      )}
    </div>
  );
}
