import { storeToRefs } from 'pinia';
import { useMcpToolDiscoveryStore } from '../stores/mcp-tool-discovery';

export function useMcpToolDiscovery() {
  const store = useMcpToolDiscoveryStore();
  const { serverResults, allTools, toolsByServer, loading, error, hasLoadedCatalog } = storeToRefs(store);

  return {
    serverResults,
    allTools,
    toolsByServer,
    loading,
    error,
    hasLoadedCatalog,
    refreshTools: store.refreshTools,
  };
}
