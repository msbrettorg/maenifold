import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import VideoBlock from '../VideoBlock.vue';

const { Plyr, plyrDestroy } = vi.hoisted(() => {
  const destroy = vi.fn();
  const constructor = vi.fn((_element: HTMLMediaElement) => ({ destroy }));
  return { Plyr: constructor, plyrDestroy: destroy };
});

vi.mock('plyr', () => ({ default: Plyr }));

const expectedVideoControls = ['play', 'progress', 'current-time', 'mute', 'volume', 'fullscreen'];

describe('VideoBlock', () => {
  beforeEach(() => {
    Plyr.mockClear();
    plyrDestroy.mockClear();
  });

  it('mounts a Plyr video player for the provided source and MIME type', () => {
    const src = 'data:video/mp4;base64,AAAA';
    const mimeType = 'video/mp4';

    const wrapper = mount(VideoBlock, {
      props: { src, mimeType },
    });

    const video = wrapper.find('video');
    expect(video.exists()).toBe(true);
    expect(video.attributes('src')).toBe(src);
    expect(video.attributes('preload')).toBe('metadata');
    expect(video.attributes('playsinline')).toBeDefined();
    expect(video.attributes('autoplay')).toBeUndefined();
    expect(video.attributes('controls')).toBeUndefined();

    const source = video.find('source');
    expect(source.attributes('src')).toBe(src);
    expect(source.attributes('type')).toBe(mimeType);
    expect(Plyr).toHaveBeenCalledOnce();
    expect(Plyr).toHaveBeenCalledWith(
      video.element,
      expect.objectContaining({ controls: expect.arrayContaining(expectedVideoControls) }),
    );
    wrapper.unmount();
  });

  it('destroys the Plyr video instance on unmount', () => {
    const wrapper = mount(VideoBlock, {
      props: { src: 'data:video/mp4;base64,AAAA', mimeType: 'video/mp4' },
    });

    expect(Plyr).toHaveBeenCalledOnce();
    expect(plyrDestroy).not.toHaveBeenCalled();
    wrapper.unmount();
    expect(plyrDestroy).toHaveBeenCalledOnce();
  });

  it('renders the component-owned Plyr container for visible video controls', () => {
    const wrapper = mount(VideoBlock, {
      props: { src: 'data:video/mp4;base64,AAAA', mimeType: 'video/mp4' },
    });

    const playerContainer = wrapper.find('[data-testid="video-plyr-container"]');
    expect(playerContainer.exists()).toBe(true);
    expect(playerContainer.isVisible()).toBe(true);
    expect(Plyr.mock.calls[0]?.[1]).toEqual(
      expect.objectContaining({ controls: expect.arrayContaining(expectedVideoControls) }),
    );
    wrapper.unmount();
  });

  it('keeps the Electron renderer CSP compatible with inline video data URLs', () => {
    const mainProcess = readFileSync(join(process.cwd(), 'electron/main.ts'), 'utf8');
    expect(mainProcess).toContain("media-src 'self' data:");
  });
});
