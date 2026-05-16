import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
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
    expect(video.attributes('src')).toBe(src);
    expect(video.attributes('controls')).toBeDefined();
    expect(video.attributes('preload')).toBe('metadata');
    expect(video.attributes('playsinline')).toBeDefined();
    expect(video.attributes('autoplay')).toBeUndefined();

    const source = video.find('source');
    expect(source.attributes('src')).toBe(src);
    expect(source.attributes('type')).toBe(mimeType);
    wrapper.unmount();
  });

  it('keeps the Electron renderer CSP compatible with inline video data URLs', () => {
    const mainProcess = readFileSync(join(process.cwd(), 'electron/main.ts'), 'utf8');
    expect(mainProcess).toContain("media-src 'self' data:");
  });
});
