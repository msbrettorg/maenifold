export type ResourceContent = {
  uri: string;
  mimeType?: string;
  text?: string;
  blob?: string;
  size?: number;
  name?: string;
};

export type DispatchedResource =
  | { kind: 'markdown'; text: string }
  | { kind: 'json'; pretty: string }
  | { kind: 'code'; text: string; language: string }
  | { kind: 'image'; src: string; alt: string }
  | { kind: 'video'; src: string; mimeType: string }
  | { kind: 'audio'; src: string }
  | { kind: 'download'; name: string; size?: number; uri: string };

const CODE_MIME_LANGUAGES: Record<string, string> = {
  'text/x-rust': 'rust',
  'text/x-typescript': 'typescript',
  'text/x-python': 'python',
  'text/javascript': 'javascript',
  'text/typescript': 'typescript',
};

function nameFromUri(uri: string): string {
  const withoutQuery = uri.split(/[?#]/, 1)[0] ?? uri;
  const lastSegment = withoutQuery.split('/').filter(Boolean).at(-1);
  if (lastSegment) {
    try {
      return decodeURIComponent(lastSegment);
    } catch {
      return lastSegment;
    }
  }
  return uri;
}

function download(content: ResourceContent): DispatchedResource {
  return {
    kind: 'download',
    name: content.name || nameFromUri(content.uri),
    size: content.size,
    uri: content.uri,
  };
}

export function dispatchResource(content: ResourceContent): DispatchedResource {
  const mimeType = content.mimeType?.toLowerCase();

  if (!mimeType) {
    return download(content);
  }

  if (mimeType === 'text/markdown' && content.text != null) {
    return { kind: 'markdown', text: content.text };
  }

  if (mimeType === 'image/svg+xml' && content.text != null) {
    return { kind: 'markdown', text: content.text };
  }

  if (mimeType === 'application/json' && content.text != null) {
    try {
      return { kind: 'json', pretty: JSON.stringify(JSON.parse(content.text), null, 2) };
    } catch {
      return { kind: 'code', language: 'json', text: content.text };
    }
  }

  const language = CODE_MIME_LANGUAGES[mimeType];
  if (language && content.text != null) {
    return { kind: 'code', language, text: content.text };
  }

  if (mimeType.startsWith('text/') && content.text != null) {
    return { kind: 'code', language: 'text', text: content.text };
  }

  if (mimeType.startsWith('image/') && content.blob) {
    return {
      kind: 'image',
      src: `data:${mimeType};base64,${content.blob}`,
      alt: content.name || nameFromUri(content.uri),
    };
  }

  if (mimeType.startsWith('video/') && content.blob) {
    return { kind: 'video', src: `data:${mimeType};base64,${content.blob}`, mimeType };
  }

  if (mimeType.startsWith('audio/') && content.blob) {
    return { kind: 'audio', src: `data:${mimeType};base64,${content.blob}` };
  }

  return download(content);
}
