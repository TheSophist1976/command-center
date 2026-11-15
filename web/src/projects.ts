import type { Project, ProjectStatus } from './types';

export const PROJECT_STATUSES: { value: ProjectStatus; label: string }[] = [
  { value: 'active', label: 'Active' },
  { value: 'on-hold', label: 'On hold' },
  { value: 'done', label: 'Done' },
  { value: 'archived', label: 'Archived' },
];

/** Named project colors; the server accepts exactly these names or `#rrggbb`. */
export const PROJECT_COLORS: Record<string, string> = {
  red: '#bf616a',
  orange: '#d08770',
  yellow: '#ebcb8b',
  green: '#a3be8c',
  teal: '#8fbcbb',
  blue: '#81a1c1',
  purple: '#b48ead',
  pink: '#d38aa8',
  gray: '#7b8394',
};

/** CSS color for a project's `color` value, or undefined when it has none. */
export function projectCssColor(color?: string | null): string | undefined {
  if (!color) return undefined;
  return PROJECT_COLORS[color] ?? (/^#[0-9a-f]{6}$/i.test(color) ? color : undefined);
}

/** Lowercase-name → project, matching how the server compares project names. */
export function indexProjects(projects: Project[]): Map<string, Project> {
  return new Map(projects.map((p) => [p.name.toLowerCase(), p]));
}
