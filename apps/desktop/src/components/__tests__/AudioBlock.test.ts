import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import AudioBlock from '../AudioBlock.vue';

const { Plyr, plyrDestroy } = vi.hoisted(() => {
  const destroy = vi.fn();
  const constructor = vi.fn((_element: HTMLMediaElement) => ({ destroy }));
  return { Plyr: constructor, plyrDestroy: destroy };
});

vi.mock('plyr', () => ({ default: Plyr }));

const expectedAudioControls = ['play', 'progress', 'current-time', 'mute', 'volume'];

describe('AudioBlock', () => {
  beforeEach(() => {
    Plyr.mockClear();
    plyrDestroy.mockClear();
  });

  it('mounts a Plyr audio player for the provided source and MIME type', () => {
    const src = 'data:audio/mpeg;base64,AAAA';
    const mimeType = 'audio/mpeg';

    const wrapper = mount(AudioBlock, {
      props: { src, mimeType, label: 'Narration sample' },
    });

    const audio = wrapper.find('audio');
    expect(audio.exists()).toBe(true);
    expect(audio.attributes('src')).toBe(src);
    expect(audio.attributes('preload')).toBe('metadata');
    expect(audio.attributes('aria-label')).toBe('Narration sample');
    expect(audio.attributes('controls')).toBeUndefined();

    const source = audio.find('source');
    expect(source.attributes('src')).toBe(src);
    expect(source.attributes('type')).toBe(mimeType);
    expect(Plyr).toHaveBeenCalledOnce();
    expect(Plyr).toHaveBeenCalledWith(
      audio.element,
      expect.objectContaining({ controls: expect.arrayContaining(expectedAudioControls) }),
    );
    wrapper.unmount();
  });

  it('destroys the Plyr audio instance on unmount', () => {
    const wrapper = mount(AudioBlock, {
      props: { src: 'data:audio/mpeg;base64,AAAA', mimeType: 'audio/mpeg' },
    });

    expect(Plyr).toHaveBeenCalledOnce();
    expect(plyrDestroy).not.toHaveBeenCalled();
    wrapper.unmount();
    expect(plyrDestroy).toHaveBeenCalledOnce();
  });

  it('renders the component-owned Plyr container for visible audio controls', () => {
    const wrapper = mount(AudioBlock, {
      props: { src: 'data:audio/mpeg;base64,AAAA', mimeType: 'audio/mpeg' },
    });

    const playerContainer = wrapper.find('[data-testid="audio-plyr-container"]');
    expect(playerContainer.exists()).toBe(true);
    expect(playerContainer.isVisible()).toBe(true);
    expect(Plyr.mock.calls[0]?.[1]).toEqual(
      expect.objectContaining({ controls: expect.arrayContaining(expectedAudioControls) }),
    );
    wrapper.unmount();
  });
});
