import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  type CancelNotification,
  type Client,
  type CreateTerminalRequest,
  type CreateTerminalResponse,
  type KillTerminalRequest,
  type KillTerminalResponse,
  type ReadTextFileRequest,
  type ReadTextFileResponse,
  type ReleaseTerminalRequest,
  type ReleaseTerminalResponse,
  type RequestPermissionRequest,
  type RequestPermissionResponse,
  type SessionNotification,
  type TerminalOutputRequest,
  type TerminalOutputResponse,
  type WaitForTerminalExitRequest,
  type WaitForTerminalExitResponse,
  type WriteTextFileRequest,
  type WriteTextFileResponse,
} from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import type { AcpClientSinks } from '../acp-sinks';
import { AcpClientBridgeCore } from '../acp-client-core';
import { PermissionHandler } from '../acp-permissions';

const LIB_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const PUBLIC_METHODS = [
  'connect',
  'disconnect',
  'isTransportClosed',
  'getTransportCloseReason',
  'setPermissionModeProvider',
  'cancel',
  'requestPermission',
  'resolvePermission',
  'cancelPermission',
  'sessionUpdate',
  'writeTextFile',
  'readTextFile',
  'createTerminal',
  'terminalOutput',
  'waitForTerminalExit',
  'killTerminal',
  'releaseTerminal',
  'terminalCreate',
  'terminalWaitForExit',
  'supportsAgentMethod',
] as const satisfies readonly (keyof AcpClientBridgeCore)[];

type PublicContract = {
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  isTransportClosed: () => boolean;
  getTransportCloseReason: () => string | null;
  setPermissionModeProvider: (fn: () => string | null) => void;
  cancel: (params: CancelNotification) => Promise<void>;
  requestPermission: (params: RequestPermissionRequest) => Promise<RequestPermissionResponse>;
  resolvePermission: (
    requestId: string,
    optionId: string,
    scope?: 'session' | 'agent' | 'forever'
  ) => Promise<void>;
  cancelPermission: () => void;
  sessionUpdate: (params: SessionNotification) => Promise<void>;
  writeTextFile: (params: WriteTextFileRequest) => Promise<WriteTextFileResponse>;
  readTextFile: (params: ReadTextFileRequest) => Promise<ReadTextFileResponse>;
  createTerminal: (params: CreateTerminalRequest) => Promise<CreateTerminalResponse>;
  terminalOutput: (params: TerminalOutputRequest) => Promise<TerminalOutputResponse>;
  waitForTerminalExit: (params: WaitForTerminalExitRequest) => Promise<WaitForTerminalExitResponse>;
  killTerminal: (params: KillTerminalRequest) => Promise<KillTerminalResponse>;
  releaseTerminal: (params: ReleaseTerminalRequest) => Promise<ReleaseTerminalResponse>;
  terminalCreate: (params: CreateTerminalRequest) => Promise<CreateTerminalResponse>;
  terminalWaitForExit: (
    params: WaitForTerminalExitRequest
  ) => Promise<WaitForTerminalExitResponse>;
  supportsAgentMethod: (method: string) => boolean;
};

const _publicContract: PublicContract = AcpClientBridgeCore.prototype;

const sinks: AcpClientSinks = {
  traffic: { addEntry: () => {} },
  terminals: {
    setTerminal: () => {},
    markExited: () => {},
    markReleased: () => {},
  },
};

function createInstance(permissionHandler = new PermissionHandler()): AcpClientBridgeCore {
  const instance = new AcpClientBridgeCore('agent-1', 'agent-1', permissionHandler, sinks);
  const _typeCheck: Client = instance;
  expect(_typeCheck).toBe(instance);
  return instance;
}

function listAcpClientFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        return entry.name === '__tests__' ? [] : listAcpClientFiles(fullPath);
      }
      return entry.isFile() && /^acp-client-.*\.ts$/.test(entry.name) ? [fullPath] : [];
    })
    .sort();
}

describe('AcpClientBridgeCore contract', () => {
  it('locks the exported public method surface', () => {
    expect(_publicContract).toBe(AcpClientBridgeCore.prototype);
    const prototype = Object.getPrototypeOf(createInstance()) as Record<PropertyKey, unknown>;
    const prototypeKeys = new Set(Reflect.ownKeys(prototype));

    expect(PUBLIC_METHODS).toHaveLength(20);
    for (const methodName of PUBLIC_METHODS) {
      expect(prototypeKeys.has(methodName), `${methodName} missing from prototype`).toBe(true);
      expect(typeof prototype[methodName]).toBe('function');
    }
  });

  it('exposes the expected public instance properties', () => {
    const permissionHandler = new PermissionHandler();
    const instance = createInstance(permissionHandler);

    expect(Reflect.has(instance, 'pendingPermissionRequests')).toBe(true);
    expect(instance.pendingPermissionRequests).toBe(permissionHandler.pendingPermissionRequests);
    expect(Reflect.has(instance, 'onSessionUpdate')).toBe(true);
    expect(instance.onSessionUpdate).toBeNull();
    expect(Reflect.has(instance, 'onStderr')).toBe(true);
    expect(instance.onStderr).toBeNull();
  });

  it('enforces the per-file LOC budget', () => {
    const counts = listAcpClientFiles(LIB_DIR).map((file) => ({
      file: path.relative(LIB_DIR, file),
      lines: readFileSync(file, 'utf8').split('\n').length,
    }));

    expect(counts.length).toBeGreaterThan(0);
    expect(counts.some(({ file }) => file === 'acp-client-core.ts')).toBe(true);

    expect(counts.filter(({ lines }) => lines > 300)).toEqual([]);
  });
});
