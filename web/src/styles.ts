import type { CSSProperties } from 'react';

/** Uppercase section label: "Due window", "Notes", "Description"… */
export const sectionLabelStyle: CSSProperties = {
  fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 11, letterSpacing: '0.08em',
  textTransform: 'uppercase', color: 'var(--fg-5)',
};

/** Desktop table column widths, shared by the header row and the task rows. */
export const COL = { check: 18, id: 30, priority: 72, flags: 44, status: 136, agent: 110, due: 112, effort: 44 };
