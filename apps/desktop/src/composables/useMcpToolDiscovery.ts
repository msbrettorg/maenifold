import { storeToRefs } from 'pinia';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';

export function useMcpToolDiscovery() {
  const store = useMcpToolDiscoveryStore();
  const { serverResults, allTools, toolsByServer, loading, error } = storeToRefs(store);

  return {
    serverResults,
    allTools,
    toolsByServer,
    loading,
    error,
    refreshTools: store.refreshTools,
  };
}
