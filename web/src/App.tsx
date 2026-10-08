import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import './tokens.css';
import './taskPanel.css';
import './layout.css';
import { Terminal, Search, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, Menu, ChevronLeft, Plus } from 'lucide-react';
import { Button } from './components/Button';
import { NewTaskForm } from './components/NewTaskForm';
import { fetchTasks, fetchAgents, addTask, editTask, markDone, reopenTask, deleteTask, fetchTaskNotes, createTaskNote, openNote, saveNote, unlinkTaskNote, fetchTaskReview, postTaskFeedback, fetchTaskQuestion, postTaskAnswer } from './api';
import { AgentEditor } from './components/AgentEditor';
import { SettingsFooter } from './components/SettingsFooter';
import { TaskDetail } from './components/TaskDetail';
import { TaskRow } from './components/TaskRow';
import { MobileTaskRow } from './components/MobileTaskRow';
import { MobileDrawer } from './components/MobileDrawer';
import { MobileNewTaskSheet } from './components/MobileNewTaskSheet';
import { CommandPalette, type PaletteCommand } from './components/CommandPalette';
import { COL, sectionLabelStyle } from './styles';
import type { Task, AgentProfile, Note } from './types';
import { startOfToday } from './dueWindow';
import { statusFor, statusColor } from './mockAgentStatus';
import { addDays, addMonths, formatLocalDate } from './taskFormat';
import {
  ALL_FILTER_ITEMS, DUE_WINDOW_ITEMS, GROUP_BY_OPTIONS, INBOX_ITEM, NEXT_ITEM,
  byDueThenPriority, countForFilter, groupTasks, matchesFilter, type DueFilter, type FilterItem, type GroupBy,
} from './filters';
import { useMediaQuery } from './useMediaQuery';

const PANEL_MIN_WIDTH = 352;
const PANEL_MAX_WIDTH = 880;
const PANEL_DEFAULT_WIDTH = 440;

/** Below this width the app switches to the phone layout. */
const MOBILE_QUERY = '(max-width: 720px)';

/** Quick due-date shortcuts: key → [label, offset from today, or null to clear]. */
const QUICK_DUE: [string, string, ((today: Date) => Date) | null][] = [
  ['T', 'Due today', (d) => d],
  ['N', 'Due tomorrow', (d) => addDays(d, 1)],
  ['W', 'Due +1 week', (d) => addDays(d, 7)],
  ['M', 'Due +1 month', (d) => addMonths(d, 1)],
  ['Q', 'Due +1 quarter', (d) => addMonths(d, 3)],
  ['Y', 'Due +1 year', (d) => addMonths(d, 12)],
  ['X', 'Clear due date', null],
];

/** Inspector field shortcuts: key → [label, element id to click]. */
const FIELD_KEYS: [string, string, string][] = [
  ['e', 'Edit title', 'field-title'],
  ['d', 'Edit due date', 'field-due'],
  ['p', 'Edit priority', 'field-priority'],
  ['t', 'Edit tags', 'field-tags'],
  ['A', 'Edit agent', 'field-agent'],
  ['E', 'Edit effort', 'field-effort'],
];

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target.isContentEditable;
}

function countLabel(n: number): string {
  return `${n} ${n === 1 ? 'task' : 'tasks'}`;
}

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [selected, setSelected] = useState<Task | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showNewTaskForm, setShowNewTaskForm] = useState(false);
  const [groupByChoice, setGroupBy] = useState<GroupBy>('agent');
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
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentProfile | null>(null);
  // Phone-only UI state.
  const isMobile = useMediaQuery(MOBILE_QUERY);
  const [mobileSearch, setMobileSearch] = useState<string | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [newSheetOpen, setNewSheetOpen] = useState(false);
  const [swipeOpenId, setSwipeOpenId] = useState<number | null>(null);

  const today = useMemo(() => startOfToday(), []);
  const agentOptions = useMemo(
    () => [
      { value: '', label: 'Unassigned' },
      ...agents.map((a) => ({ value: a.name, label: a.name })),
      { value: 'human', label: 'human' },
    ],
    [agents],
  );
  const panelRaf1 = useRef<number>(0);
  const panelRaf2 = useRef<number>(0);
  const reviewSlug = selected ? `task-${selected.id}-review-thread` : null;
  const questionSlug = selected ? `task-${selected.id}-question` : null;
  const visibleNotes = taskNotes.filter((n) => n.slug !== reviewSlug && n.slug !== questionSlug);
  // Next is a single queue ordered by due date then priority, so it ignores the chosen grouping.
  const groupBy: GroupBy = dueFilter === 'next' ? 'none' : groupByChoice;
  const filterItem: FilterItem = ALL_FILTER_ITEMS.find((i) => i.value === dueFilter) ?? INBOX_ITEM;

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
    if (!selected || isMobile) return;
    document.querySelector(`[data-task-row="${selected.id}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selected?.id, isMobile]);

  async function handleAddTask(input: { title: string; due?: string; agent?: string }) {
    try {
      const created = await addTask(input);
      setTasks((prev) => [...prev, created]);
      setShowNewTaskForm(false);
      setNewSheetOpen(false);
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
      if (selected?.id !== task.id) return;
      // On desktop, completing the selected task moves on to the next one so you
      // can keep triaging; on a phone you stay on the task you just completed.
      const nextSelection = isMobile || currentIndex === -1
        ? completed
        : flatOrder[currentIndex + 1] ?? flatOrder[currentIndex - 1] ?? completed;
      if (isMobile) setSelected(nextSelection);
      else selectTask(nextSelection);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleReopen(task: Task) {
    try {
      const updated = await reopenTask(task.id);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
      setSelected((prev) => (prev?.id === updated.id ? updated : prev));
    } catch (e) {
      setError(String(e));
    }
  }

  function toggleDone(task: Task) {
    if (task.status === 'done') handleReopen(task);
    else handleMarkDone(task);
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
      setSelected((prev) => (prev?.id === updated.id ? updated : prev));
    } catch (e) {
      setError(String(e));
    }
  }

  function setQuickDue(task: Task, date: Date | null) {
    handleEditField(task.id, { due: date ? formatLocalDate(date) : '' });
  }

  function updateTaskNoteSlugs(taskId: number, notes: string[]) {
    setTasks((prev) => prev.map((t) => (t.id === taskId ? { ...t, notes } : t)));
    setSelected((prev) => (prev && prev.id === taskId ? { ...prev, notes } : prev));
  }

  async function handleCreateNote(title: string): Promise<string | undefined> {
    if (!selected) return undefined;
    try {
      const note = await createTaskNote(selected.id, title);
      setTaskNotes((prev) => [...prev, note]);
      updateTaskNoteSlugs(selected.id, [...(selected.notes ?? []), note.slug]);
      return note.slug;
    } catch (e) {
      setError(String(e));
      return undefined;
    }
  }

  // Throws on failure so the note editor can show the error next to its Save button.
  async function handleSaveNote(slug: string, body: string) {
    const saved = await saveNote(slug, { body });
    setTaskNotes((prev) => prev.map((n) => (n.slug === slug ? saved : n)));
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

  const query = isMobile ? (mobileSearch ?? '') : '';
  const filteredTasks = useMemo(() => {
    let result = tasks.filter((t) => matchesFilter(t, today, dueFilter));
    const q = query.trim().toLowerCase();
    if (q) {
      result = result.filter((t) => t.title.toLowerCase().includes(q));
    }
    return result;
  }, [tasks, dueFilter, query, today]);

  const grouped = useMemo(() => groupTasks(filteredTasks, groupBy, dueFilter === 'next' ? byDueThenPriority : undefined), [filteredTasks, groupBy, dueFilter]);
  const flatOrder = useMemo(() => [...grouped.values()].flat(), [grouped]);

  function moveSelection(delta: number) {
    if (flatOrder.length === 0) return;
    const currentIndex = selected ? flatOrder.findIndex((t) => t.id === selected.id) : -1;
    const nextIndex = currentIndex === -1 ? 0 : Math.min(Math.max(currentIndex + delta, 0), flatOrder.length - 1);
    selectTask(flatOrder[nextIndex]);
  }

  function cycleDueFilter(delta: number) {
    const idx = DUE_WINDOW_ITEMS.findIndex((i) => i.value === dueFilter);
    const next = (idx + delta + DUE_WINDOW_ITEMS.length) % DUE_WINDOW_ITEMS.length;
    setDueFilter(DUE_WINDOW_ITEMS[next].value);
  }

  function cycleGroupBy() {
    const idx = GROUP_BY_OPTIONS.findIndex((o) => o.value === groupBy);
    setGroupBy(GROUP_BY_OPTIONS[(idx + 1) % GROUP_BY_OPTIONS.length].value);
  }

  function clickField(id: string) {
    if (!selected) return;
    if (rightOpen) {
      document.getElementById(id)?.click();
      return;
    }
    // Open the inspector first, then click once it has rendered.
    setRightOpen(true);
    window.setTimeout(() => document.getElementById(id)?.click(), 50);
  }

  function openFirstNote() {
    if (selected?.notes?.length) handleOpenNote(selected.notes[0]);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      setPaletteOpen((v) => !v);
      return;
    }
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (paletteOpen || isTypingTarget(e.target)) return;

    const quick = QUICK_DUE.find(([key]) => key === e.key);
    if (quick) {
      if (selected) {
        e.preventDefault();
        setQuickDue(selected, quick[2] ? quick[2](today) : null);
      }
      return;
    }
    const field = FIELD_KEYS.find(([key]) => key === e.key);
    if (field) {
      if (selected && rightOpen) {
        if (e.key === 'A') e.preventDefault();
        document.getElementById(field[2])?.click();
      }
      return;
    }

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
        if (isMobile) setNewSheetOpen(true);
        else setShowNewTaskForm(true);
        break;
      case 'g':
        openFirstNote();
        break;
      case '/':
      case '?':
        e.preventDefault();
        setPaletteOpen(true);
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
      case 'Escape':
        if (isMobile) setSelected(null);
        else setRightOpen(false);
        break;
    }
  }

  // Re-subscribed every render so the handler always sees the latest state.
  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  });

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

  function paletteCommands(): PaletteCommand[] {
    const close = () => setPaletteOpen(false);
    const cmd = (label: string, kbd: string, fn: () => void): PaletteCommand => ({ label, kbd, run: () => { close(); fn(); } });
    return [
      cmd('New task', 'a', () => setShowNewTaskForm(true)),
      ...ALL_FILTER_ITEMS.map((f) => cmd(`Go to ${f.label}`, f.value === 'inbox' ? '' : '[ ]', () => setDueFilter(f.value))),
      ...GROUP_BY_OPTIONS.map((g) => cmd(`Group by ${g.label.toLowerCase()}`, 'G', () => setGroupBy(g.value))),
      cmd('Next task', 'j ↓', () => moveSelection(1)),
      cmd('Previous task', 'k ↑', () => moveSelection(-1)),
      ...(selected
        ? [
            cmd(`Selected: ${selected.status === 'done' ? 'reopen' : 'mark done'}`, 'Enter', () => toggleDone(selected)),
            ...QUICK_DUE.map(([key, label, fn]) => cmd(`Selected: ${label.toLowerCase()}`, key, () => setQuickDue(selected, fn ? fn(today) : null))),
            ...FIELD_KEYS.map(([key, label, id]) => cmd(`Selected: ${label.toLowerCase()}`, key, () => clickField(id))),
            ...(selected.notes?.length ? [cmd('Selected: open first linked note', 'g', openFirstNote)] : []),
          ]
        : []),
      cmd('Toggle sidebar', '', () => setLeftOpen((v) => !v)),
      cmd(rightOpen ? 'Close inspector' : 'Open inspector', rightOpen ? 'Esc' : '', () => setRightOpen((v) => !v)),
    ];
  }

  const detailProps = selected && {
    task: selected,
    today,
    agentOptions,
    review,
    question,
    notes: visibleNotes,
    onEdit: (changes: Parameters<typeof editTask>[1]) => handleEditField(selected.id, changes),
    onSendFeedback: handleSendFeedback,
    onSendAnswer: handleSendAnswer,
    onCreateNote: handleCreateNote,
    onOpenNote: handleOpenNote,
    onSaveNote: handleSaveNote,
    onUnlinkNote: handleUnlinkNote,
  };

  function groupDot(name: string) {
    if (groupBy !== 'agent' || !agents.some((a) => a.name === name)) return null;
    return statusFor(name).state;
  }

  if (isMobile) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100dvh', overflow: 'hidden', paddingTop: 'env(safe-area-inset-top, 0px)' }}>
        {mobileSearch === null ? (
          <div style={{ height: 56, flex: 'none', display: 'flex', alignItems: 'center', gap: 4, padding: '0 6px' }}>
            <MobileIconButton label="Menu" onClick={() => setDrawerOpen(true)}><Menu size={22} color="var(--fg-3)" /></MobileIconButton>
            <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>{filterItem.label}</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--accent)', paddingTop: 4, marginLeft: 4 }}>{filteredTasks.length}</span>
            <div style={{ flex: 1 }} />
            <MobileIconButton label="Search" onClick={() => setMobileSearch('')}><Search size={20} color="var(--fg-3)" /></MobileIconButton>
          </div>
        ) : (
          <div style={{ height: 56, flex: 'none', display: 'flex', alignItems: 'center', gap: 8, padding: '0 6px 0 16px' }}>
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 8, height: 40, padding: '0 12px', border: '1px solid var(--accent)', borderRadius: 5, background: 'var(--ink-2)' }}>
              <Search size={16} color="var(--fg-4)" />
              <input
                autoFocus
                value={mobileSearch}
                onChange={(e) => setMobileSearch(e.target.value)}
                placeholder="Search tasks…"
                style={{ flex: 1, minWidth: 0, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)', fontSize: 16 }}
              />
            </div>
            <button
              onClick={() => setMobileSearch(null)}
              style={{ height: 44, padding: '0 10px', background: 'transparent', border: 'none', color: 'var(--accent)', fontSize: 15, fontWeight: 600, cursor: 'pointer' }}
            >
              Cancel
            </button>
          </div>
        )}

        <div className="no-scrollbar" style={{ flex: 'none', display: 'flex', gap: 8, overflowX: 'auto', padding: '4px 16px 12px' }}>
          {ALL_FILTER_ITEMS.map((item) => {
            const active = dueFilter === item.value;
            return (
              <button
                key={item.value}
                onClick={() => setDueFilter(item.value)}
                style={{
                  flex: 'none', display: 'flex', alignItems: 'center', gap: 6, height: 34, padding: '0 12px', borderRadius: 999, cursor: 'pointer', whiteSpace: 'nowrap',
                  border: `1px solid ${active ? 'var(--accent-ring)' : 'var(--hairline)'}`,
                  background: active ? 'var(--accent-soft)' : 'transparent',
                }}
              >
                <span style={{ fontSize: 13.5, fontWeight: active ? 600 : 400, color: active ? 'var(--fg-1)' : 'var(--fg-3)' }}>{item.label}</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: active ? 'var(--accent)' : 'var(--fg-5)' }}>
                  {countForFilter(tasks, today, item.value)}
                </span>
              </button>
            );
          })}
        </div>

        {error && <div style={{ padding: '8px 16px', color: 'var(--danger)', fontSize: 13 }}>{error}</div>}

        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', borderTop: '1px solid var(--hairline)', paddingBottom: 96 }}>
          {[...grouped.entries()].map(([groupName, groupTasksList]) => {
            const state = groupDot(groupName);
            return (
              <div key={groupName || 'all'}>
                {groupBy !== 'none' && (
                  <div style={{ position: 'sticky', top: 0, zIndex: 2, display: 'flex', alignItems: 'center', gap: 8, height: 36, padding: '0 16px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                    {state && <span style={{ width: 7, height: 7, borderRadius: 999, background: statusColor(state) }} />}
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600, textTransform: groupBy === 'priority' ? 'uppercase' : 'none' }}>{groupName}</span>
                    <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{countLabel(groupTasksList.length)}</span>
                    <span style={{ flex: 1 }} />
                    {state && state !== 'idle' && (
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: statusColor(state) }}>{state}</span>
                    )}
                  </div>
                )}
                {groupTasksList.map((t) => (
                  <MobileTaskRow
                    key={t.id}
                    task={t}
                    today={today}
                    showAgent={groupBy !== 'agent'}
                    swipeOpen={swipeOpenId === t.id}
                    onSwipeOpenChange={(open) => setSwipeOpenId(open ? t.id : null)}
                    onSelect={() => { setSwipeOpenId(null); setSelected(t); }}
                    onToggleDone={() => toggleDone(t)}
                    onQuickDue={(days) => setQuickDue(t, addDays(today, days))}
                  />
                ))}
              </div>
            );
          })}
          {filteredTasks.length === 0 ? (
            <EmptyState title={query ? 'No matches.' : filterItem.empty[0]} body={query ? '' : filterItem.empty[1]} padding="72px 32px" />
          ) : (
            <div style={{ padding: 16, textAlign: 'center', fontSize: 12, color: 'var(--fg-5)' }}>Swipe right to complete · left to set a due date</div>
          )}
        </div>

        <button
          onClick={() => setNewSheetOpen(true)}
          aria-label="New task"
          style={{
            position: 'fixed', right: 20, bottom: 'calc(20px + env(safe-area-inset-bottom, 16px))', width: 56, height: 56, borderRadius: 16,
            background: 'var(--accent)', border: 'none', display: 'flex', alignItems: 'center', justifyContent: 'center',
            cursor: 'pointer', boxShadow: '0 8px 24px rgba(0, 0, 0, 0.35)', zIndex: 5,
          }}
        >
          <Plus size={24} strokeWidth={2.5} color="var(--ink)" />
        </button>

        {selected && detailProps && (
          <div className="mobile-detail" style={{ paddingTop: 'env(safe-area-inset-top, 0px)' }}>
            <div style={{ height: 52, flex: 'none', display: 'flex', alignItems: 'center', gap: 4, padding: '0 6px', borderBottom: '1px solid var(--hairline)' }}>
              <button
                onClick={() => setSelected(null)}
                style={{ height: 44, display: 'flex', alignItems: 'center', gap: 2, padding: '0 8px 0 4px', background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--accent)', fontSize: 15, fontWeight: 600 }}
              >
                <ChevronLeft size={22} color="var(--accent)" />
                {filterItem.label}
              </button>
              <div style={{ flex: 1 }} />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--fg-4)', paddingRight: 14 }}>#{selected.id}</span>
            </div>
            <div key={selected.id} style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: '16px 20px 24px', color: 'var(--fg-5)' }}>
              <TaskDetail {...detailProps} mobile showId={false} />
            </div>
            <div style={{ flex: 'none', display: 'flex', gap: 10, padding: '12px 16px calc(12px + env(safe-area-inset-bottom, 22px))', borderTop: '1px solid var(--hairline)', background: 'var(--ink-2)' }}>
              <MobileActionButton grow primary={selected.status === 'open'} onClick={() => toggleDone(selected)}>
                {selected.status === 'open' ? 'Mark done' : 'Reopen'}
              </MobileActionButton>
              <MobileActionButton onClick={() => handleDelete(selected)}>Delete</MobileActionButton>
            </div>
          </div>
        )}

        {drawerOpen && (
          <MobileDrawer
            tasks={tasks}
            today={today}
            agents={agents}
            filter={dueFilter}
            groupBy={groupBy}
            onFilter={(f) => { setDueFilter(f); setDrawerOpen(false); }}
            onGroupBy={setGroupBy}
            onClose={() => setDrawerOpen(false)}
          />
        )}

        {newSheetOpen && (
          <MobileNewTaskSheet
            today={today}
            agentNames={[...agents.map((a) => a.name), 'human']}
            onSubmit={handleAddTask}
            onClose={() => setNewSheetOpen(false)}
          />
        )}
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      {leftOpen && (
      <aside
        style={{
          width: 252, flex: 'none', background: 'var(--ink-2)', overflowY: 'auto',
          borderRight: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column', padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 26, height: 26, borderRadius: 5, background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>command center</span>
        </div>
        <SidebarNav items={[NEXT_ITEM, INBOX_ITEM]} active={dueFilter} tasks={tasks} today={today} onPick={setDueFilter} />
        <div style={{ ...sectionLabelStyle, padding: '0 20px 8px' }}>Due window</div>
        <SidebarNav items={DUE_WINDOW_ITEMS} active={dueFilter} tasks={tasks} today={today} onPick={setDueFilter} />
        {agents.length > 0 && <div style={{ ...sectionLabelStyle, padding: '4px 20px 8px' }}>Agents</div>}
        <div style={{ padding: '0 20px', display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 12 }}>
          {agents.map((a) => {
            const status = statusFor(a.name);
            return (
              <div
                key={a.name}
                onClick={() => setEditingAgent(a)}
                title={status.detail ? `${status.detail} — click to edit ${a.name}` : `Edit ${a.name}`}
                style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28, cursor: 'pointer', borderRadius: 4 }}
              >
                <span style={{ width: 7, height: 7, borderRadius: 999, background: statusColor(status.state) }} />
                <span style={{ flex: 1, minWidth: 0, fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{a.name}</span>
                {status.state !== 'idle' && (
                  <span style={{ fontSize: 11.5, color: statusColor(status.state) }}>{status.state}</span>
                )}
              </div>
            );
          })}
        </div>
        <SettingsFooter />
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
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22, whiteSpace: 'nowrap' }}>{filterItem.label}</span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--accent)', paddingTop: 4 }}>{filteredTasks.length}</span>
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
          <button
            onClick={() => setPaletteOpen(true)}
            title="Search tasks or run a command (⌘K)"
            style={{
              display: 'flex', alignItems: 'center', gap: 8, height: 34, padding: '0 6px 0 12px', border: '1px solid var(--hairline)',
              borderRadius: 5, background: 'var(--ink-2)', width: 280, minWidth: 0, flexShrink: 1, cursor: 'pointer',
            }}
          >
            <Search size={14} color="var(--fg-4)" style={{ flex: 'none' }} />
            <span style={{ flex: 1, minWidth: 0, textAlign: 'left', color: 'var(--fg-5)', fontSize: 13, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              Search or run a command…
            </span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--fg-3)', border: '1px solid var(--hairline)', borderRadius: 4, padding: '2px 6px' }}>⌘K</span>
          </button>
          {showNewTaskForm ? (
            <NewTaskForm onSubmit={(title) => handleAddTask({ title })} onCancel={() => setShowNewTaskForm(false)} />
          ) : (
            <Button onClick={() => setShowNewTaskForm(true)}>New task</Button>
          )}
          <button
            onClick={() => setRightOpen((v) => !v)}
            title={rightOpen ? 'Hide inspector' : 'Show inspector'}
            style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 4 }}
          >
            {rightOpen ? <PanelRightClose size={18} color="var(--fg-4)" /> : <PanelRightOpen size={18} color="var(--fg-4)" />}
          </button>
        </header>
        {error && <div style={{ padding: 16, color: 'var(--danger)' }}>{error}</div>}
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, height: 34, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline-soft)', fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
          <span style={{ width: COL.check, flex: 'none' }} />
          <span style={{ width: COL.id, flex: 'none' }}>ID</span>
          <span style={{ width: COL.priority, flex: 'none' }}>Priority</span>
          <span style={{ width: COL.flags, flex: 'none' }} />
          <span style={{ width: COL.status, flex: 'none' }}>Status</span>
          <span style={{ width: COL.agent, flex: 'none' }}>Agent</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: COL.due, flex: 'none' }}>Due</span>
          <span style={{ width: COL.effort, flex: 'none' }}>Effort</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
          {[...grouped.entries()].map(([groupName, groupTasksList]) => {
            const state = groupDot(groupName);
            return (
              <div key={groupName || 'all'}>
                {groupBy !== 'none' && (
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 38, padding: '0 24px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                    {state && <span style={{ width: 7, height: 7, borderRadius: 999, background: statusColor(state) }} />}
                    <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600, textTransform: groupBy === 'priority' ? 'uppercase' : 'none' }}>
                      {groupName}
                    </span>
                    <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{countLabel(groupTasksList.length)}</span>
                  </div>
                )}
                {groupTasksList.map((t) => (
                  <TaskRow
                    key={t.id}
                    task={t}
                    today={today}
                    selected={selected?.id === t.id}
                    onSelect={() => selectTask(t)}
                    onToggleDone={() => toggleDone(t)}
                  />
                ))}
              </div>
            );
          })}
          {filteredTasks.length === 0 && (
            <EmptyState title={filterItem.empty[0]} body={filterItem.empty[1]} padding="120px 24px" onNewTask={() => setShowNewTaskForm(true)} />
          )}
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
        {selected && detailProps ? (
          <TaskDetail
            {...detailProps}
            onClose={() => setRightOpen(false)}
            actions={
              <>
                {selected.status === 'open' ? (
                  <Button size="sm" onClick={() => handleMarkDone(selected)}>Mark done</Button>
                ) : (
                  <Button size="sm" variant="secondary" onClick={() => handleReopen(selected)}>Reopen</Button>
                )}
                <Button size="sm" variant="secondary" onClick={() => handleDelete(selected)}>Delete</Button>
              </>
            }
          />
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

      {paletteOpen && (
        <CommandPalette
          tasks={tasks}
          commands={paletteCommands()}
          onPickTask={(t) => { setPaletteOpen(false); selectTask(t); }}
          onClose={() => setPaletteOpen(false)}
        />
      )}
    </div>
  );
}

function SidebarNav({ items, active, tasks, today, onPick }: {
  items: FilterItem[];
  active: DueFilter;
  tasks: Task[];
  today: Date;
  onPick: (f: DueFilter) => void;
}) {
  return (
    <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2, marginBottom: 12 }}>
      {items.map((item) => {
        const on = active === item.value;
        const Icon = item.icon;
        return (
          <div
            key={item.value}
            onClick={() => onPick(item.value)}
            style={{
              display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px', borderRadius: 5, cursor: 'pointer',
              background: on ? 'var(--accent-soft)' : 'transparent',
              boxShadow: on ? '0 0 0 1px var(--accent-ring)' : 'none',
            }}
          >
            <Icon size={16} color={on ? 'var(--accent)' : 'var(--fg-4)'} />
            <span style={{ flex: 1, fontSize: 14, fontWeight: on ? 600 : 400, color: on ? 'var(--fg-1)' : 'var(--fg-3)' }}>{item.label}</span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: on ? 'var(--accent)' : 'var(--fg-5)' }}>
              {countForFilter(tasks, today, item.value)}
            </span>
          </div>
        );
      })}
    </nav>
  );
}

function EmptyState({ title, body, padding, onNewTask }: { title: string; body: string; padding: string; onNewTask?: () => void }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: onNewTask ? 10 : 8, padding, textAlign: 'center' }}>
      <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: onNewTask ? 18 : 17 }}>{title}</div>
      {body && <div style={{ fontSize: 14, color: 'var(--fg-4)', maxWidth: 360 }}>{body}</div>}
      {onNewTask && (
        <div style={{ marginTop: 6 }}>
          <Button variant="secondary" onClick={onNewTask}>New task</Button>
        </div>
      )}
    </div>
  );
}

function MobileIconButton({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      style={{ width: 44, height: 44, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'transparent', border: 'none', cursor: 'pointer' }}
    >
      {children}
    </button>
  );
}

function MobileActionButton({ primary, grow, onClick, children }: { primary?: boolean; grow?: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      onClick={onClick}
      style={{
        flex: grow ? 1 : 'none', height: 48, padding: '0 20px', fontSize: 16, fontWeight: 600, borderRadius: 5, cursor: 'pointer',
        background: primary ? 'var(--accent)' : 'transparent', color: primary ? 'var(--ink)' : 'var(--fg-1)',
        border: `1px solid ${primary ? 'transparent' : 'var(--hairline)'}`,
      }}
    >
      {children}
    </button>
  );
}
