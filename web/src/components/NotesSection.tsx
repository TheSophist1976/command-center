import { useState } from 'react';
import { X } from 'lucide-react';
import type { Note } from '../types';
import { EditableField } from './EditableField';
import { Button } from './Button';

interface NotesSectionProps {
  notes: Note[];
  onCreate: (title: string) => void;
  onEdit: (slug: string, changes: { title?: string; body?: string }) => void;
  onUnlink: (slug: string) => void;
}

export function NotesSection({ notes, onCreate, onEdit, onUnlink }: NotesSectionProps) {
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [expanded, setExpanded] = useState<string | null>(null);

  function submitNewNote() {
    const title = newTitle.trim();
    if (!title) return;
    onCreate(title);
    setNewTitle('');
    setAdding(false);
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 8 }}>
      <div
        style={{
          fontFamily: 'var(--font-display)',
          fontWeight: 700,
          fontSize: 11,
          letterSpacing: '0.08em',
          textTransform: 'uppercase',
          color: 'var(--fg-5)',
        }}
      >
        Notes
      </div>

      {notes.length === 0 && !adding && (
        <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>No notes yet.</div>
      )}

      {notes.map((n) => (
        <div key={n.slug} style={{ border: '1px solid var(--hairline)', borderRadius: 6, padding: 10 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <div
              style={{ flex: 1, cursor: 'pointer' }}
              onClick={() => setExpanded(expanded === n.slug ? null : n.slug)}
            >
              <EditableField
                value={n.title}
                onSave={(v) => v.trim() && onEdit(n.slug, { title: v.trim() })}
                display={<span style={{ fontSize: 13, fontWeight: 600 }}>{n.title}</span>}
              />
            </div>
            <button
              onClick={() => onUnlink(n.slug)}
              title="Unlink note"
              style={{ background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--fg-5)', padding: 0 }}
            >
              <X size={14} />
            </button>
          </div>
          {expanded === n.slug && (
            <div style={{ marginTop: 8 }}>
              <EditableField
                value={n.body}
                type="textarea"
                placeholder="Empty"
                onSave={(v) => onEdit(n.slug, { body: v })}
              />
            </div>
          )}
        </div>
      ))}

      {adding ? (
        <div style={{ display: 'flex', gap: 6 }}>
          <input
            autoFocus
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            placeholder="Note title"
            style={{
              flex: 1,
              background: 'var(--ink-2)',
              border: '1px solid var(--hairline)',
              borderRadius: 4,
              color: 'var(--fg-1)',
              fontSize: 13,
              padding: '4px 8px',
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') submitNewNote();
              if (e.key === 'Escape') {
                setAdding(false);
                setNewTitle('');
              }
            }}
          />
          <Button size="sm" onClick={submitNewNote}>Add</Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              setAdding(false);
              setNewTitle('');
            }}
          >
            Cancel
          </Button>
        </div>
      ) : (
        <Button size="sm" variant="secondary" onClick={() => setAdding(true)}>
          + Add note
        </Button>
      )}
    </div>
  );
}
