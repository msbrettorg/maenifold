import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import VideoBlock from '../VideoBlock.vue';

describe('VideoBlock', () => {
  it('renders a native video player for the provided source', () => {
    const src = 'data:video/mp4;base64,AAAA';
    const mimeType = 'video/mp4';

    const wrapper = mount(VideoBlock, {
      props: { src, mimeType },
    });

    const video = wrapper.find('video');
    expect(video.exists()).toBe(true);
    expect(video.attributes('controls')).toBeDefined();
    expect(video.attributes('preload')).toBe('metadata');
    expect(video.attributes('playsinline')).toBeDefined();
    expect(video.attributes('autoplay')).toBeUndefined();

    const source = video.find('source');
    expect(source.attributes('src')).toBe(src);
    expect(source.attributes('type')).toBe(mimeType);
    wrapper.unmount();
  });
});
