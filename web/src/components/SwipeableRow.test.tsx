import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { SwipeableRow } from './SwipeableRow';

const today = new Date('2026-11-15T00:00:00');

function setup(over: Partial<Parameters<typeof SwipeableRow>[0]> = {}) {
  const props = {
    taskId: 7,
    today,
    done: false,
    swipeOpen: false,
    onSwipeOpenChange: vi.fn(),
    onSelect: vi.fn(),
    onToggleDone: vi.fn(),
    onQuickDue: vi.fn(),
    ...over,
  };
  render(<SwipeableRow {...props} asButton><span>Row content</span></SwipeableRow>);
  return { props, surface: screen.getByRole('button', { name: 'Row content' }) };
}

/** Drag the row horizontally by `dx` (and vertically by `dy`) the way a finger would. */
function drag(el: HTMLElement, dx: number, dy = 0) {
  fireEvent.pointerDown(el, { clientX: 200, clientY: 100, pointerId: 1 });
  fireEvent.pointerMove(el, { clientX: 200 + dx / 2, clientY: 100 + dy / 2, pointerId: 1 });
  fireEvent.pointerMove(el, { clientX: 200 + dx, clientY: 100 + dy, pointerId: 1 });
  fireEvent.pointerUp(el, { clientX: 200 + dx, clientY: 100 + dy, pointerId: 1 });
}

describe('SwipeableRow', () => {
  it('marks the task done when swiped far enough to the right', () => {
    const { props, surface } = setup();
    drag(surface, 120);
    expect(props.onToggleDone).toHaveBeenCalledTimes(1);
    expect(props.onSwipeOpenChange).toHaveBeenCalledWith(false);
  });

  it('does not act on a short swipe right', () => {
    const { props, surface } = setup();
    drag(surface, 40);
    expect(props.onToggleDone).not.toHaveBeenCalled();
  });

  it('reveals the due-date buttons when swiped far enough to the left', () => {
    const { props, surface } = setup();
    drag(surface, -150);
    expect(props.onSwipeOpenChange).toHaveBeenCalledWith(true);
    expect(props.onToggleDone).not.toHaveBeenCalled();
  });

  it('does not reveal them on a short swipe left', () => {
    const { props, surface } = setup();
    drag(surface, -40);
    expect(props.onSwipeOpenChange).toHaveBeenCalledWith(false);
  });

  it('sets the due date from the revealed buttons and closes them', () => {
    const { props } = setup({ swipeOpen: true });
    fireEvent.click(screen.getByRole('button', { name: /Tmrw/ }));
    expect(props.onQuickDue).toHaveBeenCalledWith(1);
    expect(props.onSwipeOpenChange).toHaveBeenCalledWith(false);
    fireEvent.click(screen.getByRole('button', { name: /\+1 wk/ }));
    expect(props.onQuickDue).toHaveBeenCalledWith(7);
  });

  it('offers Today, Tomorrow and +1 week with their dates', () => {
    setup({ swipeOpen: true });
    expect(screen.getByRole('button', { name: /Today\s*Nov 15/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Tmrw\s*Nov 16/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /\+1 wk\s*Nov 22/ })).toBeInTheDocument();
  });

  it('opens the task on a tap', () => {
    const { props, surface } = setup();
    fireEvent.click(surface);
    expect(props.onSelect).toHaveBeenCalledTimes(1);
  });

  it('does not open the task when a swipe ends, but a tap after it does', () => {
    const { props, surface } = setup();
    drag(surface, 40);
    fireEvent.click(surface);
    expect(props.onSelect).not.toHaveBeenCalled();
    fireEvent.click(surface);
    expect(props.onSelect).toHaveBeenCalledTimes(1);
  });

  it('closes an open row on a tap instead of opening the task', () => {
    const { props, surface } = setup({ swipeOpen: true });
    fireEvent.click(surface);
    expect(props.onSwipeOpenChange).toHaveBeenCalledWith(false);
    expect(props.onSelect).not.toHaveBeenCalled();
  });

  it('leaves a mostly-vertical drag to the scroll', () => {
    const { props, surface } = setup();
    drag(surface, 20, 120);
    expect(props.onToggleDone).not.toHaveBeenCalled();
    expect(props.onSwipeOpenChange).not.toHaveBeenCalled();
  });

  it('says Reopen for a done task', () => {
    setup({ done: true });
    expect(screen.getByText('Reopen')).toBeInTheDocument();
  });

  it('can be activated from the keyboard', () => {
    const { props, surface } = setup();
    fireEvent.keyDown(surface, { key: 'Enter' });
    expect(props.onSelect).toHaveBeenCalledTimes(1);
  });
});
