import { useState } from 'react';
import type { Project, ProjectPatch, ProjectStatus } from '../types';
import { PROJECT_COLORS, PROJECT_STATUSES } from '../projects';

interface ProjectEditorProps {
  project: Project;
  /** Save only the fields that changed. Rejects with the server's message on invalid input. */
  onSave: (patch: ProjectPatch) => Promise<void>;
  onCancel: () => void;
}

const input = {
  width: '100%', boxSizing: 'border-box', background: 'var(--ink)', border: '1px solid var(--hairline)',
  borderRadius: 4, color: 'var(--fg-1)', fontSize: 13, padding: '4px 6px',
} as const;

const label = { display: 'block', fontSize: 11, color: 'var(--fg-4)', margin: '8px 0 2px' } as const;

/** Edits every field of a project; renaming updates the project's tasks too. */
export function ProjectEditor({ project, onSave, onCancel }: ProjectEditorProps) {
  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description ?? '');
  const [status, setStatus] = useState<ProjectStatus>(project.status);
  const [color, setColor] = useState(project.color ?? '');
  const [agent, setAgent] = useState(project.default_agent ?? '');
  const [due, setDue] = useState(project.due ?? '');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function save() {
    const patch: ProjectPatch = {};
    if (name.trim() !== project.name) patch.name = name.trim();
    if (description !== (project.description ?? '')) patch.description = description;
    if (status !== project.status) patch.status = status;
    if (color !== (project.color ?? '')) patch.color = color;
    if (agent !== (project.default_agent ?? '')) patch.default_agent = agent;
    if (due !== (project.due ?? '')) patch.due = due;
    if (Object.keys(patch).length === 0) {
      onCancel();
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await onSave(patch);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setSaving(false);
    }
  }

  return (
    <form
      aria-label={`Edit project ${project.name}`}
      onSubmit={(e) => { e.preventDefault(); void save(); }}
      onClick={(e) => e.stopPropagation()}
      style={{ padding: 8, borderRadius: 4, background: 'var(--ink)', border: '1px solid var(--hairline-soft)' }}
    >
      <label style={{ ...label, marginTop: 0 }} htmlFor="project-name">Name</label>
      <input id="project-name" autoFocus value={name} onChange={(e) => setName(e.target.value)} style={input} />

      <label style={label} htmlFor="project-description">Description</label>
      <textarea id="project-description" rows={2} value={description} onChange={(e) => setDescription(e.target.value)} style={{ ...input, resize: 'vertical' }} />

      <label style={label} htmlFor="project-status">Status</label>
      <select id="project-status" value={status} onChange={(e) => setStatus(e.target.value as ProjectStatus)} style={input}>
        {PROJECT_STATUSES.map((s) => <option key={s.value} value={s.value}>{s.label}</option>)}
      </select>

      <span style={label}>Color</span>
      <div role="group" aria-label="Color" style={{ display: 'flex', flexWrap: 'wrap', gap: 6, alignItems: 'center' }}>
        <button type="button" aria-label="No color" aria-pressed={color === ''} onClick={() => setColor('')}
          style={{ width: 18, height: 18, borderRadius: '50%', border: '1px dashed var(--fg-4)', background: 'transparent', cursor: 'pointer', outline: color === '' ? '2px solid var(--accent)' : 'none' }} />
        {Object.entries(PROJECT_COLORS).map(([n, css]) => (
          <button key={n} type="button" aria-label={n} aria-pressed={color === n} onClick={() => setColor(n)}
            style={{ width: 18, height: 18, borderRadius: '50%', border: 'none', background: css, cursor: 'pointer', outline: color === n ? '2px solid var(--accent)' : 'none' }} />
        ))}
        <input aria-label="Custom color" placeholder="#rrggbb" value={color in PROJECT_COLORS ? '' : color}
          onChange={(e) => setColor(e.target.value.trim())} style={{ ...input, width: 80 }} />
      </div>

      <label style={label} htmlFor="project-agent">Default agent</label>
      <input id="project-agent" value={agent} onChange={(e) => setAgent(e.target.value)} placeholder="None" style={input} />

      <label style={label} htmlFor="project-due">Deadline</label>
      <input id="project-due" type="date" value={due} onChange={(e) => setDue(e.target.value)} style={input} />

      {error && <div role="alert" style={{ marginTop: 8, fontSize: 12, color: 'var(--rose, #bf616a)' }}>{error}</div>}

      <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 10 }}>
        <button type="button" onClick={onCancel} style={{ background: 'transparent', border: 'none', color: 'var(--fg-4)', cursor: 'pointer' }}>Cancel</button>
        <button type="submit" disabled={saving} style={{ background: 'transparent', border: 'none', color: 'var(--accent)', cursor: 'pointer', fontWeight: 600 }}>Save</button>
      </div>
    </form>
  );
}
