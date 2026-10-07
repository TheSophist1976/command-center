import { useState } from 'react';
import { ExternalLink, X } from 'lucide-react';
import type { Note } from '../types';
import { Button } from './Button';

interface NotesSectionProps {
  notes: Note[];
  onCreate: (title: string) => void;
  onOpen: (slug: string) => void;
  onUnlink: (slug: string) => void;
  /** Larger rows and buttons for touch screens. */
  touch?: boolean;
}

export function NotesSection({ notes, onCreate, onOpen, onUnlink, touch }: NotesSectionProps) {
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');

  function submitNewNote() {
    const title = newTitle.trim();
    if (!title) return;
    onCreate(title);
    setNewTitle('');
    setAdding(false);
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
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
        <div style={{ fontSize: touch ? 14 : 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>No notes yet.</div>
      )}

      {notes.map((n) => (
        <div
          key={n.slug}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            border: '1px solid var(--hairline)',
            borderRadius: 6,
            padding: touch ? '0 4px 0 12px' : '8px 10px',
            minHeight: touch ? 48 : undefined,
          }}
        >
          <button
            onClick={() => onOpen(n.slug)}
            title="Open in external editor"
            style={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              background: 'transparent',
              border: 'none',
              cursor: 'pointer',
              textAlign: 'left',
              fontSize: touch ? 14 : 13,
              fontWeight: 600,
              color: 'var(--fg-1)',
              padding: 0,
            }}
          >
            <ExternalLink size={13} style={{ flex: 'none', color: 'var(--fg-5)' }} />
            {n.title}
          </button>
          <button
            onClick={() => onUnlink(n.slug)}
            title="Unlink note"
            style={{
              background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--fg-5)', padding: 0,
              ...(touch ? { width: 40, height: 40, display: 'flex', alignItems: 'center', justifyContent: 'center' } : {}),
            }}
          >
            <X size={touch ? 16 : 14} />
          </button>
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
        <Button size={touch ? 'md' : 'sm'} variant="secondary" onClick={() => setAdding(true)}>
          + Add note
        </Button>
      )}
    </div>
  );
}
