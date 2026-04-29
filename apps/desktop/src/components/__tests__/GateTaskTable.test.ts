// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import GateTaskTable from '../GateTaskTable.vue';
import type { GateTask } from '../../lib/gate';

const task: GateTask = {
  taskId: 'T-238.49',
  canonicalTaskId: 'T-238.49',
  title: 'P0 Add Trouter re-registration loop to prevent silent message loss',
  pipeline: 'sdd',
  state: 'done',
  stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
  attempts: 0,
  location: 'done',
  path: '/repo/done/T-238.49.md',
  issue: '',
};

describe('GateTaskTable', () => {
  it('renders backend task titles and pipeline labels literally', () => {
    const wrapper = mount(GateTaskTable, {
      props: {
        tasks: [task],
        selectedTaskKey: '',
        sortColumn: 'id',
        sortDirection: 'asc',
      },
    });

    const text = wrapper.text();
    expect(text).toContain('P0 Add Trouter re-registration loop to prevent silent message loss');
    expect(text).toContain('sdd');
    expect(text).not.toContain('[error] T-238.49');
    expect(text).not.toContain('sdd done');
    expect(text).not.toContain('unknown done');
  });
});
