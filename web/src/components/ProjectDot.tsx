import { projectCssColor } from '../projects';

/** A small colored dot for a project; renders nothing when the project has no color. */
export function ProjectDot({ color, size = 8 }: { color?: string | null; size?: number }) {
  const css = projectCssColor(color);
  if (!css) return null;
  return (
    <span
      data-testid="project-dot"
      aria-hidden="true"
      style={{ flex: 'none', width: size, height: size, borderRadius: '50%', background: css, display: 'inline-block' }}
    />
  );
}
