import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ViewToggle } from './ViewToggle';

describe('ViewToggle', () => {
  it('marks the current view and shows both counts', () => {
    render(<ViewToggle view="next" onChange={() => {}} nextCount={7} allCount={42} />);
    expect(screen.getByRole('tab', { name: /Next/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'false');
    expect(screen.getByLabelText('7 tasks')).toBeInTheDocument();
    expect(screen.getByLabelText('42 tasks')).toBeInTheDocument();
  });

  it('reports the view that was picked', async () => {
    const onChange = vi.fn();
    render(<ViewToggle view="next" onChange={onChange} nextCount={0} allCount={0} />);
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(onChange).toHaveBeenCalledWith('all');
  });
});
