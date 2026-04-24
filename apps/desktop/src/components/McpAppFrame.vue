<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import {
  AppBridge,
  PostMessageTransport,
  buildAllowAttribute,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import type { McpUiResourcePermissions } from '@modelcontextprotocol/ext-apps/app-bridge';
import { Skeleton } from './ui/skeleton';
import { useMcpClient } from '@/composables/useMcpClient';
import type { ToolCallInfo } from '@/lib/types';

const props = defineProps<{
  toolCall: ToolCallInfo;
}>();

const iframeRef = ref<HTMLIFrameElement | null>(null);
const bridge = shallowRef<AppBridge | null>(null);
const ready = ref(false);
const errorMessage = ref<string | null>(null);
const sandboxHeight = ref<number>(420);
const { readResource } = useMcpClient();
let disposed = false;

const appUi = computed(() => props.toolCall.appUi ?? null);

const permissions = computed<McpUiResourcePermissions | undefined>(() => {
  const list = appUi.value?.permissions;
  if (!list || list.length === 0) return undefined;
  return Object.fromEntries(list.map((p) => [p, {}])) as McpUiResourcePermissions;
});

const allowAttr = computed(() => buildAllowAttribute(permissions.value));

const sandboxSrc = computed(() => {
  if (!appUi.value) return '';
  const params = new URLSearchParams();
  params.set('toolCallId', props.toolCall.toolCallId);
  if (appUi.value.csp) {
    params.set('csp', encodeURIComponent(appUi.value.csp));
  }
  return `mcp-app-sandbox://app/?${params.toString()}`;
});

async function initBridge() {
  if (disposed || !iframeRef.value || !iframeRef.value.contentWindow) return;
  try {
    const transport = new PostMessageTransport(
      iframeRef.value.contentWindow,
      iframeRef.value.contentWindow,
    );
    const ab = new AppBridge(
      null,
      { name: 'maenifold-desktop', version: '0.1.0' },
      {},
    );

    ab.onsandboxready = async () => {
      if (disposed) return;
      const uri = appUi.value?.resourceUri;
      if (!uri) {
        errorMessage.value = 'No resourceUri in tool _meta.ui';
        return;
      }
      try {
        const resource = await readResource(uri);
        if (disposed) return;
        const html = resource.text ?? '';
        if (!html) {
          errorMessage.value = 'App resource was empty';
          return;
        }
        await ab.sendSandboxResourceReady({
          html,
          sandbox: 'allow-scripts allow-same-origin allow-forms',
          permissions: permissions.value,
        } as any);
      } catch (err) {
        if (disposed) return;
        errorMessage.value = `Failed to fetch app resource`;
      }
    };

    ab.oninitialized = async () => {
      if (disposed) return;
      ready.value = true;
      if (props.toolCall.rawInput) {
        await ab.sendToolInput({ arguments: props.toolCall.rawInput } as any);
      }
      if (disposed) return;
      if (props.toolCall.rawOutput) {
        await ab.sendToolResult({ result: props.toolCall.rawOutput } as any);
      }
    };

    ab.onsizechange = (params: { height?: number; width?: number }) => {
      if (disposed) return;
      if (typeof params?.height === 'number' && params.height > 0) {
        sandboxHeight.value = Math.min(2000, Math.max(80, params.height));
      }
    };

    ab.onopenlink = async (params: { url: string }) => {
      if (disposed) return { isError: true } as any;
      // Only allow http/https URLs
      try {
        const parsed = new URL(params.url);
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
          return { isError: true } as any;
        }
        window.open(params.url, '_blank', 'noopener');
        return {};
      } catch {
        return { isError: true } as any;
      }
    };

    ab.onmessage = async () => {
      return {};
    };

    ab.oncalltool = async (_params) => {
      // Tool call scope restriction: only allow calls to the originating server
      // Full proxying will be wired in a future task when useMcpClient exposes
      // raw CallToolResult shape. For now, return an error indicating tool calls
      // are not yet supported.
      return {
        content: [{ type: 'text', text: 'Tool calls from MCP Apps are not yet supported.' }],
        isError: true,
      } as any;
    };

    await ab.connect(transport);
    if (disposed) return;
    bridge.value = ab;
  } catch (err) {
    if (!disposed) {
      errorMessage.value = (err as Error).message;
    }
  }
}

async function teardown() {
  try {
    await bridge.value?.teardownResource({} as any);
  } catch {
    /* ignore teardown errors */
  } finally {
    try {
      bridge.value?.close?.();
    } catch {
      /* ignore */
    }
    bridge.value = null;
    ready.value = false;
  }
}

onMounted(() => {
  // initBridge runs once iframe load fires
});

onBeforeUnmount(() => {
  disposed = true;
  void teardown();
});

watch(
  () => props.toolCall.toolCallId,
  async () => {
    await teardown();
    if (!disposed) await initBridge();
  },
);
</script>

<template>
  <div class="relative w-full rounded-lg overflow-hidden bg-transparent border border-border" :style="{ height: sandboxHeight + 'px' }">
    <Skeleton v-if="!ready && !errorMessage" class="absolute inset-0" />
    <div v-if="errorMessage" class="p-3 text-sm text-destructive">MCP App error: {{ errorMessage }}</div>
    <iframe
      ref="iframeRef"
      :src="sandboxSrc"
      :allow="allowAttr"
      sandbox="allow-scripts allow-same-origin allow-forms"
      class="w-full h-full border-0 block bg-transparent"
      @load="initBridge"
    />
  </div>
</template>
