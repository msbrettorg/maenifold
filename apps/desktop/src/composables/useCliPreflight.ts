import { computed, ref, watch, type Ref } from 'vue';

export interface CliPreflightResult {
  checking: Ref<boolean>;
  found: Ref<boolean | null>;
  binaryName: Ref<string>;
  resolvedPath: Ref<string | null>;
}

function extractBinaryName(command: string): string {
  const trimmed = command.trim();
  if (!trimmed) return '';
  const tokens = trimmed.split(/\s+/);
  if (tokens[0] === 'npx' || tokens[0] === 'bunx') {
    return tokens[1] ?? '';
  }
  return tokens[0];
}

const INSTALL_URLS: Record<string, string> = {
  gh: 'https://cli.github.com',
  claude: 'https://docs.anthropic.com/en/docs/claude-cli',
  ollama: 'https://ollama.com/download',
  codex: 'https://github.com/openai/codex',
};

export function getInstallUrl(binary: string): string | null {
  const base = binary.replace(/\.exe$/i, '').toLowerCase();
  return INSTALL_URLS[base] ?? null;
}

export function useCliPreflight(command: Ref<string>): CliPreflightResult {
  const checking = ref(false);
  const found = ref<boolean | null>(null);
  const resolvedPath = ref<string | null>(null);
  const binaryName = computed(() => extractBinaryName(command.value));

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;

  watch(command, (newCmd) => {
    const binary = extractBinaryName(newCmd);
    if (!binary) {
      requestId += 1;
      found.value = null;
      resolvedPath.value = null;
      checking.value = false;
      clearTimeout(debounceTimer);
      return;
    }
    requestId += 1;
    checking.value = true;
    clearTimeout(debounceTimer);
    const thisRequest = requestId;
    debounceTimer = setTimeout(async () => {
      const api = typeof window !== 'undefined' ? window.electronAPI : undefined;
      if (!api?.checkBinaryExists) {
        if (thisRequest === requestId) {
          checking.value = false;
          found.value = null;
        }
        return;
      }
      try {
        const result = await api.checkBinaryExists(binary);
        if (thisRequest === requestId) {
          found.value = result.found;
          resolvedPath.value = result.path;
        }
      } catch {
        if (thisRequest === requestId) {
          found.value = null;
          resolvedPath.value = null;
        }
      } finally {
        if (thisRequest === requestId) {
          checking.value = false;
        }
      }
    }, 500);
  }, { immediate: true });

  return { checking, found, binaryName, resolvedPath };
}
