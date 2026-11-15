import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MobileStatusBar, StatusStrip } from './StatusStrip';
import { agentActivity, serviceSummary } from '../status';
import { makeTask } from '../test/factories';
import type { Service } from '../types';

const agents = [{ name: 'CommandCenter', dir: '/a' }, { name: 'Vault', dir: '/b' }];

function props(over: { failing?: boolean } = {}) {
  const tasks = [makeTask({ id: 58, title: 'Apple Silicon release', agent: 'CommandCenter', work_status: 'in-progress' })];
  const services = over.failing
    ? [{ ...makeTask({ id: 9, title: 'Backup vault' }), kind: 'service', run_count: 1, work_status: 'failing' } as Service]
    : [];
  return {
    activity: agentActivity(tasks, agents),
    services: serviceSummary(services),
    onEditAgent: vi.fn(),
    onOpenServices: vi.fn(),
    onOpenSettings: vi.fn(),
  };
}

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ current: '5.4.2', latest: null, update_available: false, update_supported: false }))));
});

describe('StatusStrip', () => {
  it('summarises agents and services at a glance', () => {
    render(<StatusStrip {...props({ failing: true })} />);
    expect(screen.getByText('1 running · 1 idle')).toBeInTheDocument();
    expect(screen.getByText('1 failing')).toBeInTheDocument();
  });

  it('shows healthy services when none are failing', () => {
    render(<StatusStrip {...props()} />);
    expect(screen.getByText('services ok')).toBeInTheDocument();
  });

  it('shows each agent and what it is working on when opened', async () => {
    render(<StatusStrip {...props({ failing: true })} />);
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    expect(screen.getByText('#58 Apple Silicon release')).toBeInTheDocument();
    expect(screen.getByText('Backup vault')).toBeInTheDocument();
  });

  it('edits an agent when its row is clicked, and closes the panel', async () => {
    const p = props();
    render(<StatusStrip {...p} />);
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    await userEvent.click(screen.getByRole('button', { name: /Vault/ }));
    expect(p.onEditAgent).toHaveBeenCalledWith('Vault');
    expect(screen.queryByText('Open Services…')).not.toBeInTheDocument();
  });

  it('opens the services view and settings', async () => {
    const p = props();
    render(<StatusStrip {...p} />);
    await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(p.onOpenSettings).toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    await userEvent.click(screen.getByRole('button', { name: 'Open Services…' }));
    expect(p.onOpenServices).toHaveBeenCalled();
  });

  it('closes the panel on Escape', async () => {
    render(<StatusStrip {...props()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    expect(screen.getByText('Open Services…')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByText('Open Services…')).not.toBeInTheDocument();
  });

  it('shows the running version', async () => {
    render(<StatusStrip {...props()} />);
    expect(await screen.findByText('v5.4.2')).toBeInTheDocument();
  });
});

describe('MobileStatusBar', () => {
  it('opens the detail as a bottom sheet', async () => {
    render(<MobileStatusBar {...props({ failing: true })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    expect(screen.getByRole('dialog', { name: 'Agent and service status' })).toBeInTheDocument();
    expect(screen.getByText('Backup vault')).toBeInTheDocument();
  });
});
