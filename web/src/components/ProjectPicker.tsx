import { useEffect, useRef, useState } from 'react';
import { Check, Plus, Trash2 } from 'lucide-react';

interface ProjectPickerProps {
  id?: string;
  value: string;
  /** Every project currently used by any task. */
  projects: string[];
  onSave: (value: string) => void;
  /** Register a new project (stored independently of tasks); resolves to its stored name. */
  onCreateProject: (name: string) => Promise<string | undefined>;
  /** Remove a project from every task that uses it. */
  onDeleteProject: (name: string) => void;
}

/** Dropdown of existing projects, with inline "add new" and per-project delete. */
export function ProjectPicker({ id, value, projects, onSave, onCreateProject, onDeleteProject }: ProjectPickerProps) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const [confirming, setConfirming] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function onDown(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) close();
    }
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);

  function close() {
    setOpen(false);
    setDraft('');
    setConfirming(null);
  }

  function choose(v: string) {
    if (v !== value) onSave(v);
    close();
  }

  async function addNew() {
    const name = draft.trim();
    if (!name) return;
    const stored = await onCreateProject(name);
    if (stored) choose(stored);
  }

  const trimmed = draft.trim();
  const exists = projects.some((p) => p.toLowerCase() === trimmed.toLowerCase());
  // Keep the current value visible even if it somehow isn't in the list.
  const list = value && !projects.includes(value) ? [value, ...projects] : projects;

  const rowStyle = (active: boolean) => ({
    display: 'flex', alignItems: 'center', gap: 6, padding: '6px 8px', fontSize: 13, borderRadius: 4,
    cursor: 'pointer', color: active ? 'var(--fg-1)' : 'var(--fg-3)',
    background: active ? 'var(--accent-soft)' : 'transparent',
  } as const);

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <div id={id} onClick={() => setOpen(true)} style={{ cursor: 'pointer', minHeight: 20 }} title="Click to select">
        <span style={{ display: 'block', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {value || <span style={{ color: 'var(--fg-5)' }}>No project</span>}
        </span>
      </div>
      {open && (
        <div
          onKeyDown={(e) => {
            // Keep global shortcuts from firing while the picker is open.
            e.stopPropagation();
            if (e.key === 'Escape') close();
          }}
          style={{
            position: 'absolute', top: '100%', left: 0, zIndex: 50, marginTop: 4, minWidth: 220,
            background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 6,
            boxShadow: '0 4px 16px rgba(0,0,0,0.4)', padding: 4,
          }}
        >
          <div style={{ maxHeight: 200, overflowY: 'auto' }}>
            <div onClick={() => choose('')} style={rowStyle(!value)}>
              <span style={{ flex: 1, fontStyle: 'italic' }}>No project</span>
              {!value && <Check size={13} />}
            </div>
            {list.map((p) =>
              confirming === p ? (
                <div key={p} style={{ ...rowStyle(false), cursor: 'default', gap: 8, color: 'var(--fg-2)' }}>
                  <span style={{ flex: 1 }}>Delete “{p}” from all tasks?</span>
                  <button
                    type="button"
                    onClick={() => { onDeleteProject(p); close(); }}
                    style={{ background: 'transparent', border: 'none', color: 'var(--rose, #bf616a)', cursor: 'pointer', fontWeight: 600 }}
                  >
                    Delete
                  </button>
                  <button
                    type="button"
                    onClick={() => setConfirming(null)}
                    style={{ background: 'transparent', border: 'none', color: 'var(--fg-4)', cursor: 'pointer' }}
                  >
                    Cancel
                  </button>
                </div>
              ) : (
                <div key={p} onClick={() => choose(p)} className="project-row" style={rowStyle(p === value)}>
                  <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p}</span>
                  {p === value && <Check size={13} />}
                  <button
                    type="button"
                    title={`Delete project “${p}”`}
                    onClick={(e) => { e.stopPropagation(); setConfirming(p); }}
                    style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 2 }}
                  >
                    <Trash2 size={13} color="var(--fg-4)" />
                  </button>
                </div>
              ),
            )}
          </div>
          <div style={{ display: 'flex', gap: 4, marginTop: 4, paddingTop: 6, borderTop: '1px solid var(--hairline-soft)' }}>
            <input
              autoFocus
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); void addNew(); } }}
              placeholder="New project…"
              style={{
                flex: 1, minWidth: 0, background: 'var(--ink)', border: '1px solid var(--hairline)', borderRadius: 4,
                color: 'var(--fg-1)', fontSize: 13, padding: '4px 6px',
              }}
            />
            <button
              type="button"
              onClick={() => void addNew()}
              disabled={!trimmed}
              title={exists ? 'Use existing project' : 'Add project'}
              style={{
                display: 'flex', alignItems: 'center', background: 'var(--accent-soft)', border: '1px solid var(--accent)',
                borderRadius: 4, color: 'var(--fg-1)', cursor: trimmed ? 'pointer' : 'default', opacity: trimmed ? 1 : 0.4, padding: '0 8px',
              }}
            >
              <Plus size={14} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
