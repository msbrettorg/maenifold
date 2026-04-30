import stripAnsi from 'strip-ansi';
import type { ToolCallContentBlock } from '../lib/types';
import type { TerminalState } from '../stores/useTerminalsStore';
import { useTerminalOutput } from './useTerminalOutput';

type TerminalBlock = Extract<ToolCallContentBlock, { type: 'terminal' }>;

export function isTerminalBlock(block: ToolCallContentBlock): block is TerminalBlock {
  if (block.type !== 'terminal') return false;
  const terminalBlock = block as TerminalBlock;
  return terminalId(terminalBlock) != null ||
    (typeof terminalBlock.output === 'string' && terminalBlock.output.length > 0) ||
    terminalStatusLabel(terminalBlock) != null ||
    terminalBlock.truncated === true;
}

export function terminalExitCode(block: TerminalBlock): number | null {
  const directExitCode = (block as { exitCode?: unknown }).exitCode;
  if (typeof directExitCode === 'number') return directExitCode;
  return block.exitStatus?.exitCode ?? null;
}

export function terminalId(block: TerminalBlock): string | null {
  const id = (block as { terminalId?: unknown }).terminalId;
  if (typeof id !== 'string') return null;
  const trimmed = id.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function terminalStatusLabel(block: TerminalBlock): string | null {
  const exitCode = terminalExitCode(block);
  if (exitCode != null) return `exit ${exitCode}`;
  if (block.exitStatus?.signal) return `signal ${block.exitStatus.signal}`;
  if (block.released === true) return 'released';
  return null;
}

export function terminalOutputState(block: TerminalBlock): TerminalState | null {
  const id = terminalId(block);
  return id ? useTerminalOutput(id).value : null;
}

export function terminalRenderedOutput(block: TerminalBlock): string {
  const state = terminalOutputState(block);
  const output = state?.output ?? block.output ?? '';
  return stripAnsi(output);
}

export function terminalRenderedStatusLabel(block: TerminalBlock): string | null {
  const state = terminalOutputState(block);
  const exitCode = state?.exitStatus?.exitCode ?? terminalExitCode(block);
  if (exitCode != null) return state ? `exited ${exitCode}` : `exit ${exitCode}`;
  const signal = state?.exitStatus?.signal ?? block.exitStatus?.signal;
  if (signal) return `signal ${signal}`;
  if ((state?.released ?? block.released) === true) return 'released';
  return terminalStatusLabel(block);
}

export function terminalRenderedBadgeVariant(block: TerminalBlock): 'outline' | 'destructive' {
  const state = terminalOutputState(block);
  const exitCode = state?.exitStatus?.exitCode ?? terminalExitCode(block);
  return exitCode != null && exitCode !== 0 ? 'destructive' : 'outline';
}

export function terminalIsTruncated(block: TerminalBlock): boolean {
  return terminalOutputState(block)?.truncated === true || block.truncated === true;
}

export function useToolCallTerminal() {
  return {
    isTerminalBlock,
    terminalExitCode,
    terminalId,
    terminalStatusLabel,
    terminalOutputState,
    terminalRenderedOutput,
    terminalRenderedStatusLabel,
    terminalRenderedBadgeVariant,
    terminalIsTruncated,
  };
}
