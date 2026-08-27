import type { ReactNode, CSSProperties } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'cyan' | 'ghost';
type ButtonSize = 'sm' | 'md' | 'lg';

const sizeStyles: Record<ButtonSize, CSSProperties> = {
  sm: { height: 34, padding: '0 14px', fontSize: 13, gap: 6 },
  md: { height: 40, padding: '0 18px', fontSize: 14, gap: 8 },
  lg: { height: 48, padding: '0 24px', fontSize: 16, gap: 10 },
};

const variantStyles: Record<ButtonVariant, CSSProperties> = {
  primary: { background: 'var(--magenta)', color: 'var(--ink)', border: '1px solid transparent' },
  secondary: { background: 'transparent', color: 'var(--fg-1)', border: '1px solid var(--hairline)' },
  cyan: { background: 'var(--cyan)', color: 'var(--ink)', border: '1px solid transparent' },
  ghost: { background: 'transparent', color: 'var(--magenta)', border: '1px solid transparent' },
};

interface ButtonProps {
  children: ReactNode;
  variant?: ButtonVariant;
  size?: ButtonSize;
  onClick?: () => void;
  disabled?: boolean;
}

export function Button({ children, variant = 'primary', size = 'sm', onClick, disabled }: ButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        borderRadius: 'var(--radius-sm)',
        fontFamily: 'var(--font-display)',
        fontWeight: 600,
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.5 : 1,
        ...sizeStyles[size],
        ...variantStyles[variant],
      }}
    >
      {children}
    </button>
  );
}
