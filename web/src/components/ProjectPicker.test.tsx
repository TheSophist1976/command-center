import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProjectPicker } from './ProjectPicker';
import { indexProjects } from '../projects';
import type { Project } from '../types';

const project = (over: Partial<Project> & { name: string }): Project => ({ status: 'active', ...over });

function setup(over: Partial<Parameters<typeof ProjectPicker>[0]> = {}) {
  const meta = indexProjects([
    project({ name: 'Lab', color: 'teal', description: 'Racks', default_agent: 'Home' }),
    project({ name: 'Paused', status: 'on-hold' }),
    project({ name: 'Old', status: 'archived' }),
  ]);
  const props = {
    value: '',
    projects: ['Lab', 'Paused'],
    onSave: vi.fn(),
    onCreateProject: vi.fn(),
    onDeleteProject: vi.fn(),
    meta,
    onUpdateProject: vi.fn().mockResolvedValue(undefined),
    ...over,
  };
  render(<ProjectPicker {...props} />);
  return props;
}

describe('ProjectPicker metadata', () => {
  it('shows a color dot for colored projects and a badge for non-active ones', async () => {
    setup();
    await userEvent.click(screen.getByText('No project'));
    expect(screen.getAllByTestId('project-dot')).toHaveLength(1);
    expect(screen.getByText('on-hold')).toBeInTheDocument();
  });

  it('hides archived projects until "Show archived" is ticked', async () => {
    setup();
    await userEvent.click(screen.getByText('No project'));
    expect(screen.queryByText('Old')).not.toBeInTheDocument();
    await userEvent.click(screen.getByLabelText('Show archived'));
    expect(screen.getByText('Old')).toBeInTheDocument();
  });

  it('saves only the changed fields from the edit form', async () => {
    const props = setup();
    await userEvent.click(screen.getByText('No project'));
    await userEvent.click(screen.getByTitle('Edit project “Lab”'));
    await userEvent.selectOptions(screen.getByLabelText('Status'), 'archived');
    await userEvent.click(screen.getByRole('button', { name: 'blue' }));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(props.onUpdateProject).toHaveBeenCalledWith('Lab', { status: 'archived', color: 'blue' });
  });

  it('sends a new name to rename and clears a field with an empty string', async () => {
    const props = setup();
    await userEvent.click(screen.getByText('No project'));
    await userEvent.click(screen.getByTitle('Edit project “Lab”'));
    await userEvent.clear(screen.getByLabelText('Name'));
    await userEvent.type(screen.getByLabelText('Name'), 'Rack');
    await userEvent.clear(screen.getByLabelText('Default agent'));
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(props.onUpdateProject).toHaveBeenCalledWith('Lab', { name: 'Rack', default_agent: '' });
  });

  it('shows the server error and stays open when saving fails', async () => {
    setup({ onUpdateProject: vi.fn().mockRejectedValue(new Error("Project 'Paused' already exists")) });
    await userEvent.click(screen.getByText('No project'));
    await userEvent.click(screen.getByTitle('Edit project “Lab”'));
    await userEvent.clear(screen.getByLabelText('Name'));
    await userEvent.type(screen.getByLabelText('Name'), 'Paused');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent("Project 'Paused' already exists");
    expect(screen.getByRole('form', { name: 'Edit project Lab' })).toBeInTheDocument();
  });
});
