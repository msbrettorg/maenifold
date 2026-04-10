import { ref, type Ref } from 'vue';
import { useMemoryApi } from './useMemoryApi';
import { buildWorkflowTree } from './workflowTreeBuilder';
import type { WorkflowTreeNode } from '../lib/workflow-types';

export function useWorkflowTree(): {
  loadTree: (uri: string) => Promise<WorkflowTreeNode>;
  tree: Ref<WorkflowTreeNode | null>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
} {
  const api = useMemoryApi();
  const tree = ref<WorkflowTreeNode | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadTree(uri: string): Promise<WorkflowTreeNode> {
    loading.value = true;
    error.value = null;

    try {
      const rootNode = await buildWorkflowTree(api, uri);
      tree.value = rootNode;
      return rootNode;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  return { loadTree, tree, loading, error };
}
