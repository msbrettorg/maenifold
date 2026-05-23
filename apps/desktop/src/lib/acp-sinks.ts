export type TrafficEvent = {
  direction: 'in' | 'out';
  type: 'request' | 'response' | 'notification';
  method: string;
  requestId?: number | string;
  payload: unknown;
  error?: boolean;
};

export type TerminalExitStatus = {
  exitCode?: number | null;
  signal?: string | null;
};

export type TerminalSnapshot = {
  terminalId: string;
  output: string;
  truncated: boolean;
  released: boolean;
  exitStatus?: TerminalExitStatus | null;
  outputByteLimit?: number;
};

export interface TrafficEventSink {
  addEntry(entry: TrafficEvent): void;
}

export interface TerminalEventSink {
  setTerminal(terminalId: string, state: TerminalSnapshot): void;
  markExited(terminalId: string, exitStatus: TerminalExitStatus): void;
  markReleased(terminalId: string): void;
}

export type AcpClientSinks = {
  traffic: TrafficEventSink;
  terminals: TerminalEventSink;
};
