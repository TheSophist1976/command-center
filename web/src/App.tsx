import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import './tokens.css';
import './taskPanel.css';
import './layout.css';
import { Search, PanelRightClose, PanelRightOpen, ChevronLeft, ChevronUp, ChevronDown, Plus } from 'lucide-react';
import { Button } from './components/Button';
import { NewTaskForm } from './components/NewTaskForm';
import { fetchTask, fetchTasks, fetchAgents, fetchNames, fetchProjectDetails, updateProject, createName, deleteName, addTask, editTask, markDone, reopenTask, deleteTask, fetchTaskNotes, createTaskNote, openNote, saveNote, unlinkTaskNote, fetchTaskThread, postTaskThread } from './api';
import type { NameKind } from './api';
import { AgentEditor } from './components/AgentEditor';
import { ServicesView } from './components/ServicesView';
import { fetchServices } from './api';
import { SettingsDialog } from './components/SettingsDialog';
import { TaskDetail } from './components/TaskDetail';
import { TaskRow } from './components/TaskRow';
import { MobileTaskRow } from './components/MobileTaskRow';
import { MobileNewTaskSheet } from './components/MobileNewTaskSheet';
import { CommandPalette, type PaletteCommand } from './components/CommandPalette';
import { ViewToggle, type View } from './components/ViewToggle';
import { FilterPills } from './components/FilterPills';
import { NextQueue } from './components/NextQueue';
import { StatusStrip, MobileStatusBar } from './components/StatusStrip';
import { COL } from './styles';
import type { Task, AgentProfile, Note, Service, Thread, ThreadSendKind, Project, ProjectPatch } from './types';
import { indexProjects } from './projects';
import { ProjectDot } from './components/ProjectDot';
import { startOfToday } from './dueWindow';
import { agentActivity, serviceSummary } from './status';
import { bandOf, buildQueue, needsYouCount, nextSelection, queueOrder } from './queue';
import { addDays, addMonths, formatLocalDate } from './taskFormat';
import {
  ALL_TASKS_FILTERS, GROUP_BY_OPTIONS,
  groupTasks, matchesFilter, needsHumanAction, type DueFilter, type FilterItem, type GroupBy,
} from './filters';
import { useMediaQuery } from './useMediaQuery';
import { loadDeferrals, loadSettings, saveDeferrals, saveSettings, type Deferrals, type Settings } from './settings';
import { diffNotifications, loadSnapshot, saveSnapshot, showNotification, snapshotOf } from './notifications';

const VIEW_KEY = 'ccView';
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
  const [allProjects, setAllProjects] = useState<string[]>([]);
  const [projectList, setProjectList] = useState<Project[]>([]);
  const projectMeta = useMemo(() => indexProjects(projectList), [projectList]);
  const [allTags, setAllTags] = useState<string[]>([]);
  // False until the first fetch, so the empty initial list is not taken as the notification baseline.
  const [tasksLoaded, setTasksLoaded] = useState(false);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [selected, setSelected] = useState<Task | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showNewTaskForm, setShowNewTaskForm] = useState(false);
  const [groupBy, setGroupBy] = useState<GroupBy>('agent');
  // Next is the home view: the queue of things that need you. All tasks is the full list behind a toggle.
  const [view, setViewState] = useState<View>(() => (localStorage.getItem(VIEW_KEY) === 'all' ? 'all' : 'next'));
  const [dueFilter, setDueFilter] = useState<DueFilter>('all-tasks');
  // The "Agents working" band is awareness only, so it starts folded away.
  const [workingExpanded, setWorkingExpanded] = useState(false);
  const [rightOpen, setRightOpen] = useState(true);
  const [panelPhase, setPanelPhase] = useState<'closed' | 'entering' | 'open' | 'leaving'>('open');
  const [panelWidth, setPanelWidth] = useState(() => {
    const stored = Number(localStorage.getItem('taskPanelWidth'));
    return Number.isFinite(stored) && stored >= PANEL_MIN_WIDTH && stored <= PANEL_MAX_WIDTH ? stored : PANEL_DEFAULT_WIDTH;
  });
  const [isResizingPanel, setIsResizingPanel] = useState(false);
  const [taskNotes, setTaskNotes] = useState<Note[]>([]);
  const [thread, setThread] = useState<Thread | null>(null);
  const threadReq = useRef(0);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<AgentProfile | null>(null);
  const [servicesOpen, setServicesOpen] = useState(false);
  const [services, setServices] = useState<Service[]>([]);
  const [settings, setSettings] = useState<Settings>(loadSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [deferrals, setDeferrals] = useState<Deferrals>(() => loadDeferrals(Date.now()));
  // Ticks so a deferral expires (and the task returns to its place) without a reload.
  const [now, setNow] = useState(() => Date.now());
  // Phone-only UI state.
  const isMobile = useMediaQuery(MOBILE_QUERY);
  const [mobileSearch, setMobileSearch] = useState<string | null>(null);
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
  // The conversation has its own panel; the old review/question notes are folded into it.
  const threadSlugs = selected
    ? [`task-${selected.id}-thread`, `task-${selected.id}-review-thread`, `task-${selected.id}-question`]
    : [];
  const visibleNotes = taskNotes.filter((n) => !threadSlugs.includes(n.slug));
  const filterItem: FilterItem = ALL_TASKS_FILTERS.find((i) => i.value === dueFilter) ?? ALL_TASKS_FILTERS[0];
  // The docked pane (Next) is always visible; the All tasks inspector slides in and out.
  const inspectorVisible = view === 'next' || rightOpen;

  function setView(next: View) {
    setViewState(next);
    localStorage.setItem(VIEW_KEY, next);
  }

  function selectTask(task: Task) {
    setSelected(task);
    setRightOpen(true);
  }

  // Live updates: task_server broadcasts over SSE whenever tasks.db changes
  // on disk, from any writer (this tab, another tab, the CLI, an agent).
  // Refetch everything and re-sync `selected` by id rather than trying to
  // diff/merge — simplest correct approach at this app's scale.
  // A successful refetch also clears any earlier error: fetches fail transiently
  // (server restart, phone waking from sleep) and the error would otherwise stick
  // on screen until a reload. Changes missed while disconnected are picked up by
  // refetching when the stream (re)opens and when the tab becomes visible again.
  useEffect(() => {
    function refresh() {
      fetchServices()
        .then(setServices)
        .catch(() => {});
      Promise.all([fetchTasks(), fetchAgents(), fetchNames('projects'), fetchNames('tags'), fetchProjectDetails()])
        .then(([t, a, p, tg, pd]) => {
          setTasks(t);
          setAllProjects(p);
          setProjectList(pd);
          setAllTags(tg);
          setTasksLoaded(true);
          setAgents(a);
          setSelected((prev) => (prev ? t.find((task) => task.id === prev.id) ?? null : prev));
          setError(null);
        })
        .catch((e) => setError(String(e)));
    }
    function onVisibilityChange() {
      if (document.visibilityState === 'visible') refresh();
    }
    refresh();
    const source = new EventSource('/api/events');
    let opened = false;
    source.onopen = () => {
      if (opened) refresh();
      opened = true;
    };
    source.onmessage = refresh;
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      source.close();
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, []);

  // Notify about tasks that are new, newly need the human, or newly overdue since the last check.
  // Runs on every refresh and on the clock tick (so tasks turn overdue at midnight). The snapshot is
  // kept up to date even while notifications are off, so turning them on does not replay old changes.
  useEffect(() => {
    if (!tasksLoaded) return;
    const day = startOfToday();
    const prev = loadSnapshot();
    if (prev && settings.notifications) {
      for (const n of diffNotifications(prev, tasks, day)) showNotification(n).catch(() => {});
    }
    saveSnapshot(snapshotOf(tasks, day));
  }, [tasks, tasksLoaded, now, settings.notifications]);

  useEffect(() => {
    if (!selected) {
      setTaskNotes([]);
      return;
    }
    fetchTaskNotes(selected.id)
      .then(setTaskNotes)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);

  // Refetch when the task changes underneath us (an agent replying touches `updated`).
  useEffect(() => {
    if (!selected) {
      setThread(null);
      return;
    }
    // A slow GET must not overwrite a newer thread (such as the one a send just returned).
    const req = ++threadReq.current;
    fetchTaskThread(selected.id)
      .then((t) => {
        if (req === threadReq.current) setThread(t);
      })
      .catch((e) => setError(String(e)));
  }, [selected?.id, selected?.updated, selected?.work_status]);

  useEffect(() => {
    if (!selected || isMobile) return;
    document.querySelector(`[data-task-row="${selected.id}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [selected?.id, isMobile, view]);

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
      // Completing the selected task moves on to the next one so you can keep working down the
      // list. The one exception is a phone browsing All tasks, where you stay on the task you
      // just completed (it is still in the list and can be reopened).
      const neighbor = currentIndex === -1 ? undefined : flatOrder[currentIndex + 1] ?? flatOrder[currentIndex - 1];
      if (isMobile && view === 'all') setSelected(completed);
      else if (isMobile) setSelected(neighbor ?? null);
      else selectTask(neighbor ?? completed);
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

  /** Refetches project names and metadata (archiving moves a project in or out of the names list). */
  async function reloadProjects() {
    try {
      const [names, details] = await Promise.all([fetchNames('projects'), fetchProjectDetails()]);
      setAllProjects(names);
      setProjectList(details);
    } catch (e) {
      setError(String(e));
    }
  }

  /** Saves project metadata. Rejects with the server's message so the editor can show it. */
  async function handleUpdateProject(name: string, patch: ProjectPatch) {
    const updated = await updateProject(name, patch);
    if (updated.name !== name) {
      // The server renamed the project on every task that uses it; mirror that now.
      const rename = (t: Task): Task => (t.project?.toLowerCase() === name.toLowerCase() ? { ...t, project: updated.name } : t);
      setTasks((prev) => prev.map(rename));
      setSelected((prev) => (prev ? rename(prev) : prev));
    }
    await reloadProjects();
  }

  async function handleCreateName(kind: NameKind, name: string): Promise<string | undefined> {
    try {
      const stored = await createName(kind, name);
      const add = (list: string[]) => (list.includes(stored) ? list : [...list, stored].sort((a, b) => a.localeCompare(b)));
      if (kind === 'projects') { setAllProjects(add); void reloadProjects(); } else setAllTags(add);
      return stored;
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDeleteName(kind: NameKind, name: string) {
    try {
      await deleteName(kind, name);
      if (kind === 'projects') { setAllProjects((l) => l.filter((x) => x !== name)); setProjectList((l) => l.filter((x) => x.name !== name)); } else setAllTags((l) => l.filter((x) => x !== name));
      // Tasks lose the name server-side; the SSE refresh will sync, but update now for snappiness.
      setTasks((prev) => prev.map((t) => (kind === 'projects'
        ? (t.project === name ? { ...t, project: undefined } : t)
        : { ...t, tags: t.tags.filter((x) => x !== name) })));
      setSelected((prev) => (prev && (kind === 'projects'
        ? (prev.project === name ? { ...prev, project: undefined } : prev)
        : { ...prev, tags: prev.tags.filter((x) => x !== name) })));
    } catch (e) {
      setError(String(e));
    }
  }

  // A service leaves the task lists, so drop it here and show it on the Services screen.
  // The work status is reset to todo: a service in review or needs-input would never run.
  async function handleConvertToService(id: number, schedule: { recurrence: string; due: string }) {
    try {
      await editTask(id, { kind: 'service', ...schedule, work_status: 'todo' });
      setTasks((prev) => prev.filter((t) => t.id !== id));
      setSelected(null);
      setServicesOpen(true);
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

  async function handleSendThread(kind: ThreadSendKind | 'approve', text: string) {
    if (!selected) return;
    try {
      const posted = await postTaskThread(selected.id, kind, text);
      threadReq.current++; // anything still in flight is older than this
      setThread(posted);
      // Refetch: a send can move the task (or close it), and the allowed moves change.
      const updated = await fetchTask(selected.id);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
      // In Next, answering, requesting changes or approving hands the task back to an agent (or
      // finishes it), so move on to whatever still needs you instead of staying on a finished item.
      const band = bandOf(updated);
      if (view === 'next' && (band === null || band === 'working')) {
        const waiting = flatOrder.filter((t) => { const b = bandOf(t); return b !== null && b !== 'working'; });
        const at = waiting.findIndex((t) => t.id === updated.id);
        const neighbor = waiting[at + 1] ?? waiting[at - 1];
        setSelected(neighbor ?? (isMobile ? null : updated));
      } else {
        setSelected(updated);
      }
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }

  const query = isMobile ? (mobileSearch ?? '') : '';
  const searchedTasks = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? tasks.filter((t) => t.title.toLowerCase().includes(q)) : tasks;
  }, [tasks, query]);
  const deferredIds = useMemo(
    () => new Set(Object.entries(deferrals).filter(([, until]) => until > now).map(([id]) => Number(id))),
    [deferrals, now],
  );

  // Next: the queue in bands. All tasks: the filtered, grouped list.
  const queue = useMemo(() => buildQueue(searchedTasks, deferredIds), [searchedTasks, deferredIds]);
  const nextOrder = useMemo(() => queueOrder(queue, { workingExpanded }), [queue, workingExpanded]);
  const filteredTasks = useMemo(
    () => searchedTasks.filter((t) => matchesFilter(t, today, dueFilter)),
    [searchedTasks, dueFilter, today],
  );
  const grouped = useMemo(() => groupTasks(filteredTasks, groupBy), [filteredTasks, groupBy]);
  const allOrder = useMemo(() => [...grouped.values()].flat(), [grouped]);
  const flatOrder = view === 'next' ? nextOrder : allOrder;
  const nextCount = useMemo(() => needsYouCount(tasks), [tasks]);
  const activity = useMemo(() => agentActivity(tasks, agents), [tasks, agents]);
  const serviceStatus = useMemo(() => serviceSummary(services), [services]);

  // The docked pane is never left empty: keep the selection on a queued task, and when the selected
  // one leaves the queue (approved, answered, completed, or changed by an agent) move to its neighbour.
  // On a phone selecting opens a full-screen page, so it never selects on its own.
  const prevNextIds = useRef<number[]>([]);
  const swipeStart = useRef<{ x: number; y: number } | null>(null);
  const nextIdsKey = nextOrder.map((t) => t.id).join(',');
  const selectedId = selected?.id ?? null;
  useEffect(() => {
    if (view !== 'next' || isMobile) {
      prevNextIds.current = [];
      return;
    }
    const currIds = nextOrder.map((t) => t.id);
    const id = nextSelection(prevNextIds.current, currIds, selectedId, true);
    prevNextIds.current = currIds;
    if (id !== selectedId) setSelected(id === null ? null : tasks.find((t) => t.id === id) ?? null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view, isMobile, nextIdsKey, selectedId]);

  function deferTask(task: Task) {
    const until = Date.now() + settings.deferMinutes * 60_000;
    const next = { ...loadDeferrals(Date.now()), [task.id]: until };
    saveDeferrals(next);
    setDeferrals(next);
    setNow(Date.now());
    // Deferring sinks the task, so move on to whatever is now first, like completing does.
    const remaining = flatOrder.filter((t) => t.id !== task.id && !deferredIds.has(t.id));
    if (remaining.length) selectTask(remaining[0]);
  }

  function undeferTask(task: Task) {
    const next = loadDeferrals(Date.now());
    delete next[task.id];
    saveDeferrals(next);
    setDeferrals(next);
  }

  function moveSelection(delta: number) {
    if (flatOrder.length === 0) return;
    const currentIndex = selected ? flatOrder.findIndex((t) => t.id === selected.id) : -1;
    const nextIndex = currentIndex === -1 ? 0 : Math.min(Math.max(currentIndex + delta, 0), flatOrder.length - 1);
    selectTask(flatOrder[nextIndex]);
  }

  function cycleDueFilter(delta: number) {
    const idx = ALL_TASKS_FILTERS.findIndex((i) => i.value === dueFilter);
    const next = (idx + delta + ALL_TASKS_FILTERS.length) % ALL_TASKS_FILTERS.length;
    setDueFilter(ALL_TASKS_FILTERS[next].value);
  }

  function cycleGroupBy() {
    const idx = GROUP_BY_OPTIONS.findIndex((o) => o.value === groupBy);
    setGroupBy(GROUP_BY_OPTIONS[(idx + 1) % GROUP_BY_OPTIONS.length].value);
  }

  /** Click a field in the inspector, unfolding the "Task details" fold first when the field sits inside it. */
  function clickFieldById(id: string) {
    const el = document.getElementById(id);
    const fold = el?.closest('details');
    if (fold && !fold.open) fold.open = true;
    el?.click();
  }

  function clickField(id: string) {
    if (!selected) return;
    if (inspectorVisible) {
      clickFieldById(id);
      return;
    }
    // Open the inspector first, then click once it has rendered.
    setRightOpen(true);
    window.setTimeout(() => clickFieldById(id), 50);
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
      if (selected && inspectorVisible) {
        if (e.key === 'A') e.preventDefault();
        clickFieldById(field[2]);
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
      case 'z':
        if (selected && view === 'next' && needsHumanAction(selected)) {
          e.preventDefault();
          deferTask(selected);
        }
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
        if (view === 'all') {
          e.preventDefault();
          cycleGroupBy();
        }
        break;
      case '[':
      case ']':
        if (view === 'all') {
          e.preventDefault();
          cycleDueFilter(e.key === '[' ? -1 : 1);
        }
        break;
      case 'Escape':
        if (isMobile) setSelected(null);
        else if (view === 'all') setRightOpen(false);
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
    const t = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(t);
  }, []);

  useEffect(() => {
    localStorage.setItem('taskPanelWidth', String(panelWidth));
  }, [panelWidth]);

  function paletteCommands(): PaletteCommand[] {
    const close = () => setPaletteOpen(false);
    const cmd = (label: string, kbd: string, fn: () => void): PaletteCommand => ({ label, kbd, run: () => { close(); fn(); } });
    return [
      cmd('New task', 'a', () => setShowNewTaskForm(true)),
      cmd('Go to Next', '', () => setView('next')),
      cmd('Go to All tasks', '', () => setView('all')),
      ...(view === 'all'
        ? [
            ...ALL_TASKS_FILTERS.map((f) => cmd(`Filter: ${f.label}`, '[ ]', () => setDueFilter(f.value))),
            ...GROUP_BY_OPTIONS.map((g) => cmd(`Group by ${g.label.toLowerCase()}`, 'G', () => setGroupBy(g.value))),
          ]
        : []),
      cmd('Next task', 'j ↓', () => moveSelection(1)),
      cmd('Previous task', 'k ↑', () => moveSelection(-1)),
      ...(selected
        ? [
            cmd(`Selected: ${selected.status === 'done' ? 'reopen' : 'mark done'}`, 'Enter', () => toggleDone(selected)),
            ...QUICK_DUE.map(([key, label, fn]) => cmd(`Selected: ${label.toLowerCase()}`, key, () => setQuickDue(selected, fn ? fn(today) : null))),
            ...FIELD_KEYS.map(([key, label, id]) => cmd(`Selected: ${label.toLowerCase()}`, key, () => clickField(id))),
            ...(view === 'next' && needsHumanAction(selected)
              ? [cmd(`Selected: defer ${settings.deferMinutes} min`, 'z', () => deferTask(selected))]
              : []),
            ...(selected.notes?.length ? [cmd('Selected: open first linked note', 'g', openFirstNote)] : []),
          ]
        : []),
      cmd('Open Services', '', () => setServicesOpen(true)),
      cmd('Settings', '', () => setSettingsOpen(true)),
      ...(view === 'all' ? [cmd(rightOpen ? 'Close inspector' : 'Open inspector', rightOpen ? 'Esc' : '', () => setRightOpen((v) => !v))] : []),
    ];
  }

  /** Picking a task in ⌘K shows it where it lives: in the queue if it is there, otherwise in All tasks. */
  function pickFromPalette(task: Task) {
    setPaletteOpen(false);
    if (view === 'next') {
      const band = bandOf(task);
      if (band === null) setView('all');
      else if (band === 'working') setWorkingExpanded(true);
    }
    selectTask(task);
  }

  const detailProps = selected && {
    task: selected,
    today,
    agentOptions,
    thread,
    allProjects,
    allTags,
    onCreateProject: (name: string) => handleCreateName('projects', name),
    onCreateTag: (name: string) => handleCreateName('tags', name),
    onDeleteProject: (name: string) => handleDeleteName('projects', name),
    projectMeta,
    onUpdateProject: handleUpdateProject,
    onDeleteTag: (name: string) => handleDeleteName('tags', name),
    notes: visibleNotes,
    onEdit: (changes: Parameters<typeof editTask>[1]) => handleEditField(selected.id, changes),
    onSendThread: handleSendThread,
    onApprove: (text: string) => handleSendThread('approve', text),
    onCreateNote: handleCreateNote,
    onOpenNote: handleOpenNote,
    onSaveNote: handleSaveNote,
    onUnlinkNote: handleUnlinkNote,
    onConvertToService: (schedule: { recurrence: string; due: string }) => handleConvertToService(selected.id, schedule),
  };

  const statusProps = {
    activity,
    services: serviceStatus,
    onEditAgent: (name: string) => setEditingAgent(agents.find((a) => a.name === name) ?? null),
    onOpenServices: () => setServicesOpen(true),
    onOpenSettings: () => setSettingsOpen(true),
  };
  // Tasks that still need an agent or a date open with their details unfolded, so they can be fixed in place.
  const detailsOpen = !!selected && bandOf(selected) === 'triage';
  // A review or an agent's question is settled by approving or answering, so "Mark done" must not lead there.
  const doneIsPrimary = !!selected && bandOf(selected) !== 'review' && bandOf(selected) !== 'input';
  const selectedIndex = selected ? flatOrder.findIndex((t) => t.id === selected.id) : -1;
  const queueProps = {
    queue,
    selectedId,
    today,
    deferredIds,
    workingExpanded,
    onToggleWorking: () => setWorkingExpanded((v) => !v),
    onSelect: (t: Task) => (isMobile ? setSelected(t) : selectTask(t)),
    projectMeta,
  };

  function swipeToNeighbor(e: React.TouchEvent, phase: 'start' | 'end') {
    if (phase === 'start') {
      const touch = e.touches[0];
      const inScroller = (e.target as HTMLElement).closest('pre, table, textarea, input, [data-no-swipe]');
      swipeStart.current = inScroller ? null : { x: touch.clientX, y: touch.clientY };
      return;
    }
    const start = swipeStart.current;
    swipeStart.current = null;
    if (!start) return;
    const touch = e.changedTouches[0];
    const dx = touch.clientX - start.x;
    const dy = touch.clientY - start.y;
    if (Math.abs(dx) > 80 && Math.abs(dx) > Math.abs(dy) * 2) moveSelection(dx < 0 ? 1 : -1);
  }

  const detailActions = selected && (
    <>
      {selected.status === 'open' ? (
        <Button size="sm" variant={doneIsPrimary ? 'primary' : 'secondary'} onClick={() => handleMarkDone(selected)}>Mark done</Button>
      ) : (
        <Button size="sm" variant="secondary" onClick={() => handleReopen(selected)}>Reopen</Button>
      )}
      {needsHumanAction(selected) && (
        deferredIds.has(selected.id)
          ? <Button size="sm" variant="secondary" onClick={() => undeferTask(selected)}>Undefer</Button>
          : <Button size="sm" variant="secondary" onClick={() => deferTask(selected)}>Defer {settings.deferMinutes}m</Button>
      )}
      <Button size="sm" variant="secondary" onClick={() => handleDelete(selected)}>Delete</Button>
    </>
  );

  if (isMobile) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100dvh', overflow: 'hidden', paddingTop: 'env(safe-area-inset-top, 0px)' }}>
        <MobileStatusBar {...statusProps} />

        {mobileSearch === null ? (
          <div style={{ flex: 'none', display: 'flex', alignItems: 'center', gap: 6, padding: '10px 6px 8px 16px' }}>
            <ViewToggle mobile view={view} onChange={setView} nextCount={nextCount} allCount={tasks.length} />
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

        {view === 'all' && (
          <div style={{ flex: 'none', display: 'flex', alignItems: 'center', gap: 8, padding: '2px 16px 10px' }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <FilterPills mobile items={ALL_TASKS_FILTERS} active={dueFilter} tasks={tasks} today={today} onPick={setDueFilter} />
            </div>
            <select
              aria-label="Group by"
              value={groupBy}
              onChange={(e) => setGroupBy(e.target.value as GroupBy)}
              style={{ flex: 'none', height: 34, background: 'var(--ink-2)', color: 'var(--fg-1)', border: '1px solid var(--hairline)', borderRadius: 999, padding: '0 10px', fontSize: 13 }}
            >
              {GROUP_BY_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>{o.value === 'none' ? 'No grouping' : `By ${o.label.toLowerCase()}`}</option>
              ))}
            </select>
          </div>
        )}

        {error && <div style={{ padding: '8px 16px', color: 'var(--danger)', fontSize: 13 }}>{error}</div>}

        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', borderTop: '1px solid var(--hairline)', paddingBottom: 96 }}>
          {view === 'next' ? (
            <NextQueue mobile {...queueProps} />
          ) : (
            <>
              {[...grouped.entries()].map(([groupName, groupTasksList]) => (
                <div key={groupName || 'all'}>
                  {groupBy !== 'none' && (
                    <div style={{ position: 'sticky', top: 0, zIndex: 2, display: 'flex', alignItems: 'center', gap: 8, height: 36, padding: '0 16px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                      {groupBy === 'project' && <ProjectDot color={projectMeta.get(groupName.toLowerCase())?.color} />}
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600, textTransform: groupBy === 'priority' ? 'uppercase' : 'none' }}>{groupName}</span>
                      <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{countLabel(groupTasksList.length)}</span>
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
              ))}
              {filteredTasks.length === 0 ? (
                <EmptyState title={query ? 'No matches.' : filterItem.empty[0]} body={query ? '' : filterItem.empty[1]} padding="72px 32px" />
              ) : (
                <div style={{ padding: 16, textAlign: 'center', fontSize: 12, color: 'var(--fg-5)' }}>Swipe right to complete · left to set a due date</div>
              )}
            </>
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
            <div style={{ height: 52, flex: 'none', display: 'flex', alignItems: 'center', gap: 2, padding: '0 6px', borderBottom: '1px solid var(--hairline)' }}>
              <button
                onClick={() => setSelected(null)}
                style={{ height: 44, display: 'flex', alignItems: 'center', gap: 2, padding: '0 8px 0 4px', background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--accent)', fontSize: 15, fontWeight: 600 }}
              >
                <ChevronLeft size={22} color="var(--accent)" />
                {view === 'next' ? 'Next' : 'All tasks'}
              </button>
              <div style={{ flex: 1, textAlign: 'center', fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-4)' }}>
                {selectedIndex >= 0 ? `${selectedIndex + 1} of ${flatOrder.length}` : `#${selected.id}`}
              </div>
              <MobileIconButton label="Previous task" onClick={() => moveSelection(-1)}><ChevronUp size={20} color="var(--fg-3)" /></MobileIconButton>
              <MobileIconButton label="Next task" onClick={() => moveSelection(1)}><ChevronDown size={20} color="var(--fg-3)" /></MobileIconButton>
            </div>
            <div
              key={selected.id}
              onTouchStart={(e) => swipeToNeighbor(e, 'start')}
              onTouchEnd={(e) => swipeToNeighbor(e, 'end')}
              style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: '16px 20px 24px', color: 'var(--fg-5)' }}
            >
              <TaskDetail {...detailProps} mobile showId={false} compact detailsOpen={detailsOpen} />
            </div>
            <div style={{ flex: 'none', display: 'flex', gap: 10, padding: '12px 16px calc(12px + env(safe-area-inset-bottom, 22px))', borderTop: '1px solid var(--hairline)', background: 'var(--ink-2)' }}>
              <MobileActionButton grow primary={selected.status === 'open' && doneIsPrimary} onClick={() => toggleDone(selected)}>
                {selected.status === 'open' ? 'Mark done' : 'Reopen'}
              </MobileActionButton>
              {needsHumanAction(selected) && (
                deferredIds.has(selected.id)
                  ? <MobileActionButton onClick={() => undeferTask(selected)}>Undefer</MobileActionButton>
                  : <MobileActionButton onClick={() => deferTask(selected)}>Defer</MobileActionButton>
              )}
              <MobileActionButton onClick={() => handleDelete(selected)}>Delete</MobileActionButton>
            </div>
          </div>
        )}

        {editingAgent && <AgentEditor agent={editingAgent} onClose={() => setEditingAgent(null)} />}

        {servicesOpen && <ServicesView mobile agents={agents} onClose={() => setServicesOpen(false)} />}

        {settingsOpen && (
          <SettingsDialog
            settings={settings}
            onSave={(s) => { saveSettings(s); setSettings(s); }}
            onClose={() => setSettingsOpen(false)}
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
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <StatusStrip {...statusProps} />

      <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
        <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
          <header style={{ height: 52, flex: 'none', padding: '0 16px', borderBottom: '1px solid var(--hairline)', display: 'flex', alignItems: 'center', gap: 12 }}>
            <ViewToggle view={view} onChange={setView} nextCount={nextCount} allCount={tasks.length} />
            <div style={{ flex: 1 }} />
            {view === 'all' && (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, height: 30, padding: '0 10px', border: '1px solid var(--hairline)', borderRadius: 5 }}>
                <label htmlFor="group-by" style={{ fontSize: 13, color: 'var(--fg-4)' }}>Group</label>
                <select
                  id="group-by"
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
            )}
            <button
              onClick={() => setPaletteOpen(true)}
              title="Search tasks or run a command (⌘K)"
              style={{
                display: 'flex', alignItems: 'center', gap: 8, height: 30, padding: '0 6px 0 12px', border: '1px solid var(--hairline)',
                borderRadius: 5, background: 'var(--ink-2)', width: 260, minWidth: 0, flexShrink: 1, cursor: 'pointer',
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
            {view === 'all' && (
              <button
                onClick={() => setRightOpen((v) => !v)}
                title={rightOpen ? 'Hide inspector' : 'Show inspector'}
                aria-label={rightOpen ? 'Hide inspector' : 'Show inspector'}
                style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 4 }}
              >
                {rightOpen ? <PanelRightClose size={18} color="var(--fg-4)" /> : <PanelRightOpen size={18} color="var(--fg-4)" />}
              </button>
            )}
          </header>
          {error && <div style={{ padding: 16, color: 'var(--danger)' }}>{error}</div>}

          {view === 'next' ? (
            <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
              <div style={{ width: 'clamp(300px, 36%, 420px)', flex: 'none', overflowY: 'auto', borderRight: '1px solid var(--hairline)' }}>
                <NextQueue {...queueProps} />
              </div>
              <section aria-label="Selected task" style={{ flex: 1, minWidth: 0, overflowY: 'auto', background: 'var(--ink-3)', color: 'var(--fg-5)' }}>
                {selected && detailProps ? (
                  <div key={selected.id} className="task-panel-content" style={{ maxWidth: 880, margin: '0 auto' }}>
                    <TaskDetail {...detailProps} compact detailsOpen={detailsOpen} conversationMaxHeight="52vh" actions={detailActions} />
                  </div>
                ) : (
                  <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>Select a task</div>
                )}
              </section>
            </div>
          ) : (
            <>
              <div style={{ flex: 'none', padding: '10px 16px', borderBottom: '1px solid var(--hairline-soft)' }}>
                <FilterPills items={ALL_TASKS_FILTERS} active={dueFilter} tasks={tasks} today={today} onPick={setDueFilter} />
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 16, height: 34, flex: 'none', padding: '0 24px', borderBottom: '1px solid var(--hairline-soft)', fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--fg-5)' }}>
                <span style={{ width: COL.check, flex: 'none' }} />
                <span style={{ width: COL.id, flex: 'none' }}>ID</span>
                <span style={{ width: COL.priority, flex: 'none' }}>Priority</span>
                <span style={{ width: COL.flags, flex: 'none' }} />
                <span style={{ width: COL.status, flex: 'none' }}>Status</span>
                <span style={{ width: COL.agent, flex: 'none' }}>Agent</span>
                <span style={{ flex: 1 }}>Task</span>
                <span style={{ width: COL.due, flex: 'none' }}>Due</span>
                <span style={{ width: COL.created, flex: 'none' }}>Created</span>
                <span style={{ width: COL.effort, flex: 'none' }}>Effort</span>
              </div>
              <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
                {[...grouped.entries()].map(([groupName, groupTasksList]) => (
                  <div key={groupName || 'all'}>
                    {groupBy !== 'none' && (
                      <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 38, padding: '0 24px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                        {groupBy === 'project' && <ProjectDot color={projectMeta.get(groupName.toLowerCase())?.color} />}
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
                ))}
                {filteredTasks.length === 0 && (
                  <EmptyState title={filterItem.empty[0]} body={filterItem.empty[1]} padding="120px 24px" onNewTask={() => setShowNewTaskForm(true)} />
                )}
              </div>
            </>
          )}
        </main>

        {view === 'all' && panelPhase !== 'closed' && (
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
                  <TaskDetail {...detailProps} onClose={() => setRightOpen(false)} actions={detailActions} />
                ) : (
                  'Select a task'
                )}
              </div>
            </aside>
          </>
        )}
      </div>

      {editingAgent && (
        <AgentEditor agent={editingAgent} onClose={() => setEditingAgent(null)} />
      )}

      {servicesOpen && <ServicesView agents={agents} onClose={() => setServicesOpen(false)} />}

      {settingsOpen && (
        <SettingsDialog
          settings={settings}
          onSave={(s) => { saveSettings(s); setSettings(s); }}
          onClose={() => setSettingsOpen(false)}
        />
      )}

      {paletteOpen && (
        <CommandPalette
          tasks={tasks}
          commands={paletteCommands()}
          onPickTask={pickFromPalette}
          onClose={() => setPaletteOpen(false)}
        />
      )}
    </div>
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
