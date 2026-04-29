import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import ResourcePreview from '../ResourcePreview.vue';
import MarkdownView from '../MarkdownView.vue';

describe('ResourcePreview', () => {
  it('renders markdown via MarkdownView', () => {
    setActivePinia(createPinia());
    const wrapper = mount(ResourcePreview, {
      props: { dispatched: { kind: 'markdown', text: '# Resource' } },
      attachTo: document.body,
    });

    expect(wrapper.findComponent(MarkdownView).exists()).toBe(true);
    expect(wrapper.findComponent(MarkdownView).props('content')).toBe('# Resource');
    wrapper.unmount();
  });

  it('renders JSON in a collapsible region', () => {
    const wrapper = mount(ResourcePreview, {
      props: { dispatched: { kind: 'json', pretty: '{\n  "ok": true\n}' } },
    });

    expect(wrapper.find('details').exists()).toBe(true);
    expect(wrapper.find('summary').text()).toContain('JSON');
    expect(wrapper.find('pre').text()).toContain('"ok": true');
    wrapper.unmount();
  });

  it('renders an image preview', () => {
    const wrapper = mount(ResourcePreview, {
      props: { dispatched: { kind: 'image', src: 'data:image/png;base64,aW1hZ2U=', alt: 'logo.png' } },
    });

    const image = wrapper.find('img');
    expect(image.attributes('src')).toBe('data:image/png;base64,aW1hZ2U=');
    expect(image.attributes('alt')).toBe('logo.png');
    wrapper.unmount();
  });

  it('renders an audio preview', () => {
    const wrapper = mount(ResourcePreview, {
      props: { dispatched: { kind: 'audio', src: 'data:audio/mpeg;base64,YXVkaW8=' } },
    });

    const audio = wrapper.find('audio');
    expect(audio.exists()).toBe(true);
    expect(audio.attributes('controls')).toBeDefined();
    expect(audio.attributes('src')).toBe('data:audio/mpeg;base64,YXVkaW8=');
    wrapper.unmount();
  });

  it('renders a download chip', () => {
    const wrapper = mount(ResourcePreview, {
      props: { dispatched: { kind: 'download', name: 'archive.bin', size: 1024, uri: 'file:///archive.bin' } },
    });

    expect(wrapper.text()).toContain('archive.bin');
    expect(wrapper.text()).toContain('1 KiB');
    expect(wrapper.text()).toContain('file:///archive.bin');
    wrapper.unmount();
  });
});
