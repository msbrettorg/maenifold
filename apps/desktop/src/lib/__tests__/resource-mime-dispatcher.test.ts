import { describe, expect, it } from 'vitest';
import { dispatchResource } from '../resource-mime-dispatcher';

describe('dispatchResource', () => {
  it('dispatches text/markdown as markdown', () => {
    expect(dispatchResource({ uri: 'memory://note', mimeType: 'text/markdown', text: '# Title' })).toEqual({
      kind: 'markdown',
      text: '# Title',
    });
  });

  it('pretty-prints valid application/json', () => {
    expect(dispatchResource({ uri: 'memory://data', mimeType: 'application/json', text: '{"b":2}' })).toEqual({
      kind: 'json',
      pretty: '{\n  "b": 2\n}',
    });
  });

  it('falls back to json code for invalid application/json', () => {
    expect(dispatchResource({ uri: 'memory://bad', mimeType: 'application/json', text: '{"b":' })).toEqual({
      kind: 'code',
      language: 'json',
      text: '{"b":',
    });
  });

  it('dispatches text/x-rust as rust code', () => {
    expect(dispatchResource({ uri: 'file:///main.rs', mimeType: 'text/x-rust', text: 'fn main() {}' })).toEqual({
      kind: 'code',
      language: 'rust',
      text: 'fn main() {}',
    });
  });

  it('dispatches image blobs as image data URLs', () => {
    expect(dispatchResource({ uri: 'file:///logo.png', mimeType: 'image/png', blob: 'aW1hZ2U=' })).toEqual({
      kind: 'image',
      src: 'data:image/png;base64,aW1hZ2U=',
      alt: 'logo.png',
    });
  });

  it('dispatches audio blobs as audio data URLs', () => {
    expect(dispatchResource({ uri: 'file:///sound.mp3', mimeType: 'audio/mpeg', blob: 'YXVkaW8=' })).toEqual({
      kind: 'audio',
      src: 'data:audio/mpeg;base64,YXVkaW8=',
    });
  });

  it('dispatches application/octet-stream as a download', () => {
    expect(dispatchResource({
      uri: 'file:///archive.bin',
      mimeType: 'application/octet-stream',
      size: 42,
      name: 'archive.bin',
    })).toEqual({
      kind: 'download',
      name: 'archive.bin',
      size: 42,
      uri: 'file:///archive.bin',
    });
  });

  it('dispatches missing mimeType as a download', () => {
    expect(dispatchResource({ uri: 'memory://unknown' })).toEqual({
      kind: 'download',
      name: 'unknown',
      uri: 'memory://unknown',
    });
  });
});
