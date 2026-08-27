import { useEffect, useMemo, useState } from 'react';
import './tokens.css';
import { Terminal, Sun, CalendarDays, Settings } from 'lucide-react';
import { Button } from './components/Button';
import { fetchTasks, fetchAgents } from './api';
import type { Task, AgentProfile } from './types';
import { countAllOpen } from './dueWindow';

function groupByAgent(tasks: Task[]): Map<string, Task[]> {
  const groups = new Map<string, Task[]>();
  for (const t of tasks) {
    const key = t.agent ?? 'unassigned';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(t);
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

  useEffect(() => {
    Promise.all([fetchTasks(), fetchAgents()])
      .then(([t, a]) => {
        setTasks(t);
        setAgents(a);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const grouped = useMemo(() => groupByAgent(tasks), [tasks]);
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
          {agents.map((a) => (
            <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 10, height: 28 }}>
              <span style={{ width: 7, height: 7, borderRadius: 999, background: 'var(--fg-5)' }} />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, color: 'var(--fg-3)' }}>{a.name}</span>
            </div>
          ))}
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
          <Button>New task</Button>
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
          {[...grouped.entries()].map(([agentName, agentTasks]) => (
            <div key={agentName}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 38, padding: '0 24px', background: 'var(--ink-2)', borderBottom: '1px solid var(--hairline-soft)' }}>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, fontWeight: 600 }}>{agentName}</span>
                <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>{agentTasks.length} tasks</span>
              </div>
              {agentTasks.map((t) => (
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
                  <span style={{ flex: 1, minWidth: 0, fontSize: 14.5, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
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
            <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 20, color: 'var(--fg-1)', marginBottom: 12 }}>
              {selected.title}
            </div>
            <div style={{ fontSize: 13, color: 'var(--fg-3)' }}>#{selected.id} · {selected.priority}</div>
          </>
        ) : (
          'Select a task'
        )}
      </aside>
    </div>
  );
}
