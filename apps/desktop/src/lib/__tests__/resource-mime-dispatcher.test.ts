import { describe, expect, it } from 'vitest';
import { dispatchResource, MAX_PREVIEW_BYTES, type DispatchedResource } from '../resource-mime-dispatcher';

const EXPECTED_MAX_PREVIEW_BYTES = 1_048_576;

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

function previewCapBytes(): number {
  expect(MAX_PREVIEW_BYTES).toBe(EXPECTED_MAX_PREVIEW_BYTES);
  return MAX_PREVIEW_BYTES;
}

function expectTooLargeResource(
  resource: DispatchedResource,
  expected: { mimeType: string; name: string; size: number; uri: string },
) {
  expect(resource).toEqual({
    kind: 'too-large',
    mimeType: expected.mimeType,
    name: expected.name,
    size: expected.size,
    uri: expected.uri,
  });

  if (resource.kind === 'too-large') {
    const size: number = resource.size;
    const mimeType: string = resource.mimeType;
    const name: string = resource.name;
    const uri: string = resource.uri;

    expect({ mimeType, name, size, uri }).toEqual(expected);
  }
}

describe('dispatchResource size cap', () => {
  it('dispatches markdown over the preview cap as too-large', () => {
    const cap = previewCapBytes();
    const text = 'x'.repeat(cap + 1);

    expectTooLargeResource(
      dispatchResource({ uri: 'memory://big', mimeType: 'text/markdown', text }),
      { mimeType: 'text/markdown', name: 'big', size: text.length, uri: 'memory://big' },
    );
  });

  it('dispatches JSON over the preview cap as too-large without parsing it', () => {
    const cap = previewCapBytes();
    const text = '{"oversized":'.padEnd(cap + 1, 'x');

    expectTooLargeResource(
      dispatchResource({ uri: 'memory://big-json', mimeType: 'application/json', text }),
      { mimeType: 'application/json', name: 'big-json', size: text.length, uri: 'memory://big-json' },
    );
  });

  it('dispatches code over the preview cap as too-large', () => {
    const cap = previewCapBytes();
    const text = 'x'.repeat(cap + 1);

    expectTooLargeResource(
      dispatchResource({ uri: 'file:///main.rs', mimeType: 'text/x-rust', text }),
      { mimeType: 'text/x-rust', name: 'main.rs', size: text.length, uri: 'file:///main.rs' },
    );
  });

  it('dispatches generic text over the preview cap as too-large', () => {
    const cap = previewCapBytes();
    const text = 'x'.repeat(cap + 1);

    expectTooLargeResource(
      dispatchResource({ uri: 'memory://plain', mimeType: 'text/plain', text }),
      { mimeType: 'text/plain', name: 'plain', size: text.length, uri: 'memory://plain' },
    );
  });

  it('dispatches text exactly at the preview cap normally', () => {
    const cap = previewCapBytes();
    const text = 'x'.repeat(cap);
    const resource = dispatchResource({ uri: 'memory://cap', mimeType: 'text/markdown', text });

    expect(resource.kind).toBe('markdown');
    if (resource.kind === 'markdown') {
      expect(resource.text).toHaveLength(cap);
    }
  });

  it('dispatches SVG over the preview cap as too-large', () => {
    const cap = previewCapBytes();
    const text = `<svg>${'x'.repeat(cap + 1)}</svg>`;

    expectTooLargeResource(
      dispatchResource({ uri: 'file:///diagram.svg', mimeType: 'image/svg+xml', text }),
      { mimeType: 'image/svg+xml', name: 'diagram.svg', size: text.length, uri: 'file:///diagram.svg' },
    );
  });

  it('returns the expected too-large discriminated union shape', () => {
    const cap = previewCapBytes();
    const text = 'x'.repeat(cap + 1);
    const resource = dispatchResource({
      uri: 'memory://named',
      mimeType: 'text/plain',
      text,
      name: 'named-resource.txt',
    });

    expectTooLargeResource(
      resource,
      { mimeType: 'text/plain', name: 'named-resource.txt', size: text.length, uri: 'memory://named' },
    );
  });

  it('does not size-cap image, video, or audio blobs', () => {
    const cap = previewCapBytes();
    const blob = 'a'.repeat(cap + 1);
    const image = dispatchResource({ uri: 'file:///large.png', mimeType: 'image/png', blob });
    const video = dispatchResource({ uri: 'file:///large.mp4', mimeType: 'video/mp4', blob });
    const audio = dispatchResource({ uri: 'file:///large.mp3', mimeType: 'audio/mpeg', blob });

    expect(image.kind).toBe('image');
    expect(video.kind).toBe('video');
    expect(audio.kind).toBe('audio');

    if (image.kind === 'image') {
      expect(image.src).toBe(`data:image/png;base64,${blob}`);
      expect(image.alt).toBe('large.png');
    }
    if (video.kind === 'video') {
      expect(video.src).toBe(`data:video/mp4;base64,${blob}`);
      expect(video.mimeType).toBe('video/mp4');
    }
    if (audio.kind === 'audio') {
      expect(audio.src).toBe(`data:audio/mpeg;base64,${blob}`);
    }
  });
});
