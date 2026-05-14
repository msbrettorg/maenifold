import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import ScheduleFormDialog from '@/components/ScheduleFormDialog.vue';

describe('ScheduleFormDialog pause warning', () => {
  it('renders pause-active warning banner when isPausedAll=true', () => {
    const wrapper = mount(ScheduleFormDialog, {
      global: {
        stubs: {
          Dialog: { template: '<div><slot /></div>' },
          DialogContent: { template: '<div><slot /></div>' },
          DialogDescription: { template: '<p><slot /></p>' },
          DialogFooter: { template: '<div><slot /></div>' },
          DialogHeader: { template: '<div><slot /></div>' },
          DialogTitle: { template: '<h2><slot /></h2>' },
          Select: { template: '<div><slot /></div>' },
          SelectContent: { template: '<div><slot /></div>' },
          SelectItem: { template: '<div><slot /></div>' },
          SelectTrigger: { template: '<div><slot /></div>' },
          SelectValue: { template: '<span />' },
          Switch: { template: '<button />' },
          ScheduleCronPreview: { template: '<div />' },
        },
      },
      props: {
        open: true,
        form: {
          id: 'daily',
          schedule: 'every day at 9am',
          agent: 'swe',
          prompt: 'Run scheduled maintenance',
          description: '',
           enabled: true,
           autoGrantPermissions: false,
           allowedTools: '',
           mcpAction: '',
           mcpParameters: '',
         },
        mode: 'create',
        title: 'Create schedule',
        description: 'Create a recurring task',
        agentNames: ['swe'],
        isSubmitting: false,
        canSubmit: true,
        schedulePlatform: {
          platform: 'macos',
          supportsEnabledSchedules: true,
          scheduler: 'launchd',
          label: 'Supported',
          description: 'Supported',
          unsupportedEnabledMessage: '',
        },
        isPausedAll: true,
      },
    });

    expect(wrapper.text()).toContain('⚠ Pause-all is active. Saving with "Enabled" turned on will fail. Resume schedules first to enable individual jobs.');
  });

  it('dialog renders mcpAction Select and mcpParameters Textarea', () => {
    const wrapper = mount(ScheduleFormDialog, {
      global: {
        stubs: {
          Dialog: { template: '<div><slot /></div>' },
          DialogContent: { template: '<div><slot /></div>' },
          DialogDescription: { template: '<p><slot /></p>' },
          DialogFooter: { template: '<div><slot /></div>' },
          DialogHeader: { template: '<div><slot /></div>' },
          DialogTitle: { template: '<h2><slot /></h2>' },
          Select: { template: '<div><slot /></div>' },
          SelectContent: { template: '<div><slot /></div>' },
          SelectItem: { props: ['value'], template: '<div>{{ value }}<slot /></div>' },
          SelectTrigger: { template: '<div><slot /></div>' },
          SelectValue: { template: '<span />' },
          Switch: { template: '<button />' },
          ScheduleCronPreview: { template: '<div />' },
        },
      },
      props: {
        open: true,
        form: {
          id: 'daily',
          schedule: 'every day at 9am',
          agent: 'swe',
          prompt: '',
          description: '',
          enabled: true,
          autoGrantPermissions: false,
          allowedTools: '',
          mcpAction: 'maenifold.gate.gate_Status',
          mcpParameters: '{}',
        },
        mode: 'create',
        title: 'Create schedule',
        description: 'Create a recurring task',
        agentNames: ['swe'],
        isSubmitting: false,
        canSubmit: true,
        schedulePlatform: {
          platform: 'macos',
          supportsEnabledSchedules: true,
          scheduler: 'launchd',
          label: 'Supported',
          description: 'Supported',
          unsupportedEnabledMessage: '',
        },
        isPausedAll: false,
      },
    });

    expect(wrapper.text()).toContain('MCP action');
    expect(wrapper.text()).toContain('maenifold.gate.gate_Status');
    expect(wrapper.find('#schedule-mcp-parameters').exists()).toBe(true);
  });

  it('dialog disables save when mcpAction set and parameters invalid JSON', () => {
    const wrapper = mount(ScheduleFormDialog, {
      global: {
        stubs: {
          Dialog: { template: '<div><slot /></div>' },
          DialogContent: { template: '<div><slot /></div>' },
          DialogDescription: { template: '<p><slot /></p>' },
          DialogFooter: { template: '<div><slot /></div>' },
          DialogHeader: { template: '<div><slot /></div>' },
          DialogTitle: { template: '<h2><slot /></h2>' },
          Select: { template: '<div><slot /></div>' },
          SelectContent: { template: '<div><slot /></div>' },
          SelectItem: { template: '<div><slot /></div>' },
          SelectTrigger: { template: '<div><slot /></div>' },
          SelectValue: { template: '<span />' },
          Switch: { template: '<button />' },
          ScheduleCronPreview: { template: '<div />' },
        },
      },
      props: {
        open: true,
        form: {
          id: 'daily',
          schedule: 'every day at 9am',
          agent: 'swe',
          prompt: '',
          description: '',
          enabled: true,
          autoGrantPermissions: false,
          allowedTools: '',
          mcpAction: 'maenifold.gate.gate_Status',
          mcpParameters: '{',
        },
        mode: 'create',
        title: 'Create schedule',
        description: 'Create a recurring task',
        agentNames: ['swe'],
        isSubmitting: false,
        canSubmit: false,
        schedulePlatform: {
          platform: 'macos',
          supportsEnabledSchedules: true,
          scheduler: 'launchd',
          label: 'Supported',
          description: 'Supported',
          unsupportedEnabledMessage: '',
        },
        isPausedAll: false,
      },
    });

    expect(wrapper.text()).toContain('Parameters must be a JSON object.');
    expect(wrapper.find('button[type="submit"]').attributes('disabled')).toBeDefined();
  });
});
