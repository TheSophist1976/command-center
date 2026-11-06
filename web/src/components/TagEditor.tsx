import { useEffect, useRef, useState } from 'react';
import { Plus, X } from 'lucide-react';

/** Stable colour per tag name, so the same tag looks the same everywhere. */
export function tagColor(name: string): { bg: string; fg: string; border: string } {
  let h = 0;
  for (const ch of name.toLowerCase()) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return { bg: `hsla(${h}, 55%, 60%, 0.16)`, fg: `hsl(${h}, 60%, 78%)`, border: `hsla(${h}, 55%, 60%, 0.4)` };
}

interface TagEditorProps {
  id?: string;
  tags: string[];
  /** Every tag in use across all tasks, for suggestions. */
  allTags: string[];
  /** Replace this task's tags. */
  onChange: (tags: string[]) => void;
  /** Register a new tag (stored independently of tasks). */
  onCreateTag: (name: string) => Promise<string | undefined>;
  /** Remove a tag from every task. */
  onDeleteTag: (name: string) => void;
}

export function TagChip({ name, onRemove }: { name: string; onRemove?: () => void }) {
  const c = tagColor(name);
  return (
    <span
      style={{
        display: 'inline-flex', alignItems: 'center', gap: 4, padding: '2px 8px', borderRadius: 999, fontSize: 12,
        background: c.bg, color: c.fg, border: `1px solid ${c.border}`, maxWidth: '100%',
      }}
    >
      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{name}</span>
      {onRemove && (
        <button
          type="button"
          title={`Remove tag “${name}”`}
          onClick={(e) => { e.stopPropagation(); onRemove(); }}
          style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 0, color: 'inherit' }}
        >
          <X size={12} />
        </button>
      )}
    </span>
  );
}

/** Coloured chips with remove buttons, plus an input with suggestions for adding several tags. */
export function TagEditor({ id, tags, allTags, onChange, onCreateTag, onDeleteTag }: TagEditorProps) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState('');
  const [confirming, setConfirming] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!adding) return;
    function onDown(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) stop();
    }
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [adding]);

  function stop() {
    setAdding(false);
    setDraft('');
    setConfirming(null);
  }

  function add(raw: string) {
    // Allow pasting/typing "a, b, c" to add several at once.
    const incoming = raw.split(',').map((s) => s.trim()).filter(Boolean);
    setDraft('');
    void (async () => {
      const next = [...tags];
      for (const t of incoming) {
        const stored = (await onCreateTag(t)) ?? t;
        if (!next.some((x) => x.toLowerCase() === stored.toLowerCase())) next.push(stored);
      }
      if (next.length !== tags.length) onChange(next);
    })();
  }

  const q = draft.trim().toLowerCase();
  const suggestions = allTags.filter(
    (t) => !tags.some((x) => x.toLowerCase() === t.toLowerCase()) && (!q || t.toLowerCase().includes(q)),
  );

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, alignItems: 'center', minHeight: 22 }}>
        {tags.map((t) => (
          <TagChip key={t} name={t} onRemove={() => onChange(tags.filter((x) => x !== t))} />
        ))}
        {!adding && (
          <button
            id={id}
            type="button"
            onClick={() => setAdding(true)}
            style={{
              display: 'inline-flex', alignItems: 'center', gap: 3, padding: '2px 8px', borderRadius: 999, fontSize: 12,
              background: 'transparent', border: '1px dashed var(--hairline)', color: 'var(--fg-4)', cursor: 'pointer',
            }}
          >
            <Plus size={12} /> {tags.length ? 'Add' : 'Add tag'}
          </button>
        )}
      </div>
      {adding && (
        <div
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Escape') stop();
          }}
          style={{
            position: 'absolute', top: '100%', left: 0, zIndex: 50, marginTop: 4, minWidth: 220,
            background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 6,
            boxShadow: '0 4px 16px rgba(0,0,0,0.4)', padding: 6, display: 'flex', flexDirection: 'column', gap: 6,
          }}
        >
          <input
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ',') { e.preventDefault(); add(draft); }
              else if (e.key === 'Backspace' && !draft && tags.length) onChange(tags.slice(0, -1));
            }}
            placeholder="Type a tag, press Enter"
            style={{
              background: 'var(--ink)', border: '1px solid var(--accent)', borderRadius: 4, color: 'var(--fg-1)',
              fontSize: 13, padding: '4px 6px',
            }}
          />
          {suggestions.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2, maxHeight: 180, overflowY: 'auto' }}>
              <span style={{ fontSize: 11, color: 'var(--fg-4)' }}>Existing tags</span>
              {suggestions.map((t) =>
                confirming === t ? (
                  <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12.5, color: 'var(--fg-2)' }}>
                    <span style={{ flex: 1 }}>Delete “{t}” from all tasks?</span>
                    <button type="button" onClick={() => { onDeleteTag(t); setConfirming(null); }}
                      style={{ background: 'transparent', border: 'none', color: 'var(--rose, #bf616a)', cursor: 'pointer', fontWeight: 600 }}>
                      Delete
                    </button>
                    <button type="button" onClick={() => setConfirming(null)}
                      style={{ background: 'transparent', border: 'none', color: 'var(--fg-4)', cursor: 'pointer' }}>
                      Cancel
                    </button>
                  </div>
                ) : (
                  <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <button type="button" onClick={() => add(t)} style={{ background: 'transparent', border: 'none', cursor: 'pointer', padding: 0 }}>
                      <TagChip name={t} />
                    </button>
                    <span style={{ flex: 1 }} />
                    <button
                      type="button"
                      title={`Delete tag “${t}” everywhere`}
                      onClick={() => setConfirming(t)}
                      style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex', padding: 2, color: 'var(--fg-4)' }}
                    >
                      <X size={13} />
                    </button>
                  </div>
                ),
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
