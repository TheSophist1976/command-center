import { useEffect, useMemo, useRef, useState } from 'react';
import './tokens.css';
import './taskPanel.css';
import { Terminal, Sun, CalendarDays, Settings, Search, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, X, AlertTriangle, HelpCircle, Repeat, CalendarOff, UserX, Inbox } from 'lucide-react';
import { Button } from './components/Button';
import { NewTaskForm } from './components/NewTaskForm';
import { EditableField, FieldRow } from './components/EditableField';
import { fetchTasks, fetchAgents, addTask, editTask, markDone, reopenTask, deleteTask, fetchTaskNotes, createTaskNote, openNote, unlinkTaskNote, fetchTaskReview, postTaskFeedback, fetchTaskQuestion, postTaskAnswer } from './api';
import { NotesSection } from './components/NotesSection';
import { ReviewPanel } from './components/ReviewPanel';
import { QuestionPanel } from './components/QuestionPanel';
import { AgentPicker } from './components/AgentPicker';
import { AgentEditor } from './components/AgentEditor';
import { DatePicker } from './components/DatePicker';
import { Linkify } from './components/Linkify';
import { VersionBadge } from './components/VersionBadge';
import type { Task, AgentProfile, Note } from './types';
import { countDueWindow, dueMatches, startOfToday, isOverdue, type DueWindow } from './dueWindow';
import { statusFor, statusColor } from './mockAgentStatus';

type DueFilter = DueWindow | 'no-due-date' | 'recurring' | 'all-tasks' | 'inbox';

const INBOX_ITEMS: { value: DueFilter; label: string }[] = [
  { value: 'inbox', label: 'Inbox' },
];

const DUE_WINDOW_ITEMS: { value: DueFilter; label: string }[] = [
  { value: 'day', label: 'Today' },
  { value: 'week', label: 'This week' },
  { value: 'month', label: 'This month' },
  { value: 'year', label: 'This year' },
  { value: 'no-due-date', label: 'No due date' },
  { value: 'recurring', label: 'Recurring' },
  { value: 'all-tasks', label: 'All tasks' },
];

const ALL_FILTER_ITEMS = [...INBOX_ITEMS, ...DUE_WINDOW_ITEMS];

function hasNoDueDate(t: Task): boolean {
  return t.status === 'open' && !t.due_date;
}

function isRecurring(t: Task): boolean {
  return t.status === 'open' && !!t.recurrence;
}

function missingAgent(t: Task): boolean {
  return t.status === 'open' && !t.agent;
}

function isInboxTask(t: Task, today: Date): boolean {
  return t.status === 'open' && (!t.agent || !t.due_date || isOverdue(t, today));
}

function countForDueFilter(tasks: Task[], today: Date, value: DueFilter): number {
  switch (value) {
    case 'inbox':
      return tasks.filter((t) => isInboxTask(t, today)).length;
    case 'all-tasks':
      return tasks.length;
    case 'no-due-date':
      return tasks.filter(hasNoDueDate).length;
    case 'recurring':
      return tasks.filter(isRecurring).length;
    default:
      return countDueWindow(tasks, today, value);
  }
}

const PANEL_MIN_WIDTH = 352;
const PANEL_MAX_WIDTH = 880;
const PANEL_DEFAULT_WIDTH = 440;

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

const WORK_STATUS_OPTIONS = [
  { value: '', label: 'None' },
  { value: 'todo', label: 'To Do' },
  { value: 'in-progress', label: 'In Progress' },
  { value: 'waiting-for-review', label: 'Waiting for Review' },
  { value: 'changes-requested', label: 'Changes Requested' },
  { value: 'needs-input', label: 'Needs Input' },
  { value: 'complete', label: 'Complete' },
];

const WORK_STATUS_LABEL: Record<string, string> = {
  'todo': 'To Do',
  'in-progress': 'In Progress',
  'waiting-for-review': 'Waiting for Review',
  'changes-requested': 'Changes Requested',
  'needs-input': 'Needs Input',
  'complete': 'Complete',
};

const WORK_STATUS_COLOR: Record<string, string> = {
  'todo': 'var(--fg-4)',
  'in-progress': 'var(--citrine)',
  'waiting-for-review': 'var(--cyan)',
  'changes-requested': 'var(--danger)',
  'needs-input': 'var(--danger)',
  'complete': 'var(--teal)',
};

type GroupBy = 'agent' | 'project' | 'priority' | 'work_status' | 'none';

const GROUP_BY_OPTIONS: { value: GroupBy; label: string }[] = [
  { value: 'agent', label: 'Agent' },
  { value: 'project', label: 'Project' },
  { value: 'priority', label: 'Priority' },
  { value: 'work_status', label: 'Work status' },
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
    case 'work_status':
      return task.work_status ?? 'no status';
    case 'none':
      return '';
  }
}

function byPriority(a: Task, b: Task): number {
  return (PRIORITY_ORDER[a.priority] ?? 99) - (PRIORITY_ORDER[b.priority] ?? 99);
}

function groupTasks(tasks: Task[], groupBy: GroupBy): Map<string, Task[]> {
  const groups = new Map<string, Task[]>();
  for (const t of tasks) {
    const key = groupKey(t, groupBy);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(t);
  }
  for (const group of groups.values()) {
    group.sort(byPriority);
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
  const [dueFilter, setDueFilter] = useState<DueFilter>('day');
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(true);
  const [panelPhase, setPanelPhase] = useState<'closed' | 'entering' | 'open' | 'leaving'>('open');
  const [panelWidth, setPanelWidth] = useState(() => {
    const stored = Number(localStorage.getItem('taskPanelWidth'));
    return Number.isFinite(stored) && stored >= PANEL_MIN_WIDTH && stored <= PANEL_MAX_WIDTH ? stored : PANEL_DEFAULT_WIDTH;
  });
  const [isResizingPanel, setIsResizingPanel] = useState(false);
  const [taskNotes, setTaskNotes] = useState<Note[]>([]);
  const [review, setReview] = useState<Note | null>(null);
  const [question, setQuestion] = useState<Note | null>(null);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentProfile | null>(null);
  const today = useMemo(() => startOfToday(), []);
  const agentOptions = useMemo(
    () => [
      { value: '', label: 'Unassigned' },
      ...agents.map((a) => ({ value: a.name, label: a.name })),
      { value: 'human', label: 'human' },
    ],
    [agents],
  );
  const searchInputRef = useRef<HTMLInputElement>(null);
  const panelRaf1 = useRef<number>(0);
  const panelRaf2 = useRef<number>(0);
  const reviewSlug = selected ? `task-${selected.id}-review-thread` : null;
  const questionSlug = selected ? `task-${selected.id}-question` : null;
  const visibleNotes = taskNotes.filter((n) => n.slug !== reviewSlug && n.slug !== questionSlug);

  function selectTask(task: Task) {
    setSelected(task);
    setRightOpen(true);
  }

  useEffect(() => {
    Promise.all([fetchTasks(), fetchAgents()])
      .then(([t, a]) => {
        setTasks(t);
        setAgents(a);
      })
      .catch((e) => setError(String(e)));
  }, []);

  // Live updates: task_server broadcasts over SSE whenever tasks.db changes
  // on disk, from any writer (this tab, another tab, the CLI, an agent).
  // Refetch everything and re-sync `selected` by id rather than trying to
  // diff/merge — simplest correct approach at this app's scale.
  useEffect(() => {
    const source = new EventSource('/api/events');
    source.onmessage = () => {
      fetchTasks()
        .then((t) => {
          setTasks(t);
          setSelected((prev) => (prev ? t.find((task) => task.id === prev.id) ?? null : prev));
        })
        .catch((e) => setError(String(e)));
      fetchAgents()
        .then(setAgents)
        .catch((e) => setError(String(e)));
    };
    return () => source.close();
  }, []);

  useEffect(() => {
    if (!selected) {
      setTaskNotes([]);
      return;
    }
    fetchTaskNotes(selected.id)
      .then(setTaskNotes)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);

  useEffect(() => {
    if (!selected) {
      setReview(null);
      return;
    }
    fetchTaskReview(selected.id)
      .then(setReview)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);

  useEffect(() => {
    if (!selected) {
      setQuestion(null);
      return;
    }
    fetchTaskQuestion(selected.id)
      .then(setQuestion)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);

  useEffect(() => {
    if (!selected) return;
    document.querySelector(`[data-task-row="${selected.id}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selected?.id]);

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
      const currentIndex = flatOrder.findIndex((t) => t.id === task.id);
      const { completed, spawned } = await markDone(task.id);
      setTasks((prev) => {
        const next = prev.map((t) => (t.id === completed.id ? completed : t));
        return spawned ? [...next, spawned] : next;
      });
      const nextSelection = currentIndex === -1
        ? completed
        : flatOrder[currentIndex + 1] ?? flatOrder[currentIndex - 1] ?? completed;
      selectTask(nextSelection);
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

  function updateTaskNoteSlugs(taskId: number, notes: string[]) {
    setTasks((prev) => prev.map((t) => (t.id === taskId ? { ...t, notes } : t)));
    setSelected((prev) => (prev && prev.id === taskId ? { ...prev, notes } : prev));
  }

  async function handleCreateNote(title: string) {
    if (!selected) return;
    try {
      const note = await createTaskNote(selected.id, title);
      setTaskNotes((prev) => [...prev, note]);
      updateTaskNoteSlugs(selected.id, [...(selected.notes ?? []), note.slug]);
      await handleOpenNote(note.slug);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleOpenNote(slug: string) {
    try {
      await openNote(slug);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleUnlinkNote(slug: string) {
    if (!selected) return;
    try {
      await unlinkTaskNote(selected.id, slug);
      setTaskNotes((prev) => prev.filter((n) => n.slug !== slug));
      updateTaskNoteSlugs(selected.id, (selected.notes ?? []).filter((s) => s !== slug));
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleSendFeedback(text: string) {
    if (!selected) return;
    try {
      const note = await postTaskFeedback(selected.id, text);
      setReview(note);
      const updated = { ...selected, work_status: 'changes-requested' as const };
      setSelected(updated);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }

  async function handleSendAnswer(text: string) {
    if (!selected) return;
    try {
      const note = await postTaskAnswer(selected.id, text);
      setQuestion(note);
      const updated = { ...selected, work_status: 'changes-requested' as const };
      setSelected(updated);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }

  const filteredTasks = useMemo(() => {
    let result = tasks;
    if (dueFilter === 'inbox') {
      result = result.filter((t) => isInboxTask(t, today));
    } else if (dueFilter === 'no-due-date') {
      result = result.filter(hasNoDueDate);
    } else if (dueFilter === 'recurring') {
      result = result.filter(isRecurring);
    } else if (dueFilter !== 'all-tasks') {
      result = result.filter((t) => dueMatches(t, today, dueFilter));
    }
    const q = search.trim().toLowerCase();
    if (q) {
      result = result.filter((t) => t.title.toLowerCase().includes(q));
    }
    return result;
  }, [tasks, dueFilter, search, today]);

  const grouped = useMemo(() => groupTasks(filteredTasks, groupBy), [filteredTasks, groupBy]);
  const flatOrder = useMemo(() => [...grouped.values()].flat(), [grouped]);

  useEffect(() => {
    function isTypingTarget(target: EventTarget | null): boolean {
      if (!(target instanceof HTMLElement)) return false;
      const tag = target.tagName;
      return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target.isContentEditable;
    }

    function clickField(id: string) {
      if (!selected || !rightOpen) return;
      document.getElementById(id)?.click();
    }

    function moveSelection(delta: number) {
      if (flatOrder.length === 0) return;
      const currentIndex = selected ? flatOrder.findIndex((t) => t.id === selected.id) : -1;
      const nextIndex = currentIndex === -1 ? 0 : Math.min(Math.max(currentIndex + delta, 0), flatOrder.length - 1);
      selectTask(flatOrder[nextIndex]);
    }

    function toggleDone(task: Task) {
      if (task.status === 'done') handleReopen(task);
      else handleMarkDone(task);
    }

    function cycleDueFilter(delta: number) {
      const idx = DUE_WINDOW_ITEMS.findIndex((i) => i.value === dueFilter);
      const next = (idx + delta + DUE_WINDOW_ITEMS.length) % DUE_WINDOW_ITEMS.length;
      setDueFilter(DUE_WINDOW_ITEMS[next].value);
    }

    function cycleGroupBy() {
      const idx = GROUP_BY_OPTIONS.findIndex((o) => o.value === groupBy);
      const next = (idx + 1) % GROUP_BY_OPTIONS.length;
      setGroupBy(GROUP_BY_OPTIONS[next].value);
    }

    function formatLocalDate(d: Date): string {
      const y = d.getFullYear();
      const m = String(d.getMonth() + 1).padStart(2, '0');
      const day = String(d.getDate()).padStart(2, '0');
      return `${y}-${m}-${day}`;
    }

    function addDays(d: Date, days: number): Date {
      const copy = new Date(d);
      copy.setDate(copy.getDate() + days);
      return copy;
    }

    function addMonths(d: Date, months: number): Date {
      const copy = new Date(d);
      copy.setMonth(copy.getMonth() + months);
      return copy;
    }

    function setQuickDue(offset: Date | null) {
      if (!selected) return;
      handleEditField(selected.id, { due: offset ? formatLocalDate(offset) : '' });
    }

    function handleKeyDown(e: KeyboardEvent) {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (isTypingTarget(e.target)) return;

      switch (e.key) {
        case 'j':
        case 'ArrowDown':
          e.preventDefault();
          moveSelection(1);
          break;
        case 'k':
        case 'ArrowUp':
          e.preventDefault();
          moveSelection(-1);
          break;
        case 'Enter':
        case ' ':
          if (selected) {
            e.preventDefault();
            toggleDone(selected);
          }
          break;
        case 'a':
          e.preventDefault();
          setShowNewTaskForm(true);
          break;
        case 'e':
          clickField('field-title');
          break;
        case 'd':
          clickField('field-due');
          break;
        case 'p':
          clickField('field-priority');
          break;
        case 't':
          clickField('field-tags');
          break;
        case 'A':
          e.preventDefault();
          clickField('field-agent');
          break;
        case 'E':
          clickField('field-effort');
          break;
        case 'g':
          if (selected?.notes?.length) handleOpenNote(selected.notes[0]);
          break;
        case 'T':
          if (selected) { e.preventDefault(); setQuickDue(today); }
          break;
        case 'N':
          if (selected) { e.preventDefault(); setQuickDue(addDays(today, 1)); }
          break;
        case 'W':
          if (selected) { e.preventDefault(); setQuickDue(addDays(today, 7)); }
          break;
        case 'M':
          if (selected) { e.preventDefault(); setQuickDue(addMonths(today, 1)); }
          break;
        case 'Q':
          if (selected) { e.preventDefault(); setQuickDue(addMonths(today, 3)); }
          break;
        case 'Y':
          if (selected) { e.preventDefault(); setQuickDue(addMonths(today, 12)); }
          break;
        case 'X':
          if (selected) { e.preventDefault(); setQuickDue(null); }
          break;
        case '/':
          e.preventDefault();
          searchInputRef.current?.focus();
          break;
        case 'G':
          e.preventDefault();
          cycleGroupBy();
          break;
        case '[':
          e.preventDefault();
          cycleDueFilter(-1);
          break;
        case ']':
          e.preventDefault();
          cycleDueFilter(1);
          break;
        case '?':
          e.preventDefault();
          setShowShortcuts((v) => !v);
          break;
        case 'Escape':
          setRightOpen(false);
          break;
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selected, rightOpen, flatOrder, dueFilter, groupBy]);

  useEffect(() => {
    if (rightOpen) {
      setPanelPhase('entering');
      const raf1 = requestAnimationFrame(() => {
        const raf2 = requestAnimationFrame(() => setPanelPhase('open'));
        panelRaf2.current = raf2;
      });
      panelRaf1.current = raf1;
      return () => {
        cancelAnimationFrame(panelRaf1.current);
        if (panelRaf2.current) cancelAnimationFrame(panelRaf2.current);
      };
    }
    setPanelPhase('leaving');
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const t = window.setTimeout(() => setPanelPhase('closed'), prefersReducedMotion ? 0 : 240);
    return () => window.clearTimeout(t);
  }, [rightOpen]);

  useEffect(() => {
    if (!isResizingPanel) return;
    const prevCursor = document.body.style.cursor;
    const prevUserSelect = document.body.style.userSelect;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    function handleMove(e: MouseEvent) {
      const next = Math.min(PANEL_MAX_WIDTH, Math.max(PANEL_MIN_WIDTH, window.innerWidth - e.clientX));
      setPanelWidth(next);
    }
    function handleUp() {
      setIsResizingPanel(false);
    }
    window.addEventListener('mousemove', handleMove);
    window.addEventListener('mouseup', handleUp);
    return () => {
      window.removeEventListener('mousemove', handleMove);
      window.removeEventListener('mouseup', handleUp);
      document.body.style.cursor = prevCursor;
      document.body.style.userSelect = prevUserSelect;
    };
  }, [isResizingPanel]);

  useEffect(() => {
    localStorage.setItem('taskPanelWidth', String(panelWidth));
  }, [panelWidth]);

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      {leftOpen && (
      <aside
        style={{
          width: 252, flex: 'none', background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 26, height: 26, borderRadius: 5, background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>command center</span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2, marginBottom: 12 }}>
          {INBOX_ITEMS.map((item) => {
            const active = dueFilter === item.value;
            const count = countForDueFilter(tasks, today, item.value);
            return (
              <div
                key={item.value}
                onClick={() => setDueFilter(item.value)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px',
                  borderRadius: 5, cursor: 'pointer',
                  background: active ? 'var(--accent-soft)' : 'transparent',
                  boxShadow: active ? '0 0 0 1px var(--accent-ring)' : 'none',
                }}
              >
                <Inbox size={16} color={active ? 'var(--accent)' : 'var(--fg-4)'} />
                <span style={{ flex: 1, fontSize: 14, fontWeight: active ? 600 : 400, color: active ? 'var(--fg-1)' : 'var(--fg-3)' }}>
                  {item.label}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: active ? 'var(--accent)' : 'var(--fg-5)' }}>{count}</span>
              </div>
            );
          })}
        </nav>
        <div style={{ padding: '0 20px 8px', fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
          Due window
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2, marginBottom: 12 }}>
          {DUE_WINDOW_ITEMS.map((item) => {
            const active = dueFilter === item.value;
            const count = countForDueFilter(tasks, today, item.value);
            const Icon = item.value === 'day' ? Sun
              : item.value === 'no-due-date' ? HelpCircle
              : item.value === 'recurring' ? Repeat
              : CalendarDays;
            return (
              <div
                key={item.value}
                onClick={() => setDueFilter(item.value)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px',
                  borderRadius: 5, cursor: 'pointer',
                  background: active ? 'var(--accent-soft)' : 'transparent',
                  boxShadow: active ? '0 0 0 1px var(--accent-ring)' : 'none',
                }}
              >
                <Icon size={16} color={active ? 'var(--accent)' : 'var(--fg-4)'} />
                <span style={{ flex: 1, fontSize: 14, fontWeight: active ? 600 : 400, color: active ? 'var(--fg-1)' : 'var(--fg-3)' }}>
                  {item.label}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: active ? 'var(--accent)' : 'var(--fg-5)' }}>{count}</span>
              </div>
            );
          })}
        </nav>
        <div style={{ padding: '8px 20px', display: 'flex', flexDirection: 'column', gap: 4 }}>
          {agents.map((a) => {
            const status = statusFor(a.name);
            return (
              <div
                key={a.name}
                onClick={() => setEditingAgent(a)}
                title={`Edit ${a.name}`}
                style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28, cursor: 'pointer', borderRadius: 4 }}
              >
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
      )}

      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <header style={{ height: 72, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline)', display: 'flex', alignItems: 'center', gap: 16 }}>
          <button
            onClick={() => setLeftOpen((v) => !v)}
            title={leftOpen ? 'Hide sidebar' : 'Show sidebar'}
            style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 4 }}
          >
            {leftOpen ? <PanelLeftClose size={18} color="var(--fg-4)" /> : <PanelLeftOpen size={18} color="var(--fg-4)" />}
          </button>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>
            {ALL_FILTER_ITEMS.find((i) => i.value === dueFilter)?.label ?? 'All tasks'}
          </span>
          <div style={{ flex: 1 }} />
          <button
            onClick={() => setRightOpen((v) => !v)}
            title={rightOpen ? 'Hide inspector' : 'Show inspector'}
            style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 4 }}
          >
            {rightOpen ? <PanelRightClose size={18} color="var(--fg-4)" /> : <PanelRightOpen size={18} color="var(--fg-4)" />}
          </button>
          <button
            onClick={() => setShowShortcuts(true)}
            title="Keyboard shortcuts (?)"
            style={{ background: 'transparent', border: '1px solid var(--hairline)', borderRadius: 5, cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', width: 28, height: 28, color: 'var(--fg-4)', fontSize: 13, fontWeight: 700 }}
          >
            ?
          </button>
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
              ref={searchInputRef}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape') {
                  setSearch('');
                  (e.target as HTMLInputElement).blur();
                }
              }}
              placeholder="Search tasks…"
              style={{ flex: 1, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)', fontSize: 13 }}
            />
          </div>
          {showNewTaskForm ? (
            <NewTaskForm onSubmit={handleAddTask} onCancel={() => setShowNewTaskForm(false)} />
          ) : (
            <Button onClick={() => setShowNewTaskForm(true)}>New task</Button>
          )}
          <VersionBadge />
        </header>
        {error && <div style={{ padding: 16, color: 'var(--danger)' }}>{error}</div>}
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, height: 34, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline-soft)', fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ width: 64 }}></span>
          <span style={{ width: 100 }}>Status</span>
          <span style={{ width: 110 }}>Agent</span>
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
                  data-task-row={t.id}
                  onClick={() => selectTask(t)}
                  style={{
                    display: 'flex', alignItems: 'center', gap: 16, height: 46, padding: '0 24px 0 21px',
                    borderBottom: '1px solid var(--hairline-soft)', cursor: 'pointer',
                    background: selected?.id === t.id ? 'var(--accent-soft)' : 'transparent',
                    borderLeft: selected?.id === t.id ? '3px solid var(--accent)' : '3px solid transparent',
                  }}
                >
                  <span style={{ width: 34, fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-5)' }}>{t.id}</span>
                  <span style={{ width: 78, fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, textTransform: 'uppercase', color: priorityColor[t.priority] }}>
                    {t.priority}
                  </span>
                  <span style={{ width: 64, display: 'flex', alignItems: 'center', gap: 4 }}>
                    {isOverdue(t, today) && (
                      <span title="Overdue" style={{ display: 'flex' }}><AlertTriangle size={13} color="var(--danger)" /></span>
                    )}
                    {hasNoDueDate(t) && (
                      <span title="No due date" style={{ display: 'flex' }}><CalendarOff size={13} color="var(--fg-5)" /></span>
                    )}
                    {missingAgent(t) && (
                      <span title="No agent assigned" style={{ display: 'flex' }}><UserX size={13} color="var(--fg-5)" /></span>
                    )}
                    {t.recurrence && (
                      <span title="Recurring" style={{ display: 'flex' }}><Repeat size={13} color="var(--fg-4)" /></span>
                    )}
                  </span>
                  <span style={{ width: 100 }}>
                    {t.work_status && (
                      <span
                        style={{
                          display: 'inline-block', fontSize: 10, fontWeight: 700, textTransform: 'uppercase',
                          letterSpacing: '0.04em', padding: '2px 8px', borderRadius: 999,
                          color: 'var(--ink)', background: WORK_STATUS_COLOR[t.work_status],
                        }}
                      >
                        {WORK_STATUS_LABEL[t.work_status]}
                      </span>
                    )}
                  </span>
                  <span style={{
                    width: 110, fontSize: 12.5, color: t.agent ? 'var(--fg-3)' : 'var(--fg-5)',
                    whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                  }}>
                    {t.agent ?? '—'}
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

      {panelPhase !== 'closed' && (
      <>
      <div
        className={`task-panel-scrim${panelPhase === 'open' ? ' is-open' : ''}`}
        onClick={() => setRightOpen(false)}
      />
      <aside
        className={`task-panel${panelPhase === 'open' ? ' is-open' : ''}${isResizingPanel ? ' is-resizing' : ''}`}
        style={{ width: panelWidth }}
      >
        <div
          className="task-panel-resize-handle"
          onMouseDown={(e) => {
            e.preventDefault();
            setIsResizingPanel(true);
          }}
          title="Drag to resize"
        />
        <div key={selected?.id ?? 'empty'} className={selected ? 'task-panel-content' : 'task-panel-content task-panel-empty'}>
        {selected ? (
          <>
            <div style={{ display: 'flex', alignItems: 'flex-start', gap: 8 }}>
              <div style={{
                flex: 1, fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 20, color: 'var(--fg-1)', marginBottom: 12,
                textDecoration: selected.status === 'done' ? 'line-through' : 'none',
              }}>
                <EditableField
                  id="field-title"
                  value={selected.title}
                  onSave={(v) => v.trim() && handleEditField(selected.id, { title: v.trim() })}
                  display={selected.title ? <Linkify text={selected.title} /> : undefined}
                />
              </div>
              <button
                onClick={() => setRightOpen(false)}
                title="Close inspector"
                style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 2, flex: 'none' }}
              >
                <X size={16} color="var(--fg-4)" />
              </button>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: 'var(--fg-3)' }}>
              <span>#{selected.id} ·</span>
              <EditableField
                id="field-priority"
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
                <DatePicker
                  id="field-due"
                  value={selected.due_date ?? ''}
                  onSave={(v) => handleEditField(selected.id, { due: v })}
                />
              </FieldRow>
              <FieldRow label="Effort">
                <EditableField
                  id="field-effort"
                  value={selected.effort ?? 'medium'}
                  type="select"
                  options={EFFORT_OPTIONS}
                  onSave={(v) => handleEditField(selected.id, { effort: v })}
                  display={selected.effort ?? '—'}
                />
              </FieldRow>
              <FieldRow label="Work status">
                <EditableField
                  value={selected.work_status ?? ''}
                  type="select"
                  options={WORK_STATUS_OPTIONS}
                  onSave={(v) => handleEditField(selected.id, { work_status: v })}
                  display={selected.work_status ? WORK_STATUS_LABEL[selected.work_status] : undefined}
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
                <AgentPicker
                  id="field-agent"
                  value={selected.agent ?? ''}
                  options={agentOptions}
                  onSave={(v) => handleEditField(selected.id, { agent: v })}
                />
              </FieldRow>
              <FieldRow label="Tags">
                <EditableField
                  id="field-tags"
                  value={selected.tags.join(', ')}
                  placeholder="No tags"
                  onSave={(v) => handleEditField(selected.id, { tags: v })}
                  display={selected.tags.length ? selected.tags.join(', ') : undefined}
                />
              </FieldRow>
              <FieldRow label="Recurrence">
                <EditableField
                  value={selected.recurrence ?? ''}
                  placeholder="e.g. daily, weekly:2, weekly:fri, monthly:2:fri — empty to clear"
                  onSave={(v) => handleEditField(selected.id, { recurrence: v })}
                />
              </FieldRow>
              <FieldRow label="Description">
                <EditableField
                  value={selected.description ?? ''}
                  type="textarea"
                  placeholder="No description"
                  onSave={(v) => handleEditField(selected.id, { description: v })}
                  display={selected.description ? (
                    <span style={{ whiteSpace: 'pre-wrap' }}><Linkify text={selected.description} /></span>
                  ) : undefined}
                />
              </FieldRow>
              <FieldRow label="Instructions">
                <EditableField
                  id="field-instructions"
                  value={selected.instructions ?? ''}
                  type="textarea"
                  placeholder="No instructions"
                  onSave={(v) => handleEditField(selected.id, { instructions: v })}
                  display={selected.instructions ? (
                    <span style={{ whiteSpace: 'pre-wrap' }}><Linkify text={selected.instructions} /></span>
                  ) : undefined}
                />
              </FieldRow>
            </div>

            {(review || selected.work_status === 'waiting-for-review' || selected.work_status === 'changes-requested') && (
              <ReviewPanel
                review={review}
                workStatus={selected.work_status}
                accentColor={selected.work_status ? WORK_STATUS_COLOR[selected.work_status] : undefined}
                onSendFeedback={handleSendFeedback}
              />
            )}

            {(question || selected.work_status === 'needs-input') && (
              <QuestionPanel
                question={question}
                awaitingAnswer={selected.work_status === 'needs-input'}
                accentColor={selected.work_status ? WORK_STATUS_COLOR[selected.work_status] : undefined}
                onSendAnswer={handleSendAnswer}
              />
            )}

            <NotesSection
              notes={visibleNotes}
              onCreate={handleCreateNote}
              onOpen={handleOpenNote}
              onUnlink={handleUnlinkNote}
            />

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
        </div>
      </aside>
      </>
      )}

      {editingAgent && (
        <AgentEditor agent={editingAgent} onClose={() => setEditingAgent(null)} />
      )}

      {showShortcuts && (
        <div
          onClick={() => setShowShortcuts(false)}
          style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 }}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style={{ background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8, padding: 24, width: 420, color: 'var(--fg-1)' }}
          >
            <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16 }}>
              <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 16 }}>Keyboard shortcuts</span>
              <button onClick={() => setShowShortcuts(false)} style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex' }}>
                <X size={16} color="var(--fg-4)" />
              </button>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6, fontSize: 13 }}>
              {[
                ['j / k, ↓ / ↑', 'Move selection'],
                ['Enter / Space', 'Toggle done'],
                ['a', 'New task'],
                ['e', 'Edit title'],
                ['d', 'Edit due date'],
                ['p', 'Edit priority'],
                ['t', 'Edit tags'],
                ['A', 'Edit agent'],
                ['E', 'Edit effort'],
                ['g', 'Open first linked note'],
                ['T', 'Due: today'],
                ['N', 'Due: tomorrow'],
                ['W', 'Due: +1 week'],
                ['M', 'Due: +1 month'],
                ['Q', 'Due: +1 quarter'],
                ['Y', 'Due: +1 year'],
                ['X', 'Clear due date'],
                ['/', 'Focus search'],
                ['G', 'Cycle group-by'],
                ['[ / ]', 'Cycle due window'],
                ['Esc', 'Close inspector'],
                ['?', 'Toggle this help'],
              ].map(([key, desc]) => (
                <div key={key} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <span style={{ width: 130, flex: 'none', fontFamily: 'var(--font-mono)', color: 'var(--accent)' }}>{key}</span>
                  <span style={{ color: 'var(--fg-3)' }}>{desc}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
