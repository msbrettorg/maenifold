import { describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import ChatAttachmentBar, { type PendingAttachment } from '../ChatAttachmentBar.vue';

vi.mock('vue-sonner', () => ({
  toast: { info: vi.fn() },
}));

function mountBar(pendingAttachments: PendingAttachment[] = []): VueWrapper {
  return mount(ChatAttachmentBar, {
    props: {
      isLoading: false,
      supportsImage: true,
      supportsAudio: true,
      imageTooltip: 'Attach image',
      audioTooltip: 'Attach audio',
      pendingAttachments,
    },
  });
}

function setFiles(input: HTMLInputElement, files: File[]): void {
  Object.defineProperty(input, 'files', {
    value: files,
    configurable: true,
  });
}

async function selectFiles(wrapper: VueWrapper, files: File[]): Promise<PendingAttachment[]> {
  const input = wrapper.findAll<HTMLInputElement>('input[type="file"]')[2];
  setFiles(input.element, files);

  await input.trigger('change');
  await flushPromises();
  await vi.waitFor(() => {
    expect(wrapper.emitted('add-attachments')?.[0]?.[0]).toBeDefined();
  });

  const emitted = wrapper.emitted('add-attachments')?.[0]?.[0];
  return emitted as PendingAttachment[];
}

describe('ChatAttachmentBar', () => {
  it('clicks the generic file input from the file picker button', async () => {
    const wrapper = mountBar();
    const input = wrapper.findAll<HTMLInputElement>('input[type="file"]')[2];
    const click = vi.spyOn(input.element, 'click').mockImplementation(() => {});

    await wrapper.findAll('button').find((button) => button.text().includes('File'))?.trigger('click');

    expect(click).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it('emits resource blocks for multiple generic file attachments', async () => {
    const wrapper = mountBar();
    const files = [
      new File(['pdf'], 'brief.pdf', { type: 'application/pdf' }),
      new File(['a,b'], 'data.csv', { type: 'text/csv' }),
      new File(['hello'], 'notes.txt', { type: 'text/plain' }),
      new File(['zip'], 'archive.zip', { type: 'application/zip' }),
    ];

    const attachments = await selectFiles(wrapper, files);

    expect(attachments).toHaveLength(4);
    expect(attachments.map((item) => item.kind)).toEqual(['file', 'file', 'file', 'file']);
    expect(attachments.map((item) => item.label)).toEqual([
      'brief.pdf',
      'data.csv',
      'notes.txt',
      'archive.zip',
    ]);
    expect(attachments.map((item) => item.block)).toMatchObject([
      { type: 'resource', resource: { uri: 'file://brief.pdf', mimeType: 'application/pdf' } },
      { type: 'resource', resource: { uri: 'file://data.csv', mimeType: 'text/csv' } },
      { type: 'resource', resource: { uri: 'file://notes.txt', mimeType: 'text/plain' } },
      { type: 'resource', resource: { uri: 'file://archive.zip', mimeType: 'application/zip' } },
    ]);
    expect(attachments[0]?.block).toMatchObject({
      type: 'resource',
      resource: { blob: 'cGRm' },
    });
    wrapper.unmount();
  });

  it('falls back to application/octet-stream for untyped generic files', async () => {
    const wrapper = mountBar();

    const attachments = await selectFiles(wrapper, [
      new File(['binary'], 'payload', { type: '' }),
    ]);

    expect(attachments[0]?.block).toEqual({
      type: 'resource',
      resource: {
        uri: 'file://payload',
        mimeType: 'application/octet-stream',
        blob: 'YmluYXJ5',
      },
    });
    wrapper.unmount();
  });

  it('renders chips for selected attachments and removes them', async () => {
    const wrapper = mountBar();
    const attachments = await selectFiles(wrapper, [
      new File(['one'], 'one.txt', { type: 'text/plain' }),
      new File(['two'], 'two.txt', { type: 'text/plain' }),
    ]);

    await wrapper.setProps({ pendingAttachments: attachments });

    expect(wrapper.text()).toContain('one.txt');
    expect(wrapper.text()).toContain('two.txt');
    expect(wrapper.findAll('button').filter((button) => button.text() === '×')).toHaveLength(2);

    await wrapper.findAll('button').find((button) => button.text() === '×')?.trigger('click');

    expect(wrapper.emitted('remove-attachment')?.[0]).toEqual([attachments[0]?.id]);
    wrapper.unmount();
  });
});
