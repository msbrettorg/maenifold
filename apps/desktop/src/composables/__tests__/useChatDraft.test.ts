// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { effectScope, nextTick, ref } from 'vue';
import { useChatDraft } from '../useChatDraft';

describe('useChatDraft', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
    localStorage.clear();
  });

  it('saves draft to localStorage after debounce flush', async () => {
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    inputText.value = 'hello world';
    await nextTick();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBeNull();

    vi.runAllTimers();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBe('hello world');

    scope.stop();
  });

  it('seeds input from pre-existing localStorage draft on init', async () => {
    localStorage.setItem('maenifold.chatDraft.ses-1', 'saved draft text');
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    expect(inputText.value).toBe('saved draft text');

    scope.stop();
  });

  it('clears localStorage draft on clearDraft (simulates send)', async () => {
    localStorage.setItem('maenifold.chatDraft.ses-1', 'unsent message');
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText, clearDraft } = scope.run(() => useChatDraft(sid))!;

    expect(inputText.value).toBe('unsent message');
    clearDraft();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBeNull();

    scope.stop();
  });

  it('loads draft for new session when session switches', async () => {
    localStorage.setItem('maenifold.chatDraft.ses-2', 'second session draft');
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    expect(inputText.value).toBe('');

    sid.value = 'ses-2';
    await nextTick();
    expect(inputText.value).toBe('second session draft');

    scope.stop();
  });

  it('drafts are keyed per session and do not bleed across sessions', async () => {
    localStorage.setItem('maenifold.chatDraft.ses-A', 'draft for A');
    const sid = ref<string | null>('ses-B');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    expect(inputText.value).toBe('');
    expect(localStorage.getItem('maenifold.chatDraft.ses-A')).toBe('draft for A');
    expect(localStorage.getItem('maenifold.chatDraft.ses-B')).toBeNull();

    scope.stop();
  });

  it('removes localStorage entry when input is cleared', async () => {
    localStorage.setItem('maenifold.chatDraft.ses-1', 'some draft');
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    inputText.value = '';
    await nextTick();
    vi.runAllTimers();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBeNull();

    scope.stop();
  });

  it('flushes pending draft for old session before switching', async () => {
    const sid = ref<string | null>('ses-1');
    const scope = effectScope();
    const { inputText } = scope.run(() => useChatDraft(sid))!;

    inputText.value = 'typed before switch';
    await nextTick();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBeNull();

    sid.value = 'ses-2';
    await nextTick();
    expect(localStorage.getItem('maenifold.chatDraft.ses-1')).toBe('typed before switch');

    scope.stop();
  });
});
