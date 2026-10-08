import { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { ChevronDown, ChevronRight, ExternalLink, X } from 'lucide-react';
import type { Note } from '../types';
import { Button } from './Button';
import '../noteMarkdown.css';

interface NotesSectionProps {
  notes: Note[];
  /** Resolves to the new note's slug so it can be expanded for editing, or undefined on failure. */
  onCreate: (title: string) => Promise<string | undefined> | void;
  /** Open in the server machine's external editor (Obsidian/$EDITOR) — only useful at the server. */
  onOpen: (slug: string) => void;
  onSave: (slug: string, body: string) => Promise<void>;
  onUnlink: (slug: string) => void;
  /** Larger rows and buttons for touch screens. */
  touch?: boolean;
}

export function NotesSection({ notes, onCreate, onOpen, onSave, onUnlink, touch }: NotesSectionProps) {
  const [adding, setAdding] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState('');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  function toggle(slug: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(slug)) next.delete(slug);
      else next.add(slug);
      return next;
    });
    if (editing === slug) setEditing(null);
  }

  function startEdit(n: Note) {
    setDraft(n.body);
    setSaveError(null);
    setEditing(n.slug);
  }

  async function save(slug: string) {
    setSaving(true);
    setSaveError(null);
    try {
      await onSave(slug, draft);
      setEditing(null);
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function submitNewNote() {
    const title = newTitle.trim();
    if (!title) return;
    setNewTitle('');
    setAdding(false);
    const slug = await onCreate(title);
    if (slug) {
      setExpanded((prev) => new Set(prev).add(slug));
      setDraft('');
      setSaveError(null);
      setEditing(slug);
    }
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

      {notes.map((n) => {
        const isOpen = expanded.has(n.slug);
        const isEditing = editing === n.slug;
        return (
          <div
            key={n.slug}
            style={{ border: '1px solid var(--hairline)', borderRadius: 6, overflow: 'hidden' }}
          >
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: touch ? '0 4px 0 12px' : '8px 10px',
                minHeight: touch ? 48 : undefined,
              }}
            >
              <button
                onClick={() => toggle(n.slug)}
                aria-expanded={isOpen}
                title={isOpen ? 'Collapse note' : 'Read note'}
                style={{
                  flex: 1,
                  minWidth: 0,
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
                {isOpen ? (
                  <ChevronDown size={14} style={{ flex: 'none', color: 'var(--fg-4)' }} />
                ) : (
                  <ChevronRight size={14} style={{ flex: 'none', color: 'var(--fg-4)' }} />
                )}
                <span style={{ overflowWrap: 'anywhere' }}>{n.title}</span>
              </button>
              <button
                onClick={() => onOpen(n.slug)}
                title="Open in external editor (on the server machine)"
                style={{
                  background: 'transparent', border: 'none', cursor: 'pointer', color: 'var(--fg-5)', padding: 0,
                  ...(touch ? { width: 40, height: 40, display: 'flex', alignItems: 'center', justifyContent: 'center' } : {}),
                }}
              >
                <ExternalLink size={touch ? 16 : 13} />
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

            {isOpen && (
              <div style={{ borderTop: '1px solid var(--hairline-soft)', padding: '10px 12px', display: 'flex', flexDirection: 'column', gap: 8 }}>
                {isEditing ? (
                  <>
                    <textarea
                      autoFocus
                      value={draft}
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Escape') setEditing(null);
                        if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) save(n.slug);
                      }}
                      placeholder="Write markdown…"
                      style={{
                        minHeight: 180,
                        resize: 'vertical',
                        background: 'var(--ink-2)',
                        border: '1px solid var(--hairline)',
                        borderRadius: 4,
                        color: 'var(--fg-1)',
                        fontFamily: 'var(--font-mono)',
                        fontSize: touch ? 14 : 12.5,
                        lineHeight: 1.5,
                        padding: '8px 10px',
                      }}
                    />
                    {saveError && <div style={{ fontSize: 12, color: 'var(--danger)' }}>{saveError}</div>}
                    <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                      <Button size="sm" onClick={() => save(n.slug)} disabled={saving}>
                        {saving ? 'Saving…' : 'Save'}
                      </Button>
                      <Button size="sm" variant="secondary" onClick={() => setEditing(null)} disabled={saving}>
                        Cancel
                      </Button>
                      <span style={{ fontSize: 11, color: 'var(--fg-5)' }}>⌘/Ctrl+Enter to save</span>
                    </div>
                  </>
                ) : (
                  <>
                    {n.body.trim() ? (
                      <div className="note-md">
                        <ReactMarkdown remarkPlugins={[remarkGfm]}>{n.body}</ReactMarkdown>
                      </div>
                    ) : (
                      <div style={{ fontSize: 13, color: 'var(--fg-5)', fontStyle: 'italic' }}>Empty note.</div>
                    )}
                    <div>
                      <Button size="sm" variant="secondary" onClick={() => startEdit(n)}>
                        Edit
                      </Button>
                    </div>
                  </>
                )}
              </div>
            )}
          </div>
        );
      })}

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
