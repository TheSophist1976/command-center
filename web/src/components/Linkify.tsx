import type { ReactNode } from 'react';

// Excludes ()[] from the URL body so a link inside "(see https://x.com)" or
// "[https://x.com]" naturally terminates at the closing bracket instead of
// swallowing it.
const URL_PATTERN = /(https?:\/\/[^\s<>"')\]]+|www\.[^\s<>"')\]]+)/gi;
const TRAILING_PUNCTUATION = /[.,;:!?'"]+$/;

/**
 * Renders `text` with any http(s)/www URLs turned into clickable links,
 * plain text everywhere else. Stops click propagation so a link inside a
 * click-to-edit field (see EditableField) opens instead of entering edit
 * mode.
 */
export function Linkify({ text }: { text: string }): ReactNode {
  if (!text) return text;

  const nodes: ReactNode[] = [];
  let cursor = 0;
  let key = 0;
  const regex = new RegExp(URL_PATTERN);
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    const start = match.index;
    let url = match[0];
    const trailing = url.match(TRAILING_PUNCTUATION);
    if (trailing) {
      url = url.slice(0, url.length - trailing[0].length);
    }
    if (url.length === 0) continue;

    if (start > cursor) {
      nodes.push(text.slice(cursor, start));
    }
    const href = url.startsWith('www.') ? `https://${url}` : url;
    nodes.push(
      <a
        key={key++}
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        onClick={(e) => e.stopPropagation()}
        style={{ color: 'var(--magenta)', textDecoration: 'underline' }}
      >
        {url}
      </a>,
    );
    cursor = start + url.length;
  }

  if (cursor < text.length) {
    nodes.push(text.slice(cursor));
  }

  return <>{nodes}</>;
}
