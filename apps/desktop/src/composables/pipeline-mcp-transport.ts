export interface PipelineEnvelope<T> {
  success: boolean;
  data: T;
  error?: unknown;
}

export class ToolEnvelopeError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ToolEnvelopeError';
  }
}

function isPipelineEnvelope(value: unknown): value is PipelineEnvelope<unknown> {
  return (
    typeof value === 'object' &&
    value !== null &&
    'success' in value &&
    typeof (value as { success?: unknown }).success === 'boolean' &&
    'data' in value
  );
}

function getEnvelopeErrorMessage(error: unknown): string {
  if (typeof error === 'string' && error.length > 0) {
    return error;
  }

  if (error instanceof Error && error.message.length > 0) {
    return error.message;
  }

  if (error == null) {
    return 'Pipeline operation failed';
  }

  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function getEnvelopeErrorRecord(error: unknown): { code: string; message: string } | null {
  if (
    typeof error === 'object' &&
    error !== null &&
    typeof (error as { code?: unknown }).code === 'string'
  ) {
    return {
      code: (error as { code: string }).code,
      message: getEnvelopeErrorMessage((error as { message?: unknown }).message ?? error),
    };
  }
  return null;
}

export function unwrapEnvelope<T>(result: unknown): T {
  if (!isPipelineEnvelope(result)) {
    return result as T;
  }

  if (!result.success) {
    const errorRecord = getEnvelopeErrorRecord(result.error);
    if (errorRecord) {
      throw new ToolEnvelopeError(errorRecord.code, errorRecord.message);
    }
    throw new Error(getEnvelopeErrorMessage(result.error));
  }

  if (result.data == null) {
    throw new Error(getEnvelopeErrorMessage(result.error) || 'Pipeline operation returned empty data');
  }

  return result.data as T;
}

function tryParseJson(value: string): unknown {
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

export function unwrapToolData<T>(result: unknown): T {
  let current = unwrapEnvelope<unknown>(result);

  for (let i = 0; i < 3; i += 1) {
    if (typeof current !== 'string') {
      break;
    }

    current = tryParseJson(current);
  }

  return current as T;
}

export function normalizeWorkflowIds(result: unknown): string[] {
  const current = unwrapToolData<string[] | { assets?: string[] } | { id?: string }[] | undefined>(result);

  if (Array.isArray(current)) {
    return current
      .map((entry) => (typeof entry === 'string' ? entry : (entry as { id?: string }).id))
      .filter((entry): entry is string => typeof entry === 'string' && entry.length > 0);
  }

  return (current as { assets?: string[] })?.assets ?? [];
}
