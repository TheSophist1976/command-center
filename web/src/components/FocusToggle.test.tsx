import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { FocusToggle } from './FocusToggle';

describe('FocusToggle', () => {
  it('reflects whether Focus is on', () => {
    const { rerender } = render(<FocusToggle on={false} onChange={() => {}} hidden={0} />);
    expect(screen.getByRole('button', { name: /Focus/ })).toHaveAttribute('aria-pressed', 'false');
    rerender(<FocusToggle on onChange={() => {}} hidden={0} />);
    expect(screen.getByRole('button', { name: /Focus/ })).toHaveAttribute('aria-pressed', 'true');
  });

  it('asks for the opposite state when clicked', async () => {
    const onChange = vi.fn();
    const { rerender } = render(<FocusToggle on={false} onChange={onChange} hidden={0} />);
    await userEvent.click(screen.getByRole('button', { name: /Focus/ }));
    expect(onChange).toHaveBeenLastCalledWith(true);
    rerender(<FocusToggle on onChange={onChange} hidden={0} />);
    await userEvent.click(screen.getByRole('button', { name: /Focus/ }));
    expect(onChange).toHaveBeenLastCalledWith(false);
  });

  it('says how many tasks are hidden, only while on', () => {
    const { rerender } = render(<FocusToggle on={false} onChange={() => {}} hidden={3} />);
    expect(screen.queryByText('3 hidden')).not.toBeInTheDocument();
    rerender(<FocusToggle on onChange={() => {}} hidden={3} />);
    expect(screen.getByText('3 hidden')).toBeInTheDocument();
  });

  it('stays quiet when nothing is hidden', () => {
    render(<FocusToggle on onChange={() => {}} hidden={0} />);
    expect(screen.queryByText(/hidden/)).not.toBeInTheDocument();
  });
});
