// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';

import SessionRowActions from '../SessionRowActions.vue';
import type { SavedSession } from '../../lib/types';

function createSavedSession(overrides: Partial<SavedSession> = {}): SavedSession {
  return {
    id: 'saved-test',
    agentName: 'architect',
    sessionId: 'acp-remote-target',
    title: 'Saved session',
    lastUpdated: Date.now(),
    cwd: '/repo',
    supportsLoadSession: true,
    ...overrides,
  };
}

function mountActions(deleteTitle = "Delete 'Saved session'?") {
  const pinia = createPinia();
  setActivePinia(pinia);

  const wrapper = mount(SessionRowActions, {
    attachTo: document.body,
    props: {
      session: createSavedSession(),
      deleteTitle,
    },
    global: {
      plugins: [pinia],
    },
  });

  return wrapper;
}

function bodyButton(label: string): HTMLButtonElement {
  const button = Array.from(document.body.querySelectorAll('button')).find(
    (item) => item.textContent?.trim() === label,
  );
  if (!(button instanceof HTMLButtonElement)) throw new Error(`Missing ${label} button`);
  return button;
}

function openDialogContent(): Element | null {
  return document.body.querySelector('[data-slot="alert-dialog-content"][data-state="open"]');
}

beforeEach(() => {
  document.body.innerHTML = '';
  window.localStorage.clear();
});

describe('SessionRowActions delete confirmation', () => {
  it('opens the AlertDialog from the delete trigger', async () => {
    const deleteTitle = "Delete 'Saved session'?";
    const wrapper = mountActions(deleteTitle);

    await wrapper.get(`button[aria-label="${deleteTitle}"]`).trigger('click');
    await nextTick();

    expect(document.body.textContent).toContain(deleteTitle);
    expect(document.body.textContent).toContain("the agent's server-side session is unaffected");
    expect(openDialogContent()).not.toBeNull();
    wrapper.unmount();
  });

  it('closes on Cancel without emitting delete', async () => {
    const deleteTitle = "Delete 'Saved session'?";
    const wrapper = mountActions(deleteTitle);

    await wrapper.get(`button[aria-label="${deleteTitle}"]`).trigger('click');
    await nextTick();
    bodyButton('Cancel').click();
    await nextTick();

    expect(openDialogContent()).toBeNull();
    expect(wrapper.emitted('delete')).toBeUndefined();
    wrapper.unmount();
  });

  it('emits delete with the session id on Confirm', async () => {
    const deleteTitle = "Delete 'Saved session'?";
    const wrapper = mountActions(deleteTitle);

    await wrapper.get(`button[aria-label="${deleteTitle}"]`).trigger('click');
    await nextTick();
    bodyButton('Confirm').click();
    await nextTick();

    expect(wrapper.emitted('delete')?.[0]?.[0]).toBe('saved-test');
    wrapper.unmount();
  });
});
