import { useRef, useState, type CSSProperties, type MouseEvent, type ReactNode } from 'react';
import ReactMarkdown, { type Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Check, Copy } from 'lucide-react';
import '../markdown.css';

/** Inline markdown keeps emphasis, code and links but drops block structure. */
const INLINE_ELEMENTS = ['p', 'a', 'strong', 'em', 'del', 'code', 'br'];

// Links open in a new tab and stop click propagation so a link inside a
// click-to-edit field (see EditableField) opens instead of entering edit mode.
const linkComponent: Components['a'] = ({ href, children }) => (
  <a href={href} target="_blank" rel="noopener noreferrer" onClick={(e) => e.stopPropagation()}>
    {children}
  </a>
);

const blockComponents: Components = {
  a: linkComponent,
  pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
};

const inlineComponents: Components = {
  a: linkComponent,
  p: ({ children }) => <>{children}</>,
};

function CodeBlock({ children }: { children?: ReactNode }) {
  const ref = useRef<HTMLPreElement>(null);
  return (
    <div className="md-code">
      <pre ref={ref}>{children}</pre>
      <CopyButton text={() => ref.current?.textContent ?? ''} title="Copy code" className="md-code-copy" />
    </div>
  );
}

interface CopyButtonProps {
  /** The text to copy, or a function returning it at click time. */
  text: string | (() => string);
  title?: string;
  /** Show a "Copy" / "Copied" label next to the icon. */
  label?: boolean;
  className?: string;
  style?: CSSProperties;
}

/** Copies `text` to the clipboard; never lets the click reach a parent (e.g. a click-to-edit field). */
export function CopyButton({ text, title = 'Copy text', label, className, style }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  async function copy(e: MouseEvent) {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(typeof text === 'function' ? text() : text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard unavailable — no-op
    }
  }

  const Icon = copied ? Check : Copy;
  return (
    <button
      type="button"
      onClick={copy}
      title={copied ? 'Copied' : title}
      aria-label={title}
      className={['md-copy', className].filter(Boolean).join(' ')}
      style={style}
    >
      <Icon size={12} />
      {label && <span>{copied ? 'Copied' : 'Copy'}</span>}
    </button>
  );
}

interface MarkdownProps {
  text: string;
  /** Render inside a line of text (titles, summaries): no paragraphs, lists or headings. */
  inline?: boolean;
  /** Show a copy button for the raw markdown in the top-right corner. */
  copyable?: boolean;
  className?: string;
}

/**
 * The one way the UI renders user- or agent-written text: GitHub-flavoured
 * markdown with clickable links (bare URLs included) and copy buttons on
 * code blocks. Styling lives in markdown.css; see docs/markdown-style-guide.md
 * for how agents should write text that renders well here.
 */
export function Markdown({ text, inline, copyable, className }: MarkdownProps) {
  if (inline) {
    return (
      <span className={['md', 'md-inline', className].filter(Boolean).join(' ')}>
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={inlineComponents}
          allowedElements={INLINE_ELEMENTS}
          unwrapDisallowed
        >
          {text}
        </ReactMarkdown>
      </span>
    );
  }

  return (
    <div className={['md', copyable && 'md-copyable', className].filter(Boolean).join(' ')}>
      {copyable && <CopyButton text={text} className="md-body-copy" />}
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={blockComponents}>
        {text}
      </ReactMarkdown>
    </div>
  );
}
