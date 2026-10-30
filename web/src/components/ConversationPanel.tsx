import { useEffect, useRef, useState } from 'react';
import type { Task, Thread, ThreadSendKind } from '../types';
import { Button } from './Button';
import { CopyButton, Markdown } from './Markdown';

interface ConversationPanelProps {
  task: Task;
  thread: Thread | null;
  accentColor?: string;
  onSend: (kind: ThreadSendKind, text: string) => Promise<void>;
  onApprove: (text: string) => Promise<void>;
}

type Sender = 'agent' | 'human' | 'system';

interface ChatMessage {
  kind: string;
  sender: Sender;
  label: string;
  dateLabel?: string;
  body: string;
}

const KIND_SENDER: Record<string, Sender> = {
  agent: 'agent',
  response: 'agent',
  question: 'agent',
  snapshot: 'system',
  approval: 'system',
};

/** Thread sections are `## <Kind> — <date>`; the kind decides who spoke. */
function parseMessages(body: string): ChatMessage[] {
  const withoutTitle = body.replace(/^#\s+.*(\r?\n)+/, '');
  const sections = withoutTitle.split(/\n(?=##\s)/g).map((s) => s.trim()).filter(Boolean);
  if (sections.length === 0) {
    return withoutTitle.trim() ? [{ kind: 'agent', sender: 'agent', label: 'Agent', body: withoutTitle.trim() }] : [];
  }
  return sections.map((section) => {
    const heading = section.match(/^##\s+(.*)$/m)?.[1] ?? '';
    const text = section.replace(/^##\s+.*$/m, '').trim();
    const kind = (heading.match(/^[A-Za-z]+/)?.[0] ?? '').toLowerCase();
    const sender = KIND_SENDER[kind] ?? 'human';
    const date = heading.match(/[—–-]\s*(\S.*)$/)?.[1]?.trim();
    const label =
      kind === 'snapshot' ? 'Snapshot'
      : kind === 'approval' ? 'Approved'
      : kind === 'question' ? 'Question'
      : sender === 'agent' ? 'Agent'
      : 'Mark';
    return { kind, sender, label, dateLabel: date, body: text || heading };
  });
}

const COMPOSER: Record<ThreadSendKind, { title: string; placeholder: string; button: string; hint?: string }> = {
  answer: {
    title: 'Needs input',
    placeholder: 'Answer the agent… (Enter to send, Shift+Enter for a new line)',
    button: 'Answer',
    hint: 'Sending hands the task back to the agent with your answer.',
  },
  feedback: {
    title: 'Ready for review',
    placeholder: 'Request changes… (Enter to send, Shift+Enter for a new line)',
    button: 'Request changes',
    hint: 'Sending sends the task back for another round.',
  },
  message: {
    title: 'Conversation',
    placeholder: 'Message the agent… (Enter to send, Shift+Enter for a new line)',
    button: 'Send',
    hint: 'The agent reads this the next time it works the task. The status does not change.',
  },
};

function Avatar({ sender, color }: { sender: 'agent' | 'human'; color: string }) {
  const human = sender === 'human';
  return (
    <div
      style={{
        width: 24, height: 24, borderRadius: '50%', flexShrink: 0, display: 'flex', alignItems: 'center',
        justifyContent: 'center', fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11,
        background: human ? color : 'var(--ink-3)', color: human ? 'var(--ink)' : 'var(--fg-3)',
        border: human ? 'none' : '1px solid var(--hairline)',
      }}
    >
      {human ? 'M' : 'A'}
    </div>
  );
}

export function ConversationPanel({ task, thread, accentColor, onSend, onApprove }: ConversationPanelProps) {
  const [text, setText] = useState('');
  const [sendState, setSendState] = useState<'idle' | 'sending' | 'sent'>('idle');
  const [open, setOpen] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const body = thread?.note?.body ?? '';
  const messages = body ? parseMessages(body) : [];
  const send = thread?.allowed_action.send ?? null;
  const approve = thread?.allowed_action.approve ?? false;
  const interactive = send !== null || approve;
  const color = accentColor ?? 'var(--fg-5)';
  const trimmed = text.trim();
  const mode = send ? COMPOSER[send] : null;
  const expanded = interactive || open;

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [body, expanded]);

  async function run(action: () => Promise<void>) {
    if (sendState === 'sending') return;
    setSendState('sending');
    try {
      await action();
      setText('');
      setSendState('sent');
      setTimeout(() => setSendState('idle'), 1500);
    } catch {
      setSendState('idle');
    }
  }

  const submit = () => {
    if (!send || !trimmed) return;
    run(() => onSend(send, trimmed));
  };

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  const title = mode?.title ?? (messages.length ? `Conversation · ${messages.length} message${messages.length === 1 ? '' : 's'}` : 'Conversation');
  const lastHuman = messages.length > 0 && messages[messages.length - 1].sender === 'human';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
      <button
        type="button"
        onClick={() => !interactive && setOpen((v) => !v)}
        aria-expanded={expanded}
        style={{
          display: 'flex', alignItems: 'center', gap: 8, background: 'none', border: 'none', padding: 0,
          cursor: interactive ? 'default' : 'pointer', textAlign: 'left',
        }}
      >
        <span style={{ width: 7, height: 7, borderRadius: 999, background: color, flexShrink: 0 }} />
        <span
          style={{
            fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11, letterSpacing: '0.08em',
            textTransform: 'uppercase', color,
          }}
        >
          {title}
        </span>
        {!interactive && <span style={{ fontSize: 11, color: 'var(--fg-5)' }}>{open ? 'Hide' : 'Show'}</span>}
      </button>

      {expanded && (messages.length > 0 ? (
        <div
          ref={scrollRef}
          style={{
            display: 'flex', flexDirection: 'column', gap: 10, border: `1px solid ${color}`, borderRadius: 8,
            padding: '10px 10px', background: 'var(--ink-2)', maxHeight: 360, overflowY: 'auto',
          }}
        >
          {messages.map((m, i) =>
            m.sender === 'system' ? (
              <div
                key={i}
                style={{
                  fontSize: 11.5, color: 'var(--fg-4)', fontFamily: 'var(--font-mono)',
                  border: '1px dashed var(--hairline)', borderRadius: 6, padding: '6px 10px', alignSelf: 'center',
                }}
              >
                {m.kind === 'approval' ? '✓' : '📌'} <Markdown text={m.kind === 'approval' ? `${m.label}: ${m.body}` : m.body} inline />
                {m.dateLabel && <span style={{ marginLeft: 6, color: 'var(--fg-5)' }}>· {m.dateLabel}</span>}
              </div>
            ) : (
              <div
                key={i}
                style={{ display: 'flex', flexDirection: m.sender === 'human' ? 'row-reverse' : 'row', gap: 8, alignItems: 'flex-start' }}
              >
                <Avatar sender={m.sender} color={color} />
                <div
                  style={{
                    display: 'flex', flexDirection: 'column', maxWidth: '78%',
                    alignItems: m.sender === 'human' ? 'flex-end' : 'flex-start',
                  }}
                >
                  <div
                    style={{
                      display: 'flex', gap: 6, alignItems: 'baseline', marginBottom: 2,
                      flexDirection: m.sender === 'human' ? 'row-reverse' : 'row',
                    }}
                  >
                    <span style={{ fontSize: 11, fontWeight: 700, color: m.kind === 'question' ? color : 'var(--fg-2)' }}>
                      {m.label}
                    </span>
                    {m.dateLabel && <span style={{ fontSize: 10, color: 'var(--fg-5)' }}>{m.dateLabel}</span>}
                    <CopyButton text={m.body} title="Copy message text" label style={{ padding: 0 }} />
                  </div>
                  <div
                    style={{
                      borderRadius: 12,
                      borderTopLeftRadius: m.sender === 'human' ? 12 : 4,
                      borderTopRightRadius: m.sender === 'human' ? 4 : 12,
                      padding: '8px 11px', fontSize: 13, lineHeight: 1.4,
                      color: m.sender === 'human' ? 'var(--ink)' : 'var(--fg-2)',
                      background: m.sender === 'human' ? color : 'var(--ink-3)',
                      border: m.kind === 'question' ? `1px solid ${color}` : m.sender === 'human' ? 'none' : '1px solid var(--hairline)',
                      wordBreak: 'break-word', maxWidth: '100%', overflowX: 'hidden',
                    }}
                  >
                    <Markdown text={m.body} className={m.sender === 'human' ? 'md-on-accent' : undefined} />
                  </div>
                </div>
              </div>
            ),
          )}
          {thread?.unread && lastHuman && (
            <div style={{ alignSelf: 'flex-end', fontSize: 10.5, color: 'var(--fg-5)', fontStyle: 'italic' }}>
              Not yet seen by the agent
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>
          {task.work_status === 'needs-input' ? 'Waiting on the agent to post its question.'
            : task.work_status === 'waiting-for-review' ? 'Waiting on the agent to post their review.'
            : 'No messages yet.'}
        </div>
      ))}

      {send && mode && (
        <>
          <div
            style={{
              display: 'flex', alignItems: 'flex-end', gap: 8, background: 'var(--ink-2)',
              border: '1px solid var(--hairline)', borderRadius: 18, padding: '6px 6px 6px 12px',
            }}
          >
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={mode.placeholder}
              rows={1}
              autoFocus={send === 'answer'}
              style={{
                flex: 1, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)',
                fontSize: 13, fontFamily: 'inherit', padding: '6px 0', resize: 'none', boxSizing: 'border-box',
                maxHeight: 120,
              }}
            />
            <Button size="sm" onClick={submit} disabled={!trimmed || sendState === 'sending'}>
              {sendState === 'sending' ? '…' : sendState === 'sent' ? 'Sent' : mode.button}
            </Button>
          </div>
          {mode.hint && <div style={{ fontSize: 12, color: 'var(--fg-5)' }}>{mode.hint}</div>}
        </>
      )}

      {approve && (
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <Button size="sm" variant="cyan" onClick={() => run(() => onApprove(trimmed))} disabled={sendState === 'sending'}>
            Approve{trimmed ? ' with note' : ''}
          </Button>
        </div>
      )}
    </div>
  );
}
