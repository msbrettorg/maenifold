import { useTrafficStore } from '../stores/traffic';
import { useTerminalsStore } from '../stores/useTerminalsStore';
import type { AcpClientSinks } from '../lib/acp-sinks';

export function createAcpStoreSinks(): AcpClientSinks {
  const traffic = useTrafficStore();
  const terminals = useTerminalsStore();

  return {
    traffic: {
      addEntry: (entry) => traffic.addEntry(entry),
    },
    terminals: {
      setTerminal: (terminalId, state) => terminals.setTerminal(terminalId, state),
      markExited: (terminalId, exitStatus) => terminals.markExited(terminalId, exitStatus),
      markReleased: (terminalId) => terminals.markReleased(terminalId),
    },
  };
}
