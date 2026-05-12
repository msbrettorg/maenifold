// @vitest-environment jsdom
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import GateViewToolbar from '../GateViewToolbar.vue';
import type { GateTask } from '../../lib/gate';

const stubs = {
  Sheet: { template: '<div><slot /></div>' },
  SheetContent: { template: '<div><slot /></div>' },
  SheetHeader: { template: '<div><slot /></div>' },
  SheetTitle: { template: '<div><slot /></div>' },
  DropdownMenu: { template: '<div><slot /></div>' },
  DropdownMenuTrigger: { template: '<div><slot /></div>' },
  DropdownMenuContent: { template: '<div><slot /></div>' },
  DropdownMenuItem: { template: '<div><slot /></div>' },
  DropdownMenuSub: { template: '<div><slot /></div>' },
  DropdownMenuSubTrigger: { template: '<div><slot /></div>' },
  DropdownMenuSubContent: { template: '<div><slot /></div>' },
  DropdownMenuSeparator: { template: '<div />' },
  Input: { template: '<input />', props: ['modelValue'] },
};

function makeTask(overrides: Partial<GateTask> = {}): GateTask {
  return {
    taskId: 'T-1',
    canonicalTaskId: 'T-1',
    title: 'Test task',
    pipeline: 'sdd',
    state: 'pending',
    stages: ['pending', 'swe', 'red-team', 'blue-team', 'done'],
    attempts: 0,
    location: 'todo',
    path: '/repo/todo/T-1.md',
    issue: '',
    ...overrides,
  };
}

const baseProps = {
  hasGateRoot: true,
  todoCount: 3,
  doneCount: 1,
  totalCount: 4,
  shownCount: 4,
  lastRefreshLabel: '',
  selectedGateRoot: '/repo',
  selectedLocation: 'todo',
  gateRoots: ['/repo'],
  isReady: true,
  isLoading: false,
  tasks: [],
  sprintOptions: [],
  selectedSprints: new Set<number>(),
  searchText: '',
};

describe('GateViewToolbar — sprint chips', () => {
  it('renders a chip for each sprint option', () => {
    const wrapper = mount(GateViewToolbar, {
      props: { ...baseProps, sprintOptions: [251, 250, 249] },
      global: { stubs },
    });
    const chips = wrapper.findAll('[aria-pressed]');
    expect(chips).toHaveLength(3);
    expect(chips[0].text()).toBe('S251');
    expect(chips[1].text()).toBe('S250');
    expect(chips[2].text()).toBe('S249');
  });

  it('chip reflects active state when sprint is selected', () => {
    const wrapper = mount(GateViewToolbar, {
      props: { ...baseProps, sprintOptions: [250, 249], selectedSprints: new Set([250]) },
      global: { stubs },
    });
    const chips = wrapper.findAll('[aria-pressed]');
    expect(chips[0].attributes('aria-pressed')).toBe('true');
    expect(chips[1].attributes('aria-pressed')).toBe('false');
  });

  it('emits toggle-sprint with the sprint number when a chip is clicked', async () => {
    const wrapper = mount(GateViewToolbar, {
      props: { ...baseProps, sprintOptions: [250] },
      global: { stubs },
    });
    await wrapper.find('[aria-pressed]').trigger('click');
    expect(wrapper.emitted('toggle-sprint')?.[0]).toEqual([250]);
  });
});

describe('GateViewToolbar — stage count badges', () => {
  it('displays correct stage counts from tasks prop', () => {
    const tasks: GateTask[] = [
      makeTask({ state: 'pending' }),
      makeTask({ state: 'swe' }),
      makeTask({ state: 'red-team' }),
      makeTask({ state: 'blue-team' }),
      makeTask({ state: 'done', location: 'done' }),
    ];
    const wrapper = mount(GateViewToolbar, {
      props: { ...baseProps, tasks },
      global: { stubs },
    });
    const text = wrapper.text();
    expect(text).toContain('Pending 1');
    expect(text).toContain('SWE 1');
    expect(text).toContain('Red 1');
    expect(text).toContain('Blue 1');
    expect(text).toContain('Done 1');
  });

  it('hides Done badge when count is zero', () => {
    const tasks: GateTask[] = [makeTask({ state: 'pending' })];
    const wrapper = mount(GateViewToolbar, {
      props: { ...baseProps, tasks },
      global: { stubs },
    });
    expect(wrapper.text()).not.toContain('Done ');
  });
});
