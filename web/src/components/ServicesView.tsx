import { useEffect, useRef, useState } from 'react';
import { ChevronLeft, X } from 'lucide-react';
import type { AgentProfile, Service, ServiceRun } from '../types';
import { Button } from './Button';
import { Markdown } from './Markdown';
import { addService, editTask, fetchServiceRuns, fetchServices, setServicePaused } from '../api';

// The Services screen: recurring agent jobs that advance their own schedule with
// `task service run`. Services never appear in the normal task views.

function stateOf(s: Service): { label: string; color: string } {
  if (s.paused) return { label: 'paused', color: 'var(--fg-5)' };
  if (s.work_status === 'failing') return { label: 'failing', color: 'var(--danger, #e5484d)' };
  return { label: 'active', color: 'var(--accent)' };
}

const cell = { padding: '6px 8px', fontSize: 13, borderBottom: '1px solid var(--hairline)', textAlign: 'left' as const };
const baseInput = {
  background: 'var(--ink-1, transparent)', color: 'var(--fg-1)', border: '1px solid var(--hairline)',
  borderRadius: 4, padding: '4px 6px', fontSize: 13,
};
// iOS zooms into any focused field under 16px, so phones get bigger fields.
const mobileInput = { ...baseInput, fontSize: 16, padding: '8px 10px' };

export function ServicesView({ agents, mobile = false, onClose }: { agents: AgentProfile[]; mobile?: boolean; onClose: () => void }) {
  const [services, setServices] = useState<Service[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    function refresh() {
      fetchServices().then((s) => { setServices(s); setError(null); }).catch((e) => setError(String(e)));
    }
    refresh();
    const source = new EventSource('/api/events');
    source.onmessage = refresh;
    return () => source.close();
  }, []);

  const selected = services.find((s) => s.id === selectedId) ?? null;
  const newButton = <Button variant="secondary" onClick={() => setAdding((v) => !v)}>{adding ? 'Cancel' : 'New service'}</Button>;

  const body = (
    <>
      {error && <div style={{ color: 'var(--danger, #e5484d)', fontSize: 13, marginBottom: 12 }}>{error}</div>}
      {adding && <NewServiceForm agents={agents} mobile={mobile} onDone={() => setAdding(false)} onError={setError} />}

      {services.length === 0 ? (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', padding: '20px 0' }}>
          No services yet. A service is a recurring agent job that records its own runs.
        </div>
      ) : mobile ? (
        <>
          <div style={{ fontSize: 12, color: 'var(--fg-5)', marginBottom: 6 }}>Tap a service to change its schedule, agent or instructions.</div>
          <div style={{ display: 'flex', flexDirection: 'column', marginBottom: 16, borderTop: '1px solid var(--hairline)' }}>
            {services.map((s) => {
              const st = stateOf(s);
              return (
                <div
                  key={s.id}
                  onClick={() => setSelectedId(s.id)}
                  style={{
                    display: 'flex', flexDirection: 'column', gap: 4, padding: '10px 8px', cursor: 'pointer', borderBottom: '1px solid var(--hairline)',
                    background: s.id === selectedId ? 'var(--accent-soft)' : 'transparent',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ flex: 1, minWidth: 0, fontSize: 15 }}>{s.title}</span>
                    <span style={{ fontSize: 12.5, color: st.color, fontWeight: 600 }}>{st.label}</span>
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '2px 10px', fontSize: 12.5, color: 'var(--fg-4)' }}>
                    <span style={{ fontFamily: 'var(--font-mono)' }}>{s.agent ?? '—'}</span>
                    <span style={{ fontFamily: 'var(--font-mono)' }}>{s.recurrence}</span>
                    <span style={{ fontFamily: 'var(--font-mono)' }}>next {s.due_date}</span>
                  </div>
                  <div style={{ fontSize: 12.5, color: 'var(--fg-5)' }}>
                    Last run: {s.last_run ? `${new Date(s.last_run.ran_at).toLocaleString()} · ${s.last_run.result}` : 'never'}
                  </div>
                </div>
              );
            })}
          </div>
        </>
      ) : (
        <>
          <div style={{ fontSize: 12, color: 'var(--fg-5)', marginBottom: 6 }}>Click a service to change its schedule, agent or instructions.</div>
          <table style={{ width: '100%', borderCollapse: 'collapse', marginBottom: 16 }}>
            <thead>
              <tr style={{ color: 'var(--fg-4)' }}>
                {['Title', 'Agent', 'Schedule', 'Next run', 'Last run', 'State'].map((h) => <th key={h} style={cell}>{h}</th>)}
              </tr>
            </thead>
            <tbody>
              {services.map((s) => {
                const st = stateOf(s);
                return (
                  <tr
                    key={s.id}
                    onClick={() => setSelectedId(s.id)}
                    style={{ cursor: 'pointer', background: s.id === selectedId ? 'var(--accent-soft)' : 'transparent' }}
                  >
                    <td style={cell}>{s.title}</td>
                    <td style={cell}>{s.agent ?? '—'}</td>
                    <td style={{ ...cell, fontFamily: 'var(--font-mono)' }}>{s.recurrence}</td>
                    <td style={{ ...cell, fontFamily: 'var(--font-mono)' }}>{s.due_date}</td>
                    <td style={cell}>
                      {s.last_run ? `${new Date(s.last_run.ran_at).toLocaleString()} · ${s.last_run.result}` : 'never'}
                    </td>
                    <td style={{ ...cell, color: st.color, fontWeight: 600 }}>{st.label}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </>
      )}

      {selected && <ServiceDetail key={selected.id} service={selected} agents={agents} mobile={mobile} onError={setError} />}
    </>
  );

  // On a phone the screen takes over the viewport like the task detail does.
  if (mobile) {
    return (
      <div className="mobile-detail" style={{ zIndex: 25, paddingTop: 'env(safe-area-inset-top, 0px)', color: 'var(--fg-1)' }}>
        <div style={{ height: 52, flex: 'none', display: 'flex', alignItems: 'center', gap: 4, padding: '0 10px 0 6px', borderBottom: '1px solid var(--hairline)' }}>
          <button
            onClick={onClose}
            style={{ height: 44, display: 'flex', alignItems: 'center', gap: 2, padding: '0 8px 0 4px', background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--accent)', fontSize: 15, fontWeight: 600 }}
          >
            <ChevronLeft size={22} color="var(--accent)" />
            Tasks
          </button>
          <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 16, textAlign: 'center' }}>Services</span>
          {newButton}
        </div>
        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: '16px 16px calc(24px + env(safe-area-inset-bottom, 0px))' }}>
          {body}
        </div>
      </div>
    );
  }

  return (
    <div
      onClick={onClose}
      style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{ background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8, padding: 24, width: 960, maxWidth: '95vw', maxHeight: '90vh', overflow: 'auto', color: 'var(--fg-1)' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16, gap: 8 }}>
          <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 16 }}>Services</span>
          {newButton}
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex' }}>
            <X size={16} color="var(--fg-4)" />
          </button>
        </div>
        {body}
      </div>
    </div>
  );
}

function NewServiceForm({ agents, mobile, onDone, onError }: { agents: AgentProfile[]; mobile: boolean; onDone: () => void; onError: (e: string) => void }) {
  const input = mobile ? mobileInput : baseInput;
  const [title, setTitle] = useState('');
  const [recurrence, setRecurrence] = useState('daily');
  const [due, setDue] = useState('');
  const [agent, setAgent] = useState(agents[0]?.name ?? '');
  const [instructions, setInstructions] = useState('');

  function submit() {
    addService({ title, recurrence, due: due || undefined, agent: agent || undefined, instructions: instructions || undefined })
      .then(onDone)
      .catch((e) => onError(String(e)));
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginBottom: 16, padding: 12, border: '1px solid var(--hairline)', borderRadius: 6 }}>
      <input style={input} placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
        <input style={{ ...input, flex: 1, minWidth: 200 }} placeholder="Recurrence (daily, weekly:fri, monthly:2:mon…)" value={recurrence} onChange={(e) => setRecurrence(e.target.value)} />
        <input style={input} type="date" value={due} onChange={(e) => setDue(e.target.value)} title="First run (default today)" />
        <select style={input} value={agent} onChange={(e) => setAgent(e.target.value)}>
          <option value="">No agent</option>
          {agents.map((a) => <option key={a.name} value={a.name}>{a.name}</option>)}
        </select>
      </div>
      <textarea style={{ ...input, minHeight: 60 }} placeholder="Instructions for each run" value={instructions} onChange={(e) => setInstructions(e.target.value)} />
      <div><Button onClick={submit} disabled={!title.trim() || !recurrence.trim()}>Create service</Button></div>
    </div>
  );
}

// Agent names match ignoring case, so show the profile's own spelling in the picker.
function profileName(agents: AgentProfile[], name: string | null | undefined): string {
  if (!name) return '';
  return agents.find((a) => a.name.toLowerCase() === name.toLowerCase())?.name ?? name;
}

function ServiceDetail({ service, agents, mobile, onError }: { service: Service; agents: AgentProfile[]; mobile: boolean; onError: (e: string) => void }) {
  const input = mobile ? mobileInput : baseInput;
  const [runs, setRuns] = useState<ServiceRun[]>([]);
  const [recurrence, setRecurrence] = useState(service.recurrence ?? '');
  const [due, setDue] = useState(service.due_date ?? '');
  const [agent, setAgent] = useState(profileName(agents, service.agent));
  const [instructions, setInstructions] = useState(service.instructions ?? '');
  const panelRef = useRef<HTMLDivElement>(null);

  // The detail sits below the table, so bring it into view when a row is picked.
  useEffect(() => {
    panelRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }, []);

  // Reload the history whenever the service changes (a new run bumps run_count).
  useEffect(() => {
    fetchServiceRuns(service.id).then(setRuns).catch((e) => onError(String(e)));
  }, [service.id, service.run_count, onError]);

  // Follow server-side changes (a run advances the due date, a CLI edit changes the
  // agent) so the form doesn't show stale values as unsaved edits.
  const [seen, setSeen] = useState({ recurrence: service.recurrence, due: service.due_date, agent: service.agent });
  if (seen.recurrence !== service.recurrence || seen.due !== service.due_date || seen.agent !== service.agent) {
    setSeen({ recurrence: service.recurrence, due: service.due_date, agent: service.agent });
    if (seen.recurrence !== service.recurrence) setRecurrence(service.recurrence ?? '');
    if (seen.due !== service.due_date) setDue(service.due_date ?? '');
    if (seen.agent !== service.agent) setAgent(profileName(agents, service.agent));
  }

  function saveSchedule() {
    editTask(service.id, { recurrence, due, agent }).catch((e) => onError(String(e)));
  }
  function saveInstructions() {
    editTask(service.id, { instructions }).catch((e) => onError(String(e)));
  }

  const scheduleDirty =
    recurrence !== (service.recurrence ?? '') || due !== (service.due_date ?? '') || agent.toLowerCase() !== (service.agent ?? '').toLowerCase();
  const instructionsDirty = instructions !== (service.instructions ?? '');
  // Keep an agent that has no profile (e.g. set from the CLI) selectable instead of dropping it.
  const agentMissing = agent !== '' && !agents.some((a) => a.name === agent);

  return (
    <div ref={panelRef} style={{ display: 'flex', flexDirection: 'column', gap: 12, borderTop: '1px solid var(--hairline)', paddingTop: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ flex: 1, fontWeight: 600 }}>{service.title}</span>
        <Button variant="secondary" onClick={() => setServicePaused(service.id, !service.paused).catch((e) => onError(String(e)))}>
          {service.paused ? 'Resume' : 'Pause'}
        </Button>
      </div>

      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center' }}>
        <span style={{ fontSize: 13, color: 'var(--fg-4)', width: mobile ? '100%' : 90 }}>Schedule</span>
        <input style={{ ...input, flex: 1, minWidth: 160 }} value={recurrence} onChange={(e) => setRecurrence(e.target.value)} />
        <input style={input} type="date" value={due} onChange={(e) => setDue(e.target.value)} title="Next run" />
        <select style={input} value={agent} onChange={(e) => setAgent(e.target.value)} title="Agent">
          <option value="">No agent</option>
          {agentMissing && <option value={agent}>{agent}</option>}
          {agents.map((a) => <option key={a.name} value={a.name}>{a.name}</option>)}
        </select>
        <Button onClick={saveSchedule} disabled={!scheduleDirty || !recurrence.trim() || !due}>Save</Button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <span style={{ fontSize: 13, color: 'var(--fg-4)' }}>Instructions</span>
        <textarea style={{ ...input, minHeight: 90 }} value={instructions} onChange={(e) => setInstructions(e.target.value)} />
        <div><Button onClick={saveInstructions} disabled={!instructionsDirty}>Save instructions</Button></div>
      </div>

      <div>
        <div style={{ fontSize: 13, color: 'var(--fg-4)', marginBottom: 6 }}>Run history ({service.run_count})</div>
        {runs.length === 0 ? (
          <div style={{ fontSize: 13, color: 'var(--fg-5)' }}>No runs recorded yet.</div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {runs.map((r) => (
              <div key={r.id} style={{ fontSize: 13, display: 'flex', gap: 10 }}>
                <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--fg-4)', whiteSpace: 'nowrap' }}>{new Date(r.ran_at).toLocaleString()}</span>
                <span style={{ fontWeight: 600, color: r.result === 'ok' ? 'var(--accent)' : 'var(--danger, #e5484d)' }}>{r.result}</span>
                <span style={{ color: 'var(--fg-3)' }}>{r.summary && <Markdown text={r.summary} inline />}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
