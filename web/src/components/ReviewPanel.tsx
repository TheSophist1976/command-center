import { useEffect, useRef, useState } from 'react';
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

interface ChatMessage {
  sender: 'agent' | 'human' | 'system';
  label: string;
  dateLabel?: string;
  body: string;
}

function parseMessages(body: string): ChatMessage[] {
  const withoutTitle = body.replace(/^#\s+.*(\r?\n)+/, '');
  const sections = withoutTitle.split(/\n(?=##\s)/g).map((s) => s.trim()).filter(Boolean);

  // No "## " sections at all — treat the whole body as a single agent message
  // (covers review threads created before this format existed).
  if (sections.length === 0) {
    return withoutTitle.trim() ? [{ sender: 'agent', label: 'Agent', body: withoutTitle.trim() }] : [];
  }

  return sections.map((section) => {
    const headingMatch = section.match(/^##\s+(.*)$/m);
    const heading = headingMatch ? headingMatch[1] : '';
    const text = section.replace(/^##\s+.*$/m, '').trim();
    const isSnapshot = /^snapshot/i.test(heading);
    const isAgent = /agent|response/i.test(heading);
    const dateMatch = heading.match(/[—-]\s*(\S.*)$/);
    return {
      sender: isSnapshot ? 'system' : isAgent ? 'agent' : 'human',
      label: isSnapshot ? 'Snapshot' : isAgent ? 'Agent' : 'Mark',
      dateLabel: dateMatch ? dateMatch[1].trim() : undefined,
      body: text || heading,
    };
  });
}

function Avatar({ sender, color }: { sender: 'agent' | 'human'; color: string }) {
  return (
    <div
      style={{
        width: 24,
        height: 24,
        borderRadius: '50%',
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontFamily: 'var(--font-display)',
        fontWeight: 700,
        fontSize: 11,
        background: sender === 'human' ? color : 'var(--ink-3)',
        color: sender === 'human' ? 'var(--ink)' : 'var(--fg-3)',
        border: sender === 'human' ? 'none' : '1px solid var(--hairline)',
      }}
    >
      {sender === 'human' ? 'M' : 'A'}
    </div>
  );
}

const markdownComponents = {
  pre: ({ children }: { children?: React.ReactNode }) => (
    <pre
      style={{
        overflowX: 'auto',
        maxWidth: '100%',
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word',
        overflowWrap: 'anywhere',
      }}
    >
      {children}
    </pre>
  ),
  code: ({ children }: { children?: React.ReactNode }) => (
    <code style={{ overflowWrap: 'anywhere', wordBreak: 'break-word' }}>{children}</code>
  ),
};

export function ReviewPanel({ review, workStatus, accentColor, onSendFeedback }: ReviewPanelProps) {
  const [text, setText] = useState('');
  const [sendState, setSendState] = useState<'idle' | 'sending' | 'sent'>('idle');
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const color = accentColor ?? 'var(--fg-5)';
  const trimmed = text.trim();
  const messages = review ? parseMessages(review.body) : [];
  const scrollRef = useRef<HTMLDivElement>(null);

  async function copyMessage(i: number, body: string) {
    try {
      await navigator.clipboard.writeText(body);
      setCopiedIndex(i);
      setTimeout(() => setCopiedIndex((cur) => (cur === i ? null : cur)), 1500);
    } catch {
      // clipboard unavailable — no-op
    }
  }

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [review?.body]);

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
          Review
        </span>
      </div>

      {messages.length > 0 ? (
        <div
          ref={scrollRef}
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 10,
            border: `1px solid ${color}`,
            borderRadius: 8,
            padding: '10px 10px',
            background: 'var(--ink-2)',
            maxHeight: 360,
            overflowY: 'auto',
          }}
        >
          {messages.map((m, i) =>
            m.sender === 'system' ? (
              <div
                key={i}
                style={{
                  fontSize: 11.5,
                  color: 'var(--fg-4)',
                  fontFamily: 'var(--font-mono)',
                  border: '1px dashed var(--hairline)',
                  borderRadius: 6,
                  padding: '6px 10px',
                  alignSelf: 'center',
                }}
              >
                📌 {m.body}
                {m.dateLabel && <span style={{ marginLeft: 6, color: 'var(--fg-5)' }}>· {m.dateLabel}</span>}
              </div>
            ) : (
            <div
              key={i}
              style={{
                display: 'flex',
                flexDirection: m.sender === 'human' ? 'row-reverse' : 'row',
                gap: 8,
                alignItems: 'flex-start',
              }}
            >
              <Avatar sender={m.sender} color={color} />
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: m.sender === 'human' ? 'flex-end' : 'flex-start',
                  maxWidth: '78%',
                }}
              >
                <div
                  style={{
                    display: 'flex',
                    gap: 6,
                    alignItems: 'baseline',
                    marginBottom: 2,
                    flexDirection: m.sender === 'human' ? 'row-reverse' : 'row',
                  }}
                >
                  <span style={{ fontSize: 11, fontWeight: 700, color: 'var(--fg-2)' }}>{m.label}</span>
                  {m.dateLabel && (
                    <span style={{ fontSize: 10, color: 'var(--fg-5)' }}>{m.dateLabel}</span>
                  )}
                  <button
                    type="button"
                    onClick={() => copyMessage(i, m.body)}
                    title="Copy message text"
                    style={{
                      background: 'none',
                      border: 'none',
                      cursor: 'pointer',
                      fontSize: 10,
                      color: 'var(--fg-5)',
                      padding: 0,
                    }}
                  >
                    {copiedIndex === i ? 'Copied' : 'Copy'}
                  </button>
                </div>
                <div
                  style={{
                    borderRadius: 12,
                    borderTopLeftRadius: m.sender === 'human' ? 12 : 4,
                    borderTopRightRadius: m.sender === 'human' ? 4 : 12,
                    padding: '8px 11px',
                    fontSize: 13,
                    lineHeight: 1.4,
                    color: m.sender === 'human' ? 'var(--ink)' : 'var(--fg-2)',
                    background: m.sender === 'human' ? color : 'var(--ink-3)',
                    border: m.sender === 'human' ? 'none' : '1px solid var(--hairline)',
                    wordBreak: 'break-word',
                    maxWidth: '100%',
                    overflowX: 'hidden',
                  }}
                >
                  <ReactMarkdown components={markdownComponents}>{m.body}</ReactMarkdown>
                </div>
              </div>
            </div>
            ),
          )}
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>
          {(workStatus && EMPTY_STATE_COPY[workStatus]) ?? 'No review thread yet.'}
        </div>
      )}

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
          placeholder="Message the agent… (Enter to send, Shift+Enter for a new line)"
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
    </div>
  );
}
