import type { CSSProperties, ReactNode } from 'react';
import { X } from 'lucide-react';
import type { LifecycleAction, Note, Task, Thread, ThreadSendKind } from '../types';
import type { editTask } from '../api';
import { EditableField } from './EditableField';
import { DatePicker } from './DatePicker';
import { AgentPicker } from './AgentPicker';
import { Markdown } from './Markdown';
import { ConversationPanel } from './ConversationPanel';
import { NotesSection } from './NotesSection';
import { ConvertToService } from './ConvertToService';
import { PriorityBadge, StatusPill } from './Badges';
import { sectionLabelStyle } from '../styles';
import { statusFor } from '../mockAgentStatus';
import { WORK_STATUS_COLOR, WORK_STATUS_LABEL, dueLabelLong, timestampLabel } from '../taskFormat';

const PRIORITY_OPTIONS = [
  { value: 'critical', label: 'Critical' },
  { value: 'high', label: 'High' },
  { value: 'medium', label: 'Medium' },
  { value: 'low', label: 'Low' },
];

const EFFORT_OPTIONS = [
  { value: 'high', label: 'High' },
  { value: 'medium', label: 'Medium' },
  { value: 'low', label: 'Low' },
];

// Each work status the dropdown can set, and the lifecycle move that sets it.
// Agent-only states (needs-input, waiting-for-review) and the ones only a
// reply sets (changes-requested) never appear for the human.
const WORK_STATUS_MOVES: { value: NonNullable<Task['work_status']>; label: string; action: LifecycleAction }[] = [
  { value: 'todo', label: 'To Do', action: 'reset' },
  { value: 'in-progress', label: 'In Progress', action: 'start' },
  { value: 'needs-input', label: 'Needs Input', action: 'ask' },
  { value: 'waiting-for-review', label: 'Waiting for Review', action: 'submit' },
];

/** The current work status plus every status the server says the human may move to. */
function workStatusOptions(task: Task) {
  const current = task.work_status ?? '';
  const allowed = task.allowed_actions ?? [];
  const options = [{ value: current, label: current ? WORK_STATUS_LABEL[current] : 'None' }];
  for (const m of WORK_STATUS_MOVES) {
    if (m.value !== current && allowed.includes(m.action)) options.push({ value: m.value, label: m.label });
  }
  return options;
}

interface TaskDetailProps {
  task: Task;
  today: Date;
  /** Phone layout: bigger type and touch targets. */
  mobile?: boolean;
  /** Show `#id` in the meta row (the mobile page shows it in its top bar instead). */
  showId?: boolean;
  /** Rendered under the meta row (the desktop Mark done / Delete buttons). */
  actions?: ReactNode;
  onClose?: () => void;
  agentOptions: { value: string; label: string }[];
  thread: Thread | null;
  notes: Note[];
  onEdit: (changes: Parameters<typeof editTask>[1]) => void;
  onSendThread: (kind: ThreadSendKind, text: string) => Promise<void>;
  onApprove: (text: string) => Promise<void>;
  onCreateNote: (title: string) => Promise<string | undefined> | void;
  onOpenNote: (slug: string) => void;
  onSaveNote: (slug: string, body: string) => Promise<void>;
  onUnlinkNote: (slug: string) => void;
  /** Turn the task into a service; the button is hidden when not given. */
  onConvertToService?: (schedule: { recurrence: string; due: string }) => Promise<void>;
}

/**
 * Task inspector body, shared by the desktop side panel and the mobile
 * full-screen page. Order: title + meta, agent-waiting banner, conversation
 * panel, description + instructions, a two-column grid of the
 * short fields, then notes.
 */
export function TaskDetail({
  task, today, mobile, showId = true, actions, onClose, agentOptions, thread, notes,
  onEdit, onSendThread, onApprove, onCreateNote, onOpenNote, onSaveNote, onUnlinkNote, onConvertToService,
}: TaskDetailProps) {
  const done = task.status === 'done';
  const agentStatus = task.agent ? statusFor(task.agent) : null;
  const accent = task.work_status ? WORK_STATUS_COLOR[task.work_status] : undefined;
  const ellipsis: CSSProperties = { display: 'block', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' };
  const placeholder = (text: string) => <span style={{ color: 'var(--fg-5)' }}>{text}</span>;

  const longFields: { label: string; key: 'description' | 'instructions'; id?: string; empty: string }[] = [
    { label: 'Description', key: 'description', empty: 'No description' },
    { label: 'Instructions', key: 'instructions', id: 'field-instructions', empty: 'No instructions' },
  ];

  const gridFields: { label: string; node: ReactNode }[] = [
    {
      label: 'Due',
      node: (
        <DatePicker
          id="field-due"
          value={task.due_date ?? ''}
          onSave={(v) => onEdit({ due: v })}
          display={<span style={ellipsis}>{task.due_date ? dueLabelLong(task, today) : placeholder('Not set')}</span>}
        />
      ),
    },
    {
      label: 'Agent',
      node: <AgentPicker id="field-agent" value={task.agent ?? ''} options={agentOptions} onSave={(v) => onEdit({ agent: v })} />,
    },
    {
      label: 'Project',
      node: (
        <EditableField
          value={task.project ?? ''}
          onSave={(v) => onEdit({ project: v })}
          display={<span style={ellipsis}>{task.project || placeholder('No project')}</span>}
        />
      ),
    },
    {
      label: 'Effort',
      node: (
        <EditableField
          id="field-effort"
          value={task.effort ?? 'medium'}
          type="select"
          options={EFFORT_OPTIONS}
          onSave={(v) => onEdit({ effort: v })}
          display={<span style={ellipsis}>{task.effort ?? placeholder('—')}</span>}
        />
      ),
    },
    {
      label: 'Tags',
      node: (
        <EditableField
          id="field-tags"
          value={task.tags.join(', ')}
          onSave={(v) => onEdit({ tags: v })}
          display={<span style={ellipsis}>{task.tags.length ? task.tags.join(', ') : placeholder('No tags')}</span>}
        />
      ),
    },
    {
      label: 'Recurrence',
      node: (
        <EditableField
          value={task.recurrence ?? ''}
          placeholder="e.g. daily, weekly:2, weekly:fri, monthly:2:fri — empty to clear"
          onSave={(v) => onEdit({ recurrence: v })}
          display={<span style={ellipsis}>{task.recurrence || placeholder('None')}</span>}
        />
      ),
    },
    {
      label: 'Created',
      node: <span style={ellipsis} title={task.created}>{timestampLabel(task.created, today)}</span>,
    },
  ];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
          <div
            style={{
              flex: 1, minWidth: 0, fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: mobile ? 22 : 20,
              lineHeight: 1.3, color: 'var(--fg-1)', textDecoration: done ? 'line-through' : 'none', textWrap: 'pretty',
            }}
          >
            <EditableField
              id="field-title"
              value={task.title}
              onSave={(v) => v.trim() && onEdit({ title: v.trim() })}
              display={task.title ? <Markdown text={task.title} inline /> : undefined}
            />
          </div>
          {onClose && (
            <button
              onClick={onClose}
              title="Close inspector (Esc)"
              style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 2, flex: 'none' }}
            >
              <X size={16} color="var(--fg-4)" />
            </button>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 8, fontSize: 13, color: 'var(--fg-3)' }}>
          {showId && <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--fg-5)' }}>#{task.id}</span>}
          <EditableField
            id="field-priority"
            value={task.priority}
            type="select"
            options={PRIORITY_OPTIONS}
            onSave={(v) => onEdit({ priority: v })}
            display={<PriorityBadge priority={task.priority} />}
          />
          <EditableField
            id="field-work-status"
            value={task.work_status ?? ''}
            type="select"
            options={workStatusOptions(task)}
            onSave={(v) => onEdit({ work_status: v })}
            display={task.work_status
              ? <StatusPill status={task.work_status} />
              : <span style={{ fontSize: 12, color: 'var(--fg-5)' }}>+ Work status</span>}
          />
        </div>
        {actions && <div style={{ display: 'flex', gap: 8, marginTop: 2 }}>{actions}</div>}
      </div>

      {agentStatus?.state === 'waiting' && (
        <div
          style={{
            display: 'flex', alignItems: 'center', gap: 10, padding: mobile ? '12px 14px' : '10px 12px', borderRadius: 8,
            background: 'rgba(235, 203, 139, 0.12)', border: '1px solid var(--citrine)',
          }}
        >
          <span style={{ width: 7, height: 7, flex: 'none', borderRadius: 999, background: 'var(--citrine)' }} />
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 2 }}>
            <span style={{ ...sectionLabelStyle, color: 'var(--citrine)' }}>{agentStatus.name} is waiting</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: mobile ? 12.5 : 12, color: 'var(--fg-2)' }}>
              {agentStatus.detail ?? 'Waiting for input.'}
            </span>
          </div>
        </div>
      )}

      {thread && (thread.note || thread.allowed_action.send || thread.allowed_action.approve) && (
        <ConversationPanel task={task} thread={thread} accentColor={accent} onSend={onSendThread} onApprove={onApprove} />
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        {longFields.map((f) => {
          const value = task[f.key] ?? '';
          return (
            <div key={f.key} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={sectionLabelStyle}>{f.label}</span>
              <div style={{ fontSize: mobile ? 15 : 13.5, lineHeight: 1.5, color: 'var(--fg-2)', textWrap: 'pretty' }}>
                <EditableField
                  id={f.id}
                  value={value}
                  type="textarea"
                  onSave={(v) => onEdit({ [f.key]: v })}
                  display={value
                    ? <Markdown text={value} copyable />
                    : <span style={{ color: 'var(--fg-5)', fontStyle: 'italic' }}>{f.empty}</span>}
                />
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', borderTop: '1px solid var(--hairline-soft)' }}>
        {gridFields.map((f) => (
          <div
            key={f.label}
            style={{
              display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 3, minWidth: 0,
              minHeight: mobile ? 56 : undefined, padding: mobile ? '8px 12px 8px 0' : '10px 12px 10px 0',
              borderBottom: '1px solid var(--hairline-soft)',
            }}
          >
            <span style={{ fontSize: mobile ? 12 : 11.5, color: 'var(--fg-4)' }}>{f.label}</span>
            <div style={{ fontSize: mobile ? 14.5 : 13, color: 'var(--fg-1)', minWidth: 0 }}>{f.node}</div>
          </div>
        ))}
      </div>

      <NotesSection notes={notes} onCreate={onCreateNote} onOpen={onOpenNote} onSave={onSaveNote} onUnlink={onUnlinkNote} touch={mobile} />

      {onConvertToService && !done && task.kind !== 'service' && (
        <ConvertToService key={task.id} task={task} today={today} onConvert={onConvertToService} />
      )}
    </div>
  );
}
