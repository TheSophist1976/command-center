import { useEffect, useMemo, useState } from 'react';
import './tokens.css';
import { Terminal, Sun, CalendarDays, Settings, Search } from 'lucide-react';
import { Button } from './components/Button';
import { NewTaskForm } from './components/NewTaskForm';
import { EditableField, FieldRow } from './components/EditableField';
import { fetchTasks, fetchAgents, addTask, editTask, markDone, reopenTask, deleteTask } from './api';
import type { Task, AgentProfile } from './types';
import { countAllOpen } from './dueWindow';
import { statusFor, statusColor } from './mockAgentStatus';

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

type GroupBy = 'agent' | 'project' | 'priority' | 'none';

const GROUP_BY_OPTIONS: { value: GroupBy; label: string }[] = [
  { value: 'agent', label: 'Agent' },
  { value: 'project', label: 'Project' },
  { value: 'priority', label: 'Priority' },
  { value: 'none', label: 'None' },
];

const PRIORITY_ORDER: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };

function groupKey(task: Task, groupBy: GroupBy): string {
  switch (groupBy) {
    case 'agent':
      return task.agent ?? 'unassigned';
    case 'project':
      return task.project ?? 'no project';
    case 'priority':
      return task.priority;
    case 'none':
      return '';
  }
}

function groupTasks(tasks: Task[], groupBy: GroupBy): Map<string, Task[]> {
  const groups = new Map<string, Task[]>();
  for (const t of tasks) {
    const key = groupKey(t, groupBy);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(t);
  }
  if (groupBy === 'priority') {
    return new Map([...groups.entries()].sort((a, b) => (PRIORITY_ORDER[a[0]] ?? 99) - (PRIORITY_ORDER[b[0]] ?? 99)));
  }
  return groups;
}

const priorityColor: Record<string, string> = {
  critical: 'var(--danger)',
  high: 'var(--citrine)',
  medium: 'var(--fg-3)',
  low: 'var(--fg-5)',
};

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [selected, setSelected] = useState<Task | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showNewTaskForm, setShowNewTaskForm] = useState(false);
  const [search, setSearch] = useState('');
  const [groupBy, setGroupBy] = useState<GroupBy>('agent');

  useEffect(() => {
    Promise.all([fetchTasks(), fetchAgents()])
      .then(([t, a]) => {
        setTasks(t);
        setAgents(a);
      })
      .catch((e) => setError(String(e)));
  }, []);

  async function handleAddTask(title: string) {
    try {
      const created = await addTask({ title });
      setTasks((prev) => [...prev, created]);
      setShowNewTaskForm(false);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleMarkDone(task: Task) {
    try {
      const { completed, spawned } = await markDone(task.id);
      setTasks((prev) => {
        const next = prev.map((t) => (t.id === completed.id ? completed : t));
        return spawned ? [...next, spawned] : next;
      });
      setSelected(completed);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleReopen(task: Task) {
    try {
      const updated = await reopenTask(task.id);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
      setSelected(updated);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDelete(task: Task) {
    try {
      await deleteTask(task.id);
      setTasks((prev) => prev.filter((t) => t.id !== task.id));
      setSelected(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleEditField(id: number, changes: Parameters<typeof editTask>[1]) {
    try {
      const updated = await editTask(id, changes);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
      setSelected(updated);
    } catch (e) {
      setError(String(e));
    }
  }

  const filteredTasks = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return tasks;
    return tasks.filter((t) => t.title.toLowerCase().includes(q));
  }, [tasks, search]);

  const grouped = useMemo(() => groupTasks(filteredTasks, groupBy), [filteredTasks, groupBy]);
  const todayCount = useMemo(() => countAllOpen(tasks), [tasks]);

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      <aside
        style={{
          width: 252, flex: 'none', background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 26, height: 26, borderRadius: 5, background: 'var(--magenta)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>command center</span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <Sun size={16} color="var(--magenta)" />
            <span style={{ flex: 1, fontSize: 14 }}>All open</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--magenta)' }}>{todayCount}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <CalendarDays size={16} color="var(--fg-4)" />
            <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Agents</span>
          </div>
        </nav>
        <div style={{ padding: '8px 20px', display: 'flex', flexDirection: 'column', gap: 4 }}>
          {agents.map((a) => {
            const status = statusFor(a.name);
            return (
              <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28 }}>
                <span style={{ width: 7, height: 7, borderRadius: 999, background: statusColor(status.state) }} />
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>{a.name}</span>
              </div>
            );
          })}
        </div>
        <div style={{ marginTop: 'auto', padding: '14px 20px 0', borderTop: '1px solid var(--hairline-soft)', display: 'flex', alignItems: 'center', gap: 10 }}>
          <Settings size={16} color="var(--fg-4)" />
          <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Settings</span>
        </div>
      </aside>

      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <header style={{ height: 72, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline)', display: 'flex', alignItems: 'center', gap: 16 }}>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>All tasks</span>
          <div style={{ flex: 1 }} />
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, height: 34, padding: '0 12px', border: '1px solid var(--hairline)', borderRadius: 5 }}>
            <span style={{ fontSize: 13, color: 'var(--fg-4)' }}>Group</span>
            <select
              value={groupBy}
              onChange={(e) => setGroupBy(e.target.value as GroupBy)}
              style={{ background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)', fontSize: 13, fontWeight: 600 }}
            >
              {GROUP_BY_OPTIONS.map((o) => (
                <option key={o.value} value={o.value} style={{ background: 'var(--ink-2)' }}>
                  {o.label}
                </option>
              ))}
            </select>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, height: 34, padding: '0 12px', border: '1px solid var(--hairline)', borderRadius: 5, background: 'var(--ink-2)', width: 240 }}>
            <Search size={14} color="var(--fg-4)" />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search tasks…"
              style={{ flex: 1, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)', fontSize: 13 }}
            />
          </div>
          {showNewTaskForm ? (
            <NewTaskForm onSubmit={handleAddTask} onCancel={() => setShowNewTaskForm(false)} />
          ) : (
            <Button onClick={() => setShowNewTaskForm(true)}>New task</Button>
          )}
        </header>
        {error && <div style={{ padding: 16, color: 'var(--danger)' }}>{error}</div>}
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, height: 34, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline-soft)', fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: 96 }}>Due</span>
          <span style={{ width: 44 }}>Effort</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
          {[...grouped.entries()].map(([groupName, groupTasksList]) => (
            <div key={groupName || 'all'}>
              {groupBy !== 'none' && (
                <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 38, padding: '0 24px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600, textTransform: groupBy === 'priority' ? 'uppercase' : 'none' }}>
                    {groupName}
                  </span>
                  <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{groupTasksList.length} tasks</span>
                </div>
              )}
              {groupTasksList.map((t) => (
                <div
                  key={t.id}
                  onClick={() => setSelected(t)}
                  style={{
                    display: 'flex', alignItems: 'center', gap: 16, height: 46, padding: '0 24px',
                    borderBottom: '1px solid var(--hairline-soft)', cursor: 'pointer',
                    background: selected?.id === t.id ? 'rgba(255,0,149,0.08)' : 'transparent',
                  }}
                >
                  <span style={{ width: 34, fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-5)' }}>{t.id}</span>
                  <span style={{ width: 78, fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, textTransform: 'uppercase', color: priorityColor[t.priority] }}>
                    {t.priority}
                  </span>
                  <span style={{
                    flex: 1, minWidth: 0, fontSize: 14.5, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                    textDecoration: t.status === 'done' ? 'line-through' : 'none',
                    color: t.status === 'done' ? 'var(--fg-4)' : 'inherit',
                  }}>
                    {t.title}
                  </span>
                  <span style={{ width: 96, fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>
                    {t.due_date ?? '—'}
                  </span>
                  <span style={{ width: 44, fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-3)' }}>
                    {t.effort ?? '—'}
                  </span>
                </div>
              ))}
            </div>
          ))}
        </div>
      </main>

      <aside style={{ width: 352, flex: 'none', background: 'var(--ink-3)', borderLeft: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: selected ? '20px 24px' : 0, alignItems: selected ? 'stretch' : 'center', justifyContent: selected ? 'flex-start' : 'center', color: 'var(--fg-5)' }}>
        {selected ? (
          <>
            <div style={{
              fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 20, color: 'var(--fg-1)', marginBottom: 12,
              textDecoration: selected.status === 'done' ? 'line-through' : 'none',
            }}>
              <EditableField
                value={selected.title}
                onSave={(v) => v.trim() && handleEditField(selected.id, { title: v.trim() })}
              />
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: 'var(--fg-3)' }}>
              <span>#{selected.id} ·</span>
              <EditableField
                value={selected.priority}
                type="select"
                options={PRIORITY_OPTIONS}
                onSave={(v) => handleEditField(selected.id, { priority: v })}
                display={
                  <span style={{ color: priorityColor[selected.priority], fontWeight: 700, fontSize: 11, textTransform: 'uppercase' }}>
                    {selected.priority}
                  </span>
                }
              />
              <span>· {selected.status}</span>
            </div>
            <div style={{ display: 'flex', gap: 8, marginTop: 12, marginBottom: 20 }}>
              {selected.status === 'open' ? (
                <Button size="sm" onClick={() => handleMarkDone(selected)}>Mark done</Button>
              ) : (
                <Button size="sm" variant="secondary" onClick={() => handleReopen(selected)}>Reopen</Button>
              )}
              <Button size="sm" variant="secondary" onClick={() => handleDelete(selected)}>Delete</Button>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', marginBottom: 20 }}>
              <FieldRow label="Due">
                <EditableField
                  value={selected.due_date ?? ''}
                  type="date"
                  placeholder="No due date"
                  onSave={(v) => handleEditField(selected.id, { due: v })}
                />
              </FieldRow>
              <FieldRow label="Effort">
                <EditableField
                  value={selected.effort ?? 'medium'}
                  type="select"
                  options={EFFORT_OPTIONS}
                  onSave={(v) => handleEditField(selected.id, { effort: v })}
                  display={selected.effort ?? '—'}
                />
              </FieldRow>
              <FieldRow label="Project">
                <EditableField
                  value={selected.project ?? ''}
                  placeholder="No project"
                  onSave={(v) => handleEditField(selected.id, { project: v })}
                />
              </FieldRow>
              <FieldRow label="Agent">
                <EditableField
                  value={selected.agent ?? ''}
                  placeholder="Unassigned"
                  onSave={(v) => handleEditField(selected.id, { agent: v })}
                />
              </FieldRow>
              <FieldRow label="Tags">
                <EditableField
                  value={selected.tags.join(', ')}
                  placeholder="No tags"
                  onSave={(v) => handleEditField(selected.id, { tags: v })}
                  display={selected.tags.length ? selected.tags.join(', ') : undefined}
                />
              </FieldRow>
              <FieldRow label="Recurrence">
                <span style={{ color: 'var(--fg-4)' }}>{selected.recurrence ?? 'None'}</span>
              </FieldRow>
              <FieldRow label="Description">
                <EditableField
                  value={selected.description ?? ''}
                  type="textarea"
                  placeholder="No description"
                  onSave={(v) => handleEditField(selected.id, { description: v })}
                />
              </FieldRow>
            </div>

            {selected.agent && statusFor(selected.agent).state === 'waiting' && (
              <div style={{ marginTop: 'auto', paddingTop: 20, borderTop: '1px solid var(--hairline)' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                  <span style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--citrine)' }} />
                  <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--citrine)' }}>
                    Agent waiting (preview — not yet live)
                  </span>
                </div>
                <div style={{ padding: 14, borderRadius: 8, background: 'var(--ink-2)', border: '1px solid var(--hairline)', fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-2)' }}>
                  {statusFor(selected.agent).detail ?? 'Waiting for input.'}
                </div>
              </div>
            )}
          </>
        ) : (
          'Select a task'
        )}
      </aside>
    </div>
  );
}
