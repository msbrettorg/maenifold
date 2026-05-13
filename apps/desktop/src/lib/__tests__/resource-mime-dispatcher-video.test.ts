import { describe, expect, it } from 'vitest';
import { dispatchResource } from '../resource-mime-dispatcher';

describe('dispatchResource video support', () => {
  it('dispatches video/mp4 blobs as video data URLs', () => {
    expect(dispatchResource({ uri: 'inline:1', mimeType: 'video/mp4', blob: 'AAAA' })).toEqual({
      kind: 'video',
      src: 'data:video/mp4;base64,AAAA',
      mimeType: 'video/mp4',
    });
  });

  it('dispatches video/webm blobs as video data URLs', () => {
    expect(dispatchResource({ uri: 'inline:2', mimeType: 'video/webm', blob: 'BBBB' })).toEqual({
      kind: 'video',
      src: 'data:video/webm;base64,BBBB',
      mimeType: 'video/webm',
    });
  });

  it('falls back to download for video/mp4 without blob content', () => {
    expect(dispatchResource({ uri: 'inline:3', mimeType: 'video/mp4' })).toEqual({
      kind: 'download',
      name: 'inline:3',
      uri: 'inline:3',
    });
  });

  it('keeps animated GIF blobs on the image path', () => {
    expect(dispatchResource({ uri: 'inline:animated.gif', mimeType: 'image/gif', blob: 'R0lGODlh' })).toEqual({
      kind: 'image',
      src: 'data:image/gif;base64,R0lGODlh',
      alt: 'inline:animated.gif',
    });
  });

  it('keeps APNG blobs on the image path', () => {
    expect(dispatchResource({ uri: 'inline:animated.png', mimeType: 'image/apng', blob: 'iVBORw0KGgo=' })).toEqual({
      kind: 'image',
      src: 'data:image/apng;base64,iVBORw0KGgo=',
      alt: 'inline:animated.png',
    });
  });
});
