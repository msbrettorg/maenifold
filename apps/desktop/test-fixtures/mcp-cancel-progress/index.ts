type McpContentBlock = {
  type: 'text';
  text: string;
};

type McpToolResult = {
  content: McpContentBlock[];
  isError: boolean;
};

type McpToolDescriptor = {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, unknown>;
    required: string[];
  };
};

export type ProgressEvent = {
  method: 'notifications/progress';
  params: {
    completed: number;
    total: number;
  };
};

type LongRunningOptions = {
  signal?: AbortSignal;
  onProgress?: (event: ProgressEvent) => void;
};

const textResult = (text: string, isError: boolean): McpToolResult => ({
  isError,
  content: [{ type: 'text', text }],
});

const wait = (milliseconds: number, signal?: AbortSignal): Promise<void> =>
  new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(signal.reason ?? new DOMException('Aborted', 'AbortError'));
      return;
    }

    let timer: ReturnType<typeof setTimeout> | undefined;
    const onAbort = () => {
      if (timer !== undefined) {
        clearTimeout(timer);
      }
      reject(signal?.reason ?? new DOMException('Aborted', 'AbortError'));
    };

    timer = setTimeout(() => {
      signal?.removeEventListener('abort', onAbort);
      resolve();
    }, milliseconds);

    signal?.addEventListener('abort', onAbort, { once: true });
  });

export const listTools = (): McpToolDescriptor[] => [
  {
    name: 'long_running',
    description: 'Runs for the requested number of seconds and emits MCP progress notifications.',
    inputSchema: {
      type: 'object',
      properties: {
        seconds: {
          type: 'number',
          description: 'Number of seconds to run.',
        },
      },
      required: ['seconds'],
    },
  },
  {
    name: 'failing',
    description: 'Returns an MCP error result containing the supplied message.',
    inputSchema: {
      type: 'object',
      properties: {
        message: {
          type: 'string',
          description: 'Message to include in the error result.',
        },
      },
      required: ['message'],
    },
  },
];

export const longRunning = async (
  seconds: number,
  options: LongRunningOptions = {},
): Promise<McpToolResult> => {
  const total = seconds * 1_000;
  const interval = 500;

  for (let completed = interval; completed <= total; completed += interval) {
    await wait(interval, options.signal);
    options.onProgress?.({
      method: 'notifications/progress',
      params: {
        completed,
        total,
      },
    });
  }

  return textResult(`completed after ${seconds} seconds`, false);
};

export const failing = async (message: string): Promise<McpToolResult> => textResult(message, true);
