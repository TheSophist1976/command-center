import { ChevronDown, ChevronRight, Eye, HelpCircle, Inbox, RefreshCw, type LucideIcon } from 'lucide-react';
import type { Task } from '../types';
import { BAND_LABEL, reasonFor, type Band, type QueueBand } from '../queue';
import { dueColor, dueLabel } from '../taskFormat';
import { PriorityBadge } from './Badges';

const BAND_ICON: Record<Band, LucideIcon> = { review: Eye, input: HelpCircle, triage: Inbox, working: RefreshCw };
const BAND_COLOR: Record<Band, string> = {
  review: 'var(--cyan)', input: 'var(--citrine)', triage: 'var(--fg-3)', working: 'var(--teal)',
};

interface NextQueueProps {
  queue: QueueBand[];
  selectedId: number | null;
  onSelect: (task: Task) => void;
  today: Date;
  deferredIds: ReadonlySet<number>;
  workingExpanded: boolean;
  onToggleWorking: () => void;
  /** Phone layout: taller rows, two-line titles. */
  mobile?: boolean;
}

/** The Next view's list: bands of tasks, each row a title, why it is here, and a few chips. */
export function NextQueue({ queue, selectedId, onSelect, today, deferredIds, workingExpanded, onToggleWorking, mobile }: NextQueueProps) {
  const visible = queue.filter((b) => b.tasks.length > 0);
  const needsYou = queue.some((b) => b.band !== 'working' && b.tasks.length > 0);

  return (
    <div>
      {!needsYou && (
        <div style={{ padding: mobile ? '40px 24px 24px' : '32px 20px 20px', textAlign: 'center' }}>
          <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 18 }}>Nothing needs you.</div>
          <div style={{ marginTop: 6, fontSize: 14, color: 'var(--fg-4)' }}>
            No open tasks are unassigned, assigned to you, awaiting review, or blocked on your input.
          </div>
        </div>
      )}
      {visible.map(({ band, tasks }) => {
        const Icon = BAND_ICON[band];
        const collapsible = band === 'working';
        const expanded = !collapsible || workingExpanded;
        const header = (
          <>
            <Icon size={13} color={BAND_COLOR[band]} aria-hidden />
            <span>{BAND_LABEL[band]}</span>
            <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent)' }}>{tasks.length}</span>
            {collapsible && (
              <span style={{ marginLeft: 'auto', display: 'flex' }}>
                {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </span>
            )}
          </>
        );
        const headerStyle = {
          position: 'sticky' as const, top: 0, zIndex: 2, display: 'flex', alignItems: 'center', gap: 8, width: '100%',
          minHeight: mobile ? 40 : 34, padding: '0 16px', background: 'var(--ink-2)', border: 'none',
          borderBottom: '1px solid var(--hairline-soft)', textAlign: 'left' as const,
          fontFamily: 'var(--font-display)', fontSize: 11, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase' as const,
          color: 'var(--fg-5)',
        };
        return (
          <section key={band} aria-label={BAND_LABEL[band]}>
            {collapsible ? (
              <button type="button" onClick={onToggleWorking} aria-expanded={expanded} style={{ ...headerStyle, cursor: 'pointer' }}>
                {header}
              </button>
            ) : (
              <div style={headerStyle}>{header}</div>
            )}
            {expanded && tasks.map((t) => (
              <QueueRow
                key={t.id}
                task={t}
                band={band}
                today={today}
                selected={t.id === selectedId}
                deferred={deferredIds.has(t.id)}
                mobile={mobile}
                onSelect={() => onSelect(t)}
              />
            ))}
          </section>
        );
      })}
    </div>
  );
}

function QueueRow({ task: t, band, today, selected, deferred, mobile, onSelect }: {
  task: Task; band: Band; today: Date; selected: boolean; deferred: boolean; mobile?: boolean; onSelect: () => void;
}) {
  const Icon = BAND_ICON[band];
  const chip = { fontSize: mobile ? 12 : 11.5, padding: '1px 8px', borderRadius: 999, background: 'var(--ink-3)', color: 'var(--fg-3)' };
  const urgent = t.priority === 'critical' || t.priority === 'high';
  return (
    <button
      type="button"
      data-task-row={t.id}
      aria-current={selected ? 'true' : undefined}
      onClick={onSelect}
      style={{
        display: 'flex', alignItems: 'flex-start', gap: 12, width: '100%', textAlign: 'left', color: 'inherit', font: 'inherit',
        minHeight: mobile ? 68 : 0, padding: mobile ? '14px 16px' : '10px 16px', cursor: 'pointer', border: 'none',
        borderBottom: '1px solid var(--hairline-soft)', borderLeft: `3px solid ${selected ? 'var(--accent)' : 'transparent'}`,
        background: selected ? 'var(--accent-soft)' : 'transparent',
        opacity: deferred || band === 'working' ? 0.7 : 1,
      }}
    >
      <Icon size={mobile ? 16 : 15} color={BAND_COLOR[band]} style={{ flex: 'none', marginTop: 3 }} aria-hidden />
      <span style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 3 }}>
        <span
          style={{
            fontSize: mobile ? 15 : 14, fontWeight: band === 'working' ? 500 : 600, color: 'var(--fg-1)', lineHeight: 1.35,
            ...(mobile
              ? { display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden' }
              : { whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }),
          }}
        >
          {t.title}
        </span>
        <span style={{ fontSize: mobile ? 13 : 12.5, color: 'var(--fg-4)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {reasonFor(t, band)}
        </span>
        <span style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6, marginTop: 3 }}>
          {t.agent && t.agent.toLowerCase() !== 'human' && (
            <span style={{ ...chip, fontFamily: 'var(--font-mono)', color: 'var(--accent)' }}>{t.agent}</span>
          )}
          {t.due_date && <span style={{ ...chip, color: dueColor(t, today) }}>{dueLabel(t, today)}</span>}
          {t.project && <span style={chip}>{t.project}</span>}
          {urgent && <PriorityBadge priority={t.priority} size={10.5} />}
          {deferred && <span style={{ ...chip, fontStyle: 'italic' }}>deferred</span>}
        </span>
      </span>
      <span style={{ flex: 'none', fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--fg-5)', marginTop: 2 }}>#{t.id}</span>
    </button>
  );
}
