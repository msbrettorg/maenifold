<script lang="ts">
import {
  computed,
  defineComponent,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
  type PropType,
} from 'vue';
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Brain,
  Check,
  CheckCircle2,
  Download,
  RefreshCcw,
  Server,
  Sparkles,
} from '@lucide/vue';
import { mcpHttpClient, McpHttpError } from '../lib/mcp/client';

type StepId = 'welcome' | 'daemon' | 'model' | 'agent' | 'done';
type HealthState = 'idle' | 'loading' | 'ok' | 'error';
type ModelActionState = 'idle' | 'downloading' | 'activating';

const STEP_LABELS: Array<{ id: StepId; label: string }> = [
  { id: 'welcome', label: 'Welcome' },
  { id: 'daemon', label: 'Daemon health' },
  { id: 'model', label: 'Embedding model' },
  { id: 'agent', label: 'Default agent' },
  { id: 'done', label: 'Done' },
];

const ONBOARDING_DISMISSED_KEY = 'maenifold-ui:onboarding-dismissed';
const ONBOARDING_VISIBILITY_EVENT = 'maenifold-ui:onboarding-visibility';
const HEALTH_ENDPOINT = 'http://127.0.0.1:8080/api/health';
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

export interface OnboardingAgent {
  name: string;
  command?: string;
  args?: string[];
  env?: Record<string, string>;
}

export interface OnboardingConfig {
  agents?: OnboardingAgent[] | Record<string, unknown>;
  settings?: {
    defaultAgent?: string | null;
    [key: string]: unknown;
  } | null;
}

export interface OnboardingEmbedStatus {
  activeModelStatus?: {
    state?: string | null;
    message?: string | null;
    modelId?: string | null;
    installedLayout?: string | null;
    [key: string]: unknown;
  } | null;
  [key: string]: unknown;
}

interface McpToolTextContent {
  type: string;
  text?: string;
}

interface McpToolCallResult {
  content?: McpToolTextContent[];
  isError?: boolean;
  [key: string]: unknown;
}

interface HealthResponse {
  status?: string;
  [key: string]: unknown;
}

interface ModelProgress {
  current: number;
  total: number;
  filename?: string;
}

interface EmbeddingModelInfo {
  id: string;
  name: string;
  dimensions: number;
  maxTokens: number;
  sizeBytes: number;
  license: string;
  huggingfaceRepo?: string | null;
  onnxRepo?: string | null;
  released?: string | null;
  mrlDimensions?: number[] | null;
  mtebRetrieval?: number | null;
  installed: boolean;
  installedLayout?: string | null;
  isActive: boolean;
  downloadProgress?: ModelProgress | null;
}

interface EmbeddingCatalogResponse extends OnboardingEmbedStatus {
  models: EmbeddingModelInfo[];
  currentDimensions?: number;
  activeModelId?: string | null;
}

interface EmbeddingStatusResponse extends OnboardingEmbedStatus {
  loaded?: boolean;
  activeModelId?: string | null;
  vec0?: {
    message?: string | null;
    tables?: {
      vecConcepts?: { count?: number | null } | null;
      vecMemoryFiles?: { count?: number | null } | null;
    } | null;
  } | null;
  model?: {
    filename?: string;
    dimensions?: number;
    maxTokens?: number;
  } | null;
}

interface EmbeddingActionResponse {
  message?: string;
  downloaded?: boolean;
  activated?: boolean;
  modelChanged?: boolean;
  reembedRequired?: boolean;
  vectorsCleared?: boolean;
  dimensionsDiffer?: boolean;
  modelId?: string;
}

interface DaemonConfigResponse extends OnboardingConfig {
  agents: OnboardingAgent[];
  settings?: {
    defaultAgent?: string | null;
    [key: string]: unknown;
  } | null;
}

interface VisibilityEventDetail {
  open?: boolean;
}

function readDismissedState(): boolean {
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return false;
  }
  try {
    return window.localStorage.getItem(ONBOARDING_DISMISSED_KEY) === 'true';
  } catch (error) {
    console.warn('Unable to read onboarding dismissal state.', error);
    return false;
  }
}

function writeDismissedState(value: boolean): void {
  if (typeof window === 'undefined' || typeof window.localStorage === 'undefined') {
    return;
  }
  try {
    if (value) {
      window.localStorage.setItem(ONBOARDING_DISMISSED_KEY, 'true');
    } else {
      window.localStorage.removeItem(ONBOARDING_DISMISSED_KEY);
    }
  } catch (error) {
    console.warn('Unable to persist onboarding dismissal state.', error);
  }
}

function broadcastVisibilityChange(open: boolean): void {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(
    new CustomEvent<VisibilityEventDetail>(ONBOARDING_VISIBILITY_EVENT, {
      detail: { open },
    })
  );
}

export function dismiss(): void {
  writeDismissedState(true);
  broadcastVisibilityChange(false);
}

export function reopen(): void {
  writeDismissedState(false);
  broadcastVisibilityChange(true);
}

function countAgents(config: OnboardingConfig | DaemonConfigResponse | null | undefined): number {
  if (!config?.agents) return 0;
  if (Array.isArray(config.agents)) return config.agents.length;
  return Object.keys(config.agents).length;
}

function readDefaultAgentName(
  config: OnboardingConfig | DaemonConfigResponse | null | undefined
): string | null {
  const value = config?.settings?.defaultAgent;
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function needsOnboarding(
  config: OnboardingConfig | null | undefined,
  embedStatus: OnboardingEmbedStatus | null | undefined
): boolean {
  return (
    countAgents(config) === 0 ||
    !readDefaultAgentName(config) ||
    embedStatus?.activeModelStatus?.state !== 'available'
  );
}

function normalizeConfigSnapshot(
  config: OnboardingConfig | null | undefined
): DaemonConfigResponse | null {
  if (!config) return null;

  if (Array.isArray(config.agents)) {
    return {
      agents: config.agents.map((agent) => ({
        name: agent.name,
        command: agent.command,
        args: agent.args ? [...agent.args] : undefined,
        env: agent.env ? { ...agent.env } : undefined,
      })),
      settings: config.settings ? { ...config.settings } : null,
    };
  }

  const agents: OnboardingAgent[] = config.agents
    ? Object.entries(config.agents).map(([name, value]) => {
        const entry = value && typeof value === 'object' ? (value as Record<string, unknown>) : {};
        const args = Array.isArray(entry.args)
          ? entry.args.filter((item): item is string => typeof item === 'string')
          : undefined;
        const env: Record<string, string> | undefined =
          entry.env && typeof entry.env === 'object'
            ? Object.fromEntries(
                Object.entries(entry.env as Record<string, unknown>).filter(
                  ([, envValue]) => typeof envValue === 'string'
                )
              ) as Record<string, string>
            : undefined;
        return {
          name,
          command: typeof entry.command === 'string' ? entry.command : undefined,
          args,
          env,
        };
      })
    : [];

  return {
    agents,
    settings: config.settings ? { ...config.settings } : null,
  };
}

function normalizeEmbedStatus(
  status: OnboardingEmbedStatus | null | undefined
): EmbeddingStatusResponse | null {
  if (!status) return null;
  return {
    ...status,
    activeModelStatus: status.activeModelStatus ? { ...status.activeModelStatus } : null,
  } as EmbeddingStatusResponse;
}

async function gatewayExecute<T>(
  domain: string,
  command: string,
  parameters: Record<string, unknown>
): Promise<T> {
  const result = await mcpHttpClient.callTool<McpToolCallResult>('GatewayExecute', {
    domain,
    command,
    parameters,
  });

  const text = result.content?.find(
    (item) => item.type === 'text' && typeof item.text === 'string'
  )?.text;

  if (result.isError) {
    throw new McpHttpError(text ?? `GatewayExecute ${command} returned an MCP tool error.`, {
      data: result,
    });
  }

  if (typeof text !== 'string') {
    return result as T;
  }

  try {
    return JSON.parse(text) as T;
  } catch {
    return text as unknown as T;
  }
}

function formatModelSize(bytes: number): string {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${Math.round(bytes / 1_000_000)} MB`;
  if (bytes >= 1_000) return `${Math.round(bytes / 1_000)} KB`;
  return `${bytes} B`;
}

function formatMtebScore(score: number | null | undefined): string | null {
  if (typeof score !== 'number' || Number.isNaN(score)) return null;
  return score.toFixed(1);
}

function formatModelMetadata(model: EmbeddingModelInfo): string {
  const metadata = [
    `${model.dimensions}d`,
    `${model.maxTokens} max tokens`,
    formatModelSize(model.sizeBytes),
    model.license,
  ];

  const mteb = formatMtebScore(model.mtebRetrieval);
  if (mteb) {
    metadata.push(`MTEB ${mteb}`);
  }

  return metadata.join(' · ');
}

function formatAgentCommand(agent: OnboardingAgent): string {
  const command = agent.command?.trim();
  const args = agent.args?.join(' ').trim();
  if (command && args) return `${command} ${args}`;
  if (command) return command;
  return 'Configured in daemon settings';
}

export default defineComponent({
  name: 'OnboardingWizard',
  components: {
    AlertTriangle,
    ArrowLeft,
    ArrowRight,
    Bot,
    Brain,
    Check,
    CheckCircle2,
    Download,
    RefreshCcw,
    Server,
    Sparkles,
  },
  props: {
    open: {
      type: Boolean,
      default: true,
    },
    config: {
      type: Object as PropType<OnboardingConfig | null>,
      default: null,
    },
    embeddingStatus: {
      type: Object as PropType<OnboardingEmbedStatus | null>,
      default: null,
    },
  },
  emits: {
    'update:open': (_open: boolean) => true,
    dismissed: () => true,
    completed: () => true,
  },
  setup(props, { emit, expose }) {
    const isVisible = ref(props.open && !readDismissedState());
    const dialogRef = ref<HTMLElement | null>(null);
    const currentStepIndex = ref(0);
    const highestVisitedStepIndex = ref(0);

    const healthState = ref<HealthState>('idle');
    const healthPayload = ref<HealthResponse | null>(null);
    const healthError = ref<string | null>(null);

    const modelLoading = ref(false);
    const modelLoaded = ref(false);
    const modelError = ref<string | null>(null);
    const modelActionState = ref<ModelActionState>('idle');
    const modelActionError = ref<string | null>(null);
    const modelActionMessage = ref<string | null>(null);
    const modelCatalog = ref<EmbeddingCatalogResponse | null>(null);
    const modelStatus = ref<EmbeddingStatusResponse | null>(
      normalizeEmbedStatus(props.embeddingStatus)
    );
    const selectedModelId = ref<string | null>(null);

    const agentsLoading = ref(false);
    const agentsLoaded = ref(false);
    const agentsError = ref<string | null>(null);
    const configSnapshot = ref<DaemonConfigResponse | null>(
      normalizeConfigSnapshot(props.config)
    );
    const configuredDefaultAgentName = ref<string | null>(
      readDefaultAgentName(configSnapshot.value)
    );
    const selectedAgentName = ref<string | null>(
      configuredDefaultAgentName.value ?? configSnapshot.value?.agents[0]?.name ?? null
    );
    const defaultAgentSaving = ref(false);
    const defaultAgentError = ref<string | null>(null);
    const defaultAgentMessage = ref<string | null>(null);

    let modelPollHandle: number | null = null;
    let returnFocusTarget: HTMLElement | null = null;

    const steps = STEP_LABELS;

    const effectiveConfig = computed<DaemonConfigResponse | null>(() => {
      return configSnapshot.value ?? normalizeConfigSnapshot(props.config);
    });

    const effectiveModelStatus = computed<EmbeddingStatusResponse | null>(() => {
      return modelStatus.value ?? normalizeEmbedStatus(props.embeddingStatus);
    });

    const currentStepId = computed<StepId>(() => steps[currentStepIndex.value]?.id ?? 'welcome');

    const stepNumberText = computed(
      () => `Step ${currentStepIndex.value + 1} of ${steps.length}`
    );

    const selectedModel = computed<EmbeddingModelInfo | null>(() => {
      return (
        modelCatalog.value?.models.find((model) => model.id === selectedModelId.value) ?? null
      );
    });

    const currentEmbeddingDimensions = computed(() => {
      return (
        modelCatalog.value?.currentDimensions ??
        effectiveModelStatus.value?.model?.dimensions ??
        0
      );
    });

    const downloadPercent = computed(() => {
      const total = selectedModel.value?.downloadProgress?.total ?? 0;
      const current = selectedModel.value?.downloadProgress?.current ?? 0;
      if (total <= 0) return 0;
      return Math.max(0, Math.min(100, (current / total) * 100));
    });

    const hasDownloadProgress = computed(
      () => (selectedModel.value?.downloadProgress?.total ?? 0) > 0
    );

    const needsDimensionWarning = computed(() => {
      if (!selectedModel.value) return false;
      if (!currentEmbeddingDimensions.value) return false;
      return selectedModel.value.dimensions !== currentEmbeddingDimensions.value;
    });

    const stepVisualIcon = computed(() => {
      switch (currentStepId.value) {
        case 'welcome':
          return Sparkles;
        case 'daemon':
          return Server;
        case 'model':
          return Brain;
        case 'agent':
          return Bot;
        case 'done':
          return CheckCircle2;
        default:
          return Sparkles;
      }
    });

    const canAdvance = computed(() => {
      switch (currentStepId.value) {
        case 'welcome':
          return true;
        case 'daemon':
          return healthState.value === 'ok';
        case 'model':
          return selectedModel.value?.isActive === true;
        case 'agent':
          return (
            !!selectedAgentName.value &&
            selectedAgentName.value === configuredDefaultAgentName.value
          );
        case 'done':
          return true;
        default:
          return false;
      }
    });

    const primaryLabel = computed(() => {
      if (currentStepId.value === 'welcome') return 'Get started';
      if (currentStepId.value === 'done') return 'Enter app';
      return 'Next';
    });

    const primaryDisabled = computed(() => {
      if (currentStepId.value === 'welcome' || currentStepId.value === 'done') {
        return false;
      }
      return !canAdvance.value;
    });

    const footerHint = computed(() => {
      switch (currentStepId.value) {
        case 'welcome':
          return 'Five quick steps to get the app ready.';
        case 'daemon':
          if (healthState.value === 'loading') return 'Checking the local daemon…';
          if (healthState.value === 'ok') return 'The daemon responded successfully.';
          return 'The wizard needs a healthy daemon before continuing.';
        case 'model':
          if (modelLoading.value) return 'Loading embedding catalog…';
          if (modelActionState.value === 'downloading' && selectedModel.value) {
            return `Downloading ${selectedModel.value.name}…`;
          }
          if (modelActionState.value === 'activating' && selectedModel.value) {
            return `Activating ${selectedModel.value.name}…`;
          }
          if (selectedModel.value?.isActive) {
            return `${selectedModel.value.name} is active.`;
          }
          if (selectedModel.value?.installed) {
            return `${selectedModel.value.name} is installed — activate it to continue.`;
          }
          return 'Choose a model, then download and activate it.';
        case 'agent':
          if (defaultAgentSaving.value && selectedAgentName.value) {
            return `Saving ${selectedAgentName.value} as the default agent…`;
          }
          if (
            selectedAgentName.value &&
            selectedAgentName.value === configuredDefaultAgentName.value
          ) {
            return `${selectedAgentName.value} is selected as the default agent.`;
          }
          return 'Choose an agent and save it locally before continuing.';
        case 'done':
          return 'Review the recap, then enter the app.';
        default:
          return '';
      }
    });

    const downloadDisabled = computed(() => {
      return (
        currentStepId.value !== 'model' ||
        modelLoading.value ||
        modelActionState.value !== 'idle' ||
        selectedModel.value === null ||
        selectedModel.value.installed
      );
    });

    const activateDisabled = computed(() => {
      return (
        currentStepId.value !== 'model' ||
        modelLoading.value ||
        modelActionState.value !== 'idle' ||
        selectedModel.value === null ||
        !selectedModel.value.installed ||
        selectedModel.value.isActive
      );
    });

    const setDefaultDisabled = computed(() => {
      return (
        currentStepId.value !== 'agent' ||
        agentsLoading.value ||
        defaultAgentSaving.value ||
        !selectedAgentName.value ||
        selectedAgentName.value === configuredDefaultAgentName.value
      );
    });

    const reembedConceptCount = computed(() => {
      return effectiveModelStatus.value?.vec0?.tables?.vecConcepts?.count ?? null;
    });

    const reembedMemoryFileCount = computed(() => {
      return effectiveModelStatus.value?.vec0?.tables?.vecMemoryFiles?.count ?? null;
    });

    const triggerAgent = computed(() => {
      if (!effectiveConfig.value) {
        return {
          variant: 'pending',
          text: 'agents.length pending',
        };
      }

      const count = effectiveConfig.value.agents.length;
      return {
        variant: count > 0 ? 'ok' : 'bad',
        text: `agents.length = ${count}`,
      };
    });

    const triggerDefaultAgent = computed(() => {
      if (!effectiveConfig.value) {
        return {
          variant: 'pending',
          text: 'defaultAgent pending',
        };
      }

      const defaultAgent = readDefaultAgentName(effectiveConfig.value);
      return {
        variant: defaultAgent ? 'ok' : 'bad',
        text: defaultAgent ? `defaultAgent = "${defaultAgent}"` : 'defaultAgent missing',
      };
    });

    const triggerModel = computed(() => {
      if (!effectiveModelStatus.value) {
        return {
          variant: 'pending',
          text: 'activeModel.state pending',
        };
      }

      const state = effectiveModelStatus.value.activeModelStatus?.state;
      return {
        variant: state === 'available' ? 'ok' : 'bad',
        text: `activeModel.state = ${state ?? 'unknown'}`,
      };
    });

    const recapModelName = computed(() => {
      return selectedModel.value?.name ?? effectiveModelStatus.value?.activeModelStatus?.modelId ?? 'Not selected';
    });

    const recapDaemonStatus = computed(() => {
      return healthState.value === 'ok'
        ? 'OK'
        : healthState.value === 'error'
          ? 'Needs attention'
          : 'Not checked in this run';
    });

    function stopModelPolling(): void {
      if (modelPollHandle !== null) {
        window.clearInterval(modelPollHandle);
        modelPollHandle = null;
      }
    }

    function ensureModelPolling(): void {
      if (typeof window === 'undefined' || modelPollHandle !== null) return;
      modelPollHandle = window.setInterval(() => {
        void syncModelState();
      }, 1500);
    }

    function resetWizardProgress(): void {
      currentStepIndex.value = 0;
      highestVisitedStepIndex.value = 0;

      healthState.value = 'idle';
      healthPayload.value = null;
      healthError.value = null;

      stopModelPolling();
      modelLoading.value = false;
      modelLoaded.value = false;
      modelError.value = null;
      modelActionState.value = 'idle';
      modelActionError.value = null;
      modelActionMessage.value = null;
      modelCatalog.value = null;
      modelStatus.value = normalizeEmbedStatus(props.embeddingStatus);
      selectedModelId.value = null;

      agentsLoading.value = false;
      agentsLoaded.value = false;
      agentsError.value = null;
      configSnapshot.value = normalizeConfigSnapshot(props.config);
      configuredDefaultAgentName.value = readDefaultAgentName(configSnapshot.value);
      selectedAgentName.value =
        configuredDefaultAgentName.value ?? configSnapshot.value?.agents[0]?.name ?? null;
      defaultAgentSaving.value = false;
      defaultAgentError.value = null;
      defaultAgentMessage.value = null;
    }

    function getFocusableElements(): HTMLElement[] {
      const dialog = dialogRef.value;
      if (!dialog || typeof window === 'undefined') return [];

      return Array.from(dialog.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
        (element) => {
          if (element.tabIndex < 0 || element.getAttribute('aria-disabled') === 'true') {
            return false;
          }

          const style = window.getComputedStyle(element);
          return (
            style.display !== 'none' &&
            style.visibility !== 'hidden' &&
            element.getClientRects().length > 0
          );
        }
      );
    }

    function focusFirstDialogElement(): void {
      const target = getFocusableElements()[0] ?? dialogRef.value;
      target?.focus({ preventScroll: true });
    }

    function restoreReturnFocus(): void {
      const target = returnFocusTarget;
      returnFocusTarget = null;

      if (target && typeof document !== 'undefined' && document.contains(target)) {
        target.focus({ preventScroll: true });
      }
    }

    function trapDialogFocus(event: KeyboardEvent): void {
      const dialog = dialogRef.value;
      if (!dialog || typeof document === 'undefined') return;

      const focusableElements = getFocusableElements();
      if (focusableElements.length === 0) {
        event.preventDefault();
        dialog.focus({ preventScroll: true });
        return;
      }

      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];
      const activeElement = document.activeElement;

      if (focusableElements.length === 1) {
        event.preventDefault();
        firstElement.focus({ preventScroll: true });
        return;
      }

      if (event.shiftKey) {
        if (activeElement === firstElement || activeElement === dialog || !dialog.contains(activeElement)) {
          event.preventDefault();
          lastElement.focus({ preventScroll: true });
        }
        return;
      }

      if (activeElement === lastElement || !dialog.contains(activeElement)) {
        event.preventDefault();
        firstElement.focus({ preventScroll: true });
      }
    }

    function handleVisibilityEvent(event: Event): void {
      const detail = (event as CustomEvent<VisibilityEventDetail>).detail;
      if (detail?.open) {
        resetWizardProgress();
        isVisible.value = true;
        emit('update:open', true);
      } else {
        isVisible.value = false;
        emit('update:open', false);
      }
    }

    async function checkDaemonHealth(): Promise<void> {
      healthState.value = 'loading';
      healthError.value = null;

      try {
        const response = await fetch(HEALTH_ENDPOINT, {
          headers: {
            Accept: 'application/json',
          },
        });
        const raw = await response.text();
        const payload = (raw ? JSON.parse(raw) : {}) as HealthResponse;

        if (!response.ok) {
          throw new Error(`Daemon returned HTTP ${response.status}.`);
        }

        healthPayload.value = payload;
        if (payload.status === 'ok') {
          healthState.value = 'ok';
          return;
        }

        healthState.value = 'error';
        healthError.value =
          typeof payload.status === 'string'
            ? `Unexpected daemon status: ${payload.status}`
            : 'Unexpected daemon health response.';
      } catch (error) {
        healthState.value = 'error';
        healthPayload.value = null;
        healthError.value = error instanceof Error ? error.message : String(error);
      }
    }

    async function refreshEmbeddingData(): Promise<void> {
      const [catalog, status] = await Promise.all([
        gatewayExecute<EmbeddingCatalogResponse>('maenifold.system', 'EmbeddingModels', {
          action: 'list',
        }),
        gatewayExecute<EmbeddingStatusResponse>('maenifold.system', 'EmbeddingModels', {
          action: 'status',
        }),
      ]);

      modelCatalog.value = catalog;
      modelStatus.value = status;

      const preferredSelection =
        catalog.models.find((model) => model.id === selectedModelId.value) ??
        catalog.models.find((model) => model.isActive) ??
        catalog.models.find((model) => model.installed) ??
        catalog.models[0] ??
        null;

      selectedModelId.value = preferredSelection?.id ?? null;
    }

    async function loadEmbeddingModels(): Promise<void> {
      modelLoading.value = true;
      modelError.value = null;
      modelActionError.value = null;

      try {
        await refreshEmbeddingData();
        modelLoaded.value = true;

        const activeDownload =
          selectedModel.value?.downloadProgress &&
          !selectedModel.value.installed &&
          !selectedModel.value.isActive;

        if (activeDownload) {
          modelActionState.value = 'downloading';
          ensureModelPolling();
        }
      } catch (error) {
        modelError.value = error instanceof Error ? error.message : String(error);
      } finally {
        modelLoading.value = false;
      }
    }

    async function syncModelState(): Promise<void> {
      try {
        await refreshEmbeddingData();
      } catch (error) {
        stopModelPolling();
        modelActionState.value = 'idle';
        modelActionError.value = error instanceof Error ? error.message : String(error);
        return;
      }

      const latestSelection = selectedModel.value;
      if (!latestSelection) return;

      if (latestSelection.installed || latestSelection.isActive) {
        stopModelPolling();
        modelActionState.value = 'idle';
        modelActionMessage.value = latestSelection.isActive
          ? `${latestSelection.name} is active and ready to use.`
          : `${latestSelection.name} finished downloading. Activate it to continue.`;
      }
    }

    async function downloadSelectedModel(): Promise<void> {
      const model = selectedModel.value;
      if (!model || downloadDisabled.value) return;

      modelActionState.value = 'downloading';
      modelActionError.value = null;
      modelActionMessage.value = null;

      try {
        const response = await gatewayExecute<EmbeddingActionResponse>(
          'maenifold.system',
          'EmbeddingModels',
          {
            action: 'download',
            modelId: model.id,
          }
        );

        modelActionMessage.value = response.message ?? `Downloading ${model.name}…`;
        await syncModelState();

        if (
          selectedModel.value &&
          !selectedModel.value.installed &&
          !selectedModel.value.isActive
        ) {
          ensureModelPolling();
        }
      } catch (error) {
        stopModelPolling();
        modelActionState.value = 'idle';
        modelActionError.value = error instanceof Error ? error.message : String(error);
      }
    }

    async function activateSelectedModel(): Promise<void> {
      const model = selectedModel.value;
      if (!model || activateDisabled.value) return;

      modelActionState.value = 'activating';
      modelActionError.value = null;
      modelActionMessage.value = null;

      try {
        const response = await gatewayExecute<EmbeddingActionResponse>(
          'maenifold.system',
          'EmbeddingModels',
          {
            action: 'activate',
            modelId: model.id,
          }
        );

        await refreshEmbeddingData();

        const restartHint = response.reembedRequired
          ? ' Restart maenifold to load the new model. Vectors will rebuild on the next sync.'
          : '';

        modelActionMessage.value = `${response.message ?? `Activated ${model.name}.`}${restartHint}`;
      } catch (error) {
        modelActionError.value = error instanceof Error ? error.message : String(error);
      } finally {
        modelActionState.value = 'idle';
      }
    }

    async function loadConfiguredAgents(): Promise<void> {
      agentsLoading.value = true;
      agentsError.value = null;
      defaultAgentError.value = null;

      try {
        const config = await gatewayExecute<DaemonConfigResponse>(
          'maenifold.system',
          'GetConfig',
          {}
        );

        configSnapshot.value = {
          agents: config.agents.map((agent) => ({
            name: agent.name,
            command: agent.command,
            args: agent.args ? [...agent.args] : undefined,
            env: agent.env ? { ...agent.env } : undefined,
          })),
          settings: config.settings ? { ...config.settings } : null,
        };

        configuredDefaultAgentName.value = readDefaultAgentName(configSnapshot.value);

        const preferredSelection =
          selectedAgentName.value &&
          configSnapshot.value.agents.some((agent) => agent.name === selectedAgentName.value)
            ? selectedAgentName.value
            : configuredDefaultAgentName.value ?? configSnapshot.value.agents[0]?.name ?? null;

        selectedAgentName.value = preferredSelection;
        agentsLoaded.value = true;
      } catch (error) {
        agentsError.value = error instanceof Error ? error.message : String(error);
      } finally {
        agentsLoading.value = false;
      }
    }

    function selectModel(modelId: string): void {
      if (modelActionState.value !== 'idle') return;
      selectedModelId.value = modelId;
      modelActionError.value = null;
      modelActionMessage.value = null;
    }

    function selectAgent(agentName: string): void {
      if (defaultAgentSaving.value) return;
      selectedAgentName.value = agentName;
      defaultAgentError.value = null;
      defaultAgentMessage.value = null;
    }

    function applySelectedAgent(): void {
      if (setDefaultDisabled.value || !selectedAgentName.value) return;

      defaultAgentSaving.value = true;
      defaultAgentError.value = null;
      defaultAgentMessage.value = null;

      try {
        // TODO: this depends on T-413.1's SetSettings being merged.
        console.warn("TODO: this depends on T-413.1's SetSettings being merged.", {
          settings: {
            defaultAgent: selectedAgentName.value,
          },
        });

        configuredDefaultAgentName.value = selectedAgentName.value;

        configSnapshot.value = {
          agents: configSnapshot.value?.agents ? [...configSnapshot.value.agents] : [],
          settings: {
            ...(configSnapshot.value?.settings ?? {}),
            defaultAgent: selectedAgentName.value,
          },
        };

        defaultAgentMessage.value = `${selectedAgentName.value} was selected locally. TODO: this depends on T-413.1's SetSettings being merged.`;
      } catch (error) {
        defaultAgentError.value = error instanceof Error ? error.message : String(error);
      } finally {
        defaultAgentSaving.value = false;
      }
    }

    function goBack(): void {
      if (currentStepIndex.value === 0) return;
      currentStepIndex.value -= 1;
    }

    function goToStep(index: number): void {
      if (index < 0 || index > highestVisitedStepIndex.value) return;
      currentStepIndex.value = index;
    }

    function handlePrimaryAction(): void {
      if (currentStepId.value === 'welcome') {
        currentStepIndex.value = 1;
        return;
      }

      if (currentStepId.value === 'done') {
        dismiss();
        emit('completed');
        return;
      }

      if (!canAdvance.value) return;

      if (currentStepIndex.value < steps.length - 1) {
        currentStepIndex.value += 1;
      }
    }

    function dismissWizard(): void {
      dismiss();
      emit('dismissed');
    }

    function handleDialogKeydown(event: KeyboardEvent): void {
      if (event.key === 'Escape') {
        event.preventDefault();
        dismissWizard();
        return;
      }

      if (event.key === 'Tab') {
        trapDialogFocus(event);
      }
    }

    function reopenWizard(): void {
      resetWizardProgress();
      reopen();
    }

    watch(currentStepIndex, (value) => {
      if (value > highestVisitedStepIndex.value) {
        highestVisitedStepIndex.value = value;
      }
    });

    watch(
      () => props.open,
      (open) => {
        if (open) {
          resetWizardProgress();
          isVisible.value = true;
          emit('update:open', true);
        } else {
          isVisible.value = false;
          emit('update:open', false);
        }
      }
    );

    watch(
      () => props.config,
      (config) => {
        if (configSnapshot.value !== null) return;
        const normalized = normalizeConfigSnapshot(config);
        if (!normalized) return;
        configSnapshot.value = normalized;
        configuredDefaultAgentName.value = readDefaultAgentName(normalized);
        if (!selectedAgentName.value) {
          selectedAgentName.value =
            configuredDefaultAgentName.value ?? normalized.agents[0]?.name ?? null;
        }
      },
      { immediate: true }
    );

    watch(
      () => props.embeddingStatus,
      (status) => {
        if (modelStatus.value !== null) return;
        modelStatus.value = normalizeEmbedStatus(status);
      },
      { immediate: true }
    );

    watch(
      [currentStepId, isVisible],
      ([step, visible]) => {
        if (!visible) return;

        if (step === 'daemon' && healthState.value === 'idle') {
          void checkDaemonHealth();
        }

        if (step === 'model' && !modelLoaded.value && !modelLoading.value) {
          void loadEmbeddingModels();
        }

        if (step === 'agent' && !agentsLoaded.value && !agentsLoading.value) {
          void loadConfiguredAgents();
        }
      },
      { immediate: true }
    );

    watch(
      isVisible,
      async (visible, wasVisible) => {
        if (typeof document === 'undefined') return;

        if (visible) {
          returnFocusTarget =
            document.activeElement instanceof HTMLElement ? document.activeElement : null;
          await nextTick();
          focusFirstDialogElement();
          return;
        }

        if (wasVisible) {
          await nextTick();
          restoreReturnFocus();
        }
      },
      { immediate: true, flush: 'post' }
    );

    onMounted(() => {
      if (typeof window !== 'undefined') {
        window.addEventListener(
          ONBOARDING_VISIBILITY_EVENT,
          handleVisibilityEvent as EventListener
        );
      }
    });

    onBeforeUnmount(() => {
      stopModelPolling();
      if (typeof window !== 'undefined') {
        window.removeEventListener(
          ONBOARDING_VISIBILITY_EVENT,
          handleVisibilityEvent as EventListener
        );
      }
    });

    expose({
      dismiss: dismissWizard,
      reopen: reopenWizard,
    });

    return {
      activateDisabled,
      agentsError,
      agentsLoading,
      canAdvance,
      checkDaemonHealth,
      configuredDefaultAgentName,
      currentEmbeddingDimensions,
      currentStepId,
      currentStepIndex,
      dialogRef,
      defaultAgentError,
      defaultAgentMessage,
      defaultAgentSaving,
      downloadDisabled,
      downloadPercent,
      effectiveConfig,
      effectiveModelStatus,
      footerHint,
      formatAgentCommand,
      formatModelSize,
      formatModelMetadata,
      goBack,
      goToStep,
      handleDialogKeydown,
      handlePrimaryAction,
      hasDownloadProgress,
      healthError,
      healthPayload,
      healthState,
      highestVisitedStepIndex,
      isVisible,
      modelActionError,
      modelActionMessage,
      modelActionState,
      modelCatalog,
      modelError,
      modelLoading,
      needsDimensionWarning,
      primaryDisabled,
      primaryLabel,
      recapDaemonStatus,
      recapModelName,
      reembedConceptCount,
      reembedMemoryFileCount,
      refreshEmbeddingData: loadEmbeddingModels,
      loadConfiguredAgents,
      selectedAgentName,
      selectedModel,
      selectedModelId,
      selectAgent,
      selectModel,
      setDefaultDisabled,
      stepNumberText,
      stepVisualIcon,
      steps,
      triggerAgent,
      triggerDefaultAgent,
      triggerModel,
      downloadSelectedModel,
      activateSelectedModel,
      applySelectedAgent,
    };
  },
});
</script>

<template>
  <div v-if="isVisible" class="onboarding-scrim">
    <div
      ref="dialogRef"
      class="onboarding-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
      tabindex="-1"
      @keydown="handleDialogKeydown"
    >
      <header class="wizard-header">
        <div class="wizard-brand">
          <Brain class="wizard-brand-icon" />
          <span id="onboarding-title" class="wizard-brand-name">Welcome to Maenifold</span>
        </div>
        <span class="wizard-step-count">{{ stepNumberText }}</span>
      </header>

      <section class="wizard-trigger-strip" aria-label="Onboarding trigger summary">
        <span class="wizard-trigger-label">Setup needed because</span>

        <span class="wizard-trigger-pill" :class="triggerAgent.variant">
          <CheckCircle2 v-if="triggerAgent.variant === 'ok'" class="wizard-trigger-icon" />
          <AlertTriangle
            v-else-if="triggerAgent.variant === 'bad'"
            class="wizard-trigger-icon"
          />
          <RefreshCcw v-else class="wizard-trigger-icon spinning" />
          <code>{{ triggerAgent.text }}</code>
        </span>

        <span class="wizard-trigger-pill" :class="triggerDefaultAgent.variant">
          <CheckCircle2
            v-if="triggerDefaultAgent.variant === 'ok'"
            class="wizard-trigger-icon"
          />
          <AlertTriangle
            v-else-if="triggerDefaultAgent.variant === 'bad'"
            class="wizard-trigger-icon"
          />
          <RefreshCcw v-else class="wizard-trigger-icon spinning" />
          <code>{{ triggerDefaultAgent.text }}</code>
        </span>

        <span class="wizard-trigger-pill" :class="triggerModel.variant">
          <CheckCircle2 v-if="triggerModel.variant === 'ok'" class="wizard-trigger-icon" />
          <AlertTriangle
            v-else-if="triggerModel.variant === 'bad'"
            class="wizard-trigger-icon"
          />
          <RefreshCcw v-else class="wizard-trigger-icon spinning" />
          <code>{{ triggerModel.text }}</code>
        </span>
      </section>

      <div class="wizard-body">
        <nav class="wizard-stepper" aria-label="Onboarding steps">
          <button
            v-for="(step, index) in steps"
            :key="step.id"
            type="button"
            class="wizard-step"
            :class="{
              active: index === currentStepIndex,
              done: index < currentStepIndex,
              upcoming: index > currentStepIndex,
            }"
            :disabled="index > highestVisitedStepIndex"
            @click="goToStep(index)"
          >
            <span class="wizard-step-dot">
              <Check v-if="index < currentStepIndex" class="wizard-step-dot-check" />
              <span v-else>{{ index + 1 }}</span>
            </span>
            <span class="wizard-step-label">{{ step.label }}</span>
          </button>
        </nav>

        <section class="wizard-content">
          <div class="wizard-step-visual">
            <component :is="stepVisualIcon" class="wizard-step-visual-icon" />
          </div>

          <template v-if="currentStepId === 'welcome'">
            <h2 class="wizard-title">Get your workspace ready</h2>
            <p class="wizard-lede">
              This setup flow verifies the local daemon, lets you choose an embedding
              model, and captures a default agent for the current UI session.
            </p>

            <div class="wizard-card-stack">
              <div class="wizard-card">
                <strong>Step 1</strong>
                <p>Confirm the wizard and review what will be configured.</p>
              </div>
              <div class="wizard-card">
                <strong>Step 2</strong>
                <p>Check <code>http://127.0.0.1:8080/api/health</code> before the UI relies on it.</p>
              </div>
              <div class="wizard-card">
                <strong>Steps 3–5</strong>
                <p>
                  Pick an embedding model, select a default agent, and review the recap
                  before entering the app.
                </p>
              </div>
            </div>
          </template>

          <template v-else-if="currentStepId === 'daemon'">
            <h2 class="wizard-title">Check the local daemon</h2>
            <p class="wizard-lede">
              The onboarding wizard uses the daemon for health checks, embedding-model
              actions, and configuration reads.
            </p>

            <div class="wizard-card-stack">
              <div
                class="status-row"
                :class="{
                  success: healthState === 'ok',
                  danger: healthState === 'error',
                }"
              >
                <CheckCircle2 v-if="healthState === 'ok'" class="status-row-icon" />
                <RefreshCcw v-else-if="healthState === 'loading'" class="status-row-icon spinning" />
                <AlertTriangle v-else class="status-row-icon" />

                <div class="status-row-copy">
                  <strong>Daemon health</strong>
                  <p v-if="healthState === 'ok'">
                    The daemon responded successfully at
                    <code>http://127.0.0.1:8080/api/health</code>.
                  </p>
                  <p v-else-if="healthState === 'loading'">
                    Checking the daemon health endpoint now…
                  </p>
                  <p v-else>
                    The wizard could not confirm the daemon health endpoint yet.
                  </p>
                </div>

                <code class="status-row-value">{{
                  healthPayload?.status ?? (healthState === 'loading' ? 'checking' : 'unknown')
                }}</code>
              </div>

              <div class="status-row">
                <Server class="status-row-icon" />
                <div class="status-row-copy">
                  <strong>Endpoint</strong>
                  <p>Health requests target the local daemon HTTP port used by the rest of the UI.</p>
                </div>
                <code class="status-row-value">127.0.0.1:8080</code>
              </div>
            </div>

            <div v-if="healthState === 'error'" class="warning-panel">
              <AlertTriangle class="warning-panel-icon" />
              <div>
                <strong>Troubleshooting</strong>
                <ul class="troubleshooting-list">
                  <li>Ensure the daemon is running with <code>maenifold --http --port 8080</code>.</li>
                  <li>Confirm <code>http://127.0.0.1:8080/api/health</code> responds locally.</li>
                  <li>Retry the check once the daemon is healthy.</li>
                </ul>
                <p v-if="healthError" class="inline-error">{{ healthError }}</p>
              </div>
            </div>
          </template>

          <template v-else-if="currentStepId === 'model'">
            <h2 class="wizard-title">Choose an embedding model</h2>
            <p class="wizard-lede">
              Embedding models convert text into vectors for semantic search across the
              knowledge graph. Select one, download it if needed, then activate it.
            </p>

            <div v-if="modelLoading" class="inline-state">
              <RefreshCcw class="inline-state-icon spinning" />
              <span>Loading models from <code>EmbeddingModels list</code>…</span>
            </div>

            <div v-else-if="modelError" class="warning-panel">
              <AlertTriangle class="warning-panel-icon" />
              <div>
                <strong>Unable to load the embedding catalog</strong>
                <p>{{ modelError }}</p>
              </div>
            </div>

            <div v-else class="wizard-card-stack">
              <span class="section-label">
                Available models · <span class="section-label-muted">live from EmbeddingModels list</span>
              </span>

              <p v-if="!modelCatalog || modelCatalog.models.length === 0" class="empty-state">
                No embedding models were returned by the daemon.
              </p>

              <div v-else class="model-list">
                <button
                  v-for="model in modelCatalog.models"
                  :key="model.id"
                  type="button"
                  class="model-row"
                  :class="{ selected: selectedModelId === model.id }"
                  :disabled="modelActionState !== 'idle'"
                  @click="selectModel(model.id)"
                >
                  <div class="model-row-main">
                    <div class="model-row-name">
                      <span>{{ model.name }}</span>
                      <span v-if="model.isActive" class="badge success">active</span>
                      <span v-else-if="model.installed" class="badge">installed</span>
                    </div>
                    <div class="model-row-meta">{{ formatModelMetadata(model) }}</div>
                  </div>
                  <span class="selection-indicator" :class="{ selected: selectedModelId === model.id }">
                    <span v-if="selectedModelId === model.id" class="selection-indicator-dot" />
                  </span>
                </button>
              </div>

              <div v-if="needsDimensionWarning" class="warning-panel">
                <AlertTriangle class="warning-panel-icon" />
                <div>
                  <strong>Dimension change warning</strong>
                  <p>
                    Switching from {{ currentEmbeddingDimensions }}d to
                    {{ selectedModel?.dimensions }}d requires re-embedding existing data.
                    <template
                      v-if="reembedConceptCount !== null || reembedMemoryFileCount !== null"
                    >
                      Current vec0 tables report
                      {{ reembedConceptCount ?? 'unknown' }} concepts and
                      {{ reembedMemoryFileCount ?? 'unknown' }} memory files.
                    </template>
                  </p>
                </div>
              </div>

              <div v-if="modelActionState === 'downloading' && selectedModel" class="progress-panel">
                <div class="progress-panel-head">
                  <strong>Downloading {{ selectedModel.name }}</strong>
                  <span v-if="hasDownloadProgress">{{ Math.round(downloadPercent) }}%</span>
                </div>
                <div class="progress-track">
                  <div
                    class="progress-value"
                    :class="{ indeterminate: !hasDownloadProgress }"
                    :style="hasDownloadProgress ? { width: `${downloadPercent}%` } : undefined"
                  />
                </div>
                <p class="progress-copy">
                  <template v-if="hasDownloadProgress">
                    Downloaded
                    {{ formatModelSize(selectedModel.downloadProgress?.current ?? 0) }}
                    of
                    {{ formatModelSize(selectedModel.downloadProgress?.total ?? 0) }}.
                  </template>
                  <template v-else>Preparing download…</template>
                </p>
              </div>

              <p v-if="effectiveModelStatus?.activeModelStatus?.message" class="inline-note">
                {{ effectiveModelStatus.activeModelStatus.message }}
              </p>
              <p v-if="modelActionMessage" class="inline-note">{{ modelActionMessage }}</p>
              <p v-if="modelActionError" class="inline-error">
                {{ modelActionError }}
              </p>
            </div>
          </template>

          <template v-else-if="currentStepId === 'agent'">
            <h2 class="wizard-title">Pick a default agent</h2>
            <p class="wizard-lede">
              The default agent is the one the UI will pre-select for new sessions.
              This step reads the live daemon config via <code>GetConfig</code>.
            </p>

            <div v-if="agentsLoading" class="inline-state">
              <RefreshCcw class="inline-state-icon spinning" />
              <span>Loading agents from <code>GetConfig.agents</code>…</span>
            </div>

            <div v-else-if="agentsError" class="warning-panel">
              <AlertTriangle class="warning-panel-icon" />
              <div>
                <strong>Unable to load configured agents</strong>
                <p>{{ agentsError }}</p>
              </div>
            </div>

            <div v-else class="wizard-card-stack">
              <span class="section-label">
                Available agents · <span class="section-label-muted">live from GetConfig.agents</span>
              </span>

              <p v-if="!effectiveConfig || effectiveConfig.agents.length === 0" class="empty-state">
                No agents were returned by the daemon configuration.
              </p>

              <div v-else class="model-list">
                <button
                  v-for="agent in effectiveConfig.agents"
                  :key="agent.name"
                  type="button"
                  class="model-row"
                  :class="{ selected: selectedAgentName === agent.name }"
                  :disabled="defaultAgentSaving"
                  @click="selectAgent(agent.name)"
                >
                  <div class="model-row-main">
                    <div class="model-row-name">
                      <span>{{ agent.name }}</span>
                      <span
                        v-if="configuredDefaultAgentName === agent.name"
                        class="badge success"
                      >
                        default
                      </span>
                    </div>
                    <div class="model-row-meta agent-command">
                      {{ formatAgentCommand(agent) }}
                    </div>
                  </div>
                  <span class="selection-indicator" :class="{ selected: selectedAgentName === agent.name }">
                    <span v-if="selectedAgentName === agent.name" class="selection-indicator-dot" />
                  </span>
                </button>
              </div>

              <div class="warning-panel">
                <AlertTriangle class="warning-panel-icon" />
                <div>
                  <strong>Stubbed until T-413.1 lands</strong>
                  <p>
                    TODO: this depends on T-413.1's SetSettings being merged.
                    Clicking <em>Set as default</em> updates this component locally and logs a
                    console warning instead of calling the daemon.
                  </p>
                </div>
              </div>

              <p v-if="defaultAgentMessage" class="inline-note">
                {{ defaultAgentMessage }}
              </p>
              <p v-if="defaultAgentError" class="inline-error">
                {{ defaultAgentError }}
              </p>
            </div>
          </template>

          <template v-else>
            <h2 class="wizard-title">You’re ready to enter the app</h2>
            <p class="wizard-lede">
              Review the setup summary below. The wizard can be re-opened later via the
              integration point delivered by T-413.2/T-413.7.
            </p>

            <div class="recap-list">
              <div class="recap-item">
                <CheckCircle2 class="recap-item-icon" />
                <div class="recap-item-copy">
                  <strong>Daemon health</strong>
                  <p>Checked against <code>http://127.0.0.1:8080/api/health</code>.</p>
                </div>
                <span class="recap-item-value">{{ recapDaemonStatus }}</span>
              </div>

              <div class="recap-item">
                <CheckCircle2 class="recap-item-icon" />
                <div class="recap-item-copy">
                  <strong>Embedding model</strong>
                  <p>
                    Current selection from <code>EmbeddingModels</code>.
                  </p>
                </div>
                <span class="recap-item-value">{{ recapModelName }}</span>
              </div>

              <div class="recap-item">
                <CheckCircle2 class="recap-item-icon" />
                <div class="recap-item-copy">
                  <strong>Default agent</strong>
                  <p>Current local selection for the wizard session.</p>
                </div>
                <span class="recap-item-value">{{
                  configuredDefaultAgentName ?? 'Not selected'
                }}</span>
              </div>
            </div>
          </template>
        </section>
      </div>

      <footer class="wizard-footer">
        <button
          type="button"
          class="wizard-button ghost"
          :disabled="currentStepIndex === 0"
          @click="goBack"
        >
          <ArrowLeft class="wizard-button-icon" />
          Back
        </button>

        <div class="wizard-footer-spacer" />

        <span class="wizard-footer-hint">{{ footerHint }}</span>

        <template v-if="currentStepId === 'daemon'">
          <button
            type="button"
            class="wizard-button"
            :disabled="healthState === 'loading'"
            @click="checkDaemonHealth"
          >
            <RefreshCcw class="wizard-button-icon" :class="{ spinning: healthState === 'loading' }" />
            Retry
          </button>
        </template>

        <template v-else-if="currentStepId === 'model'">
          <button type="button" class="wizard-button" :disabled="modelLoading" @click="refreshEmbeddingData">
            <RefreshCcw class="wizard-button-icon" :class="{ spinning: modelLoading }" />
            Refresh
          </button>
          <button
            type="button"
            class="wizard-button"
            :disabled="downloadDisabled"
            @click="downloadSelectedModel"
          >
            <Download class="wizard-button-icon" :class="{ spinning: modelActionState === 'downloading' }" />
            {{ modelActionState === 'downloading' ? 'Downloading…' : 'Download' }}
          </button>
          <button
            type="button"
            class="wizard-button"
            :disabled="activateDisabled"
            @click="activateSelectedModel"
          >
            <CheckCircle2 class="wizard-button-icon" :class="{ spinning: modelActionState === 'activating' }" />
            {{ modelActionState === 'activating' ? 'Activating…' : 'Activate' }}
          </button>
        </template>

        <template v-else-if="currentStepId === 'agent'">
          <button
            type="button"
            class="wizard-button"
            :disabled="agentsLoading"
            @click="loadConfiguredAgents"
          >
            <RefreshCcw class="wizard-button-icon" :class="{ spinning: agentsLoading }" />
            Refresh
          </button>
          <button
            type="button"
            class="wizard-button"
            :disabled="setDefaultDisabled"
            @click="applySelectedAgent"
          >
            <CheckCircle2 class="wizard-button-icon" :class="{ spinning: defaultAgentSaving }" />
            {{ defaultAgentSaving ? 'Saving…' : 'Set as default' }}
          </button>
        </template>

        <button
          type="button"
          class="wizard-button primary"
          :disabled="primaryDisabled"
          @click="handlePrimaryAction"
        >
          {{ primaryLabel }}
          <ArrowRight v-if="currentStepId !== 'done'" class="wizard-button-icon" />
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.onboarding-scrim {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(2px);
  z-index: 1100;
}

.onboarding-dialog {
  width: 760px;
  max-width: min(96vw, 760px);
  max-height: 92vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border-color);
  border-radius: 18px;
  background: var(--bg-main);
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
}

.wizard-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.wizard-brand {
  display: inline-flex;
  align-items: center;
  gap: 0.65rem;
  min-width: 0;
}

.wizard-brand-icon {
  width: 20px;
  height: 20px;
  color: var(--text-accent);
  flex-shrink: 0;
  filter: drop-shadow(0 2px 6px color-mix(in srgb, var(--text-accent) 30%, transparent));
}

.wizard-brand-name {
  font-weight: 700;
  color: var(--text-primary);
}

.wizard-step-count {
  margin-left: auto;
  font-size: 0.76rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.wizard-trigger-strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  padding: 0.6rem 1.25rem;
  border-bottom: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--text-accent) 6%, var(--bg-main));
  font-size: 0.76rem;
}

.wizard-trigger-label {
  color: var(--text-muted);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-right: 0.2rem;
}

.wizard-trigger-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.2rem 0.55rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--bg-main);
}

.wizard-trigger-pill.ok {
  color: #4ec98a;
  border-color: color-mix(in srgb, var(--bg-success) 40%, var(--border-color));
  background: color-mix(in srgb, var(--bg-success) 9%, var(--bg-main));
}

.wizard-trigger-pill.bad {
  color: #ff8585;
  border-color: color-mix(in srgb, var(--bg-danger) 50%, var(--border-color));
  background: color-mix(in srgb, var(--bg-danger) 9%, var(--bg-main));
}

.wizard-trigger-pill.pending {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.wizard-trigger-pill code {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.7rem;
}

.wizard-trigger-icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
}

.wizard-body {
  display: grid;
  grid-template-columns: 200px 1fr;
  flex: 1;
  min-height: 0;
}

.wizard-stepper {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: 1.1rem 0.9rem;
  border-right: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.wizard-step {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.55rem 0.6rem;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.wizard-step:disabled {
  cursor: default;
}

.wizard-step-dot {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
  border-radius: 50%;
  background: var(--bg-main);
  color: var(--text-muted);
  font-size: 0.74rem;
  font-weight: 700;
}

.wizard-step-dot-check {
  width: 12px;
  height: 12px;
}

.wizard-step.done {
  color: var(--text-primary);
}

.wizard-step.done .wizard-step-dot {
  color: #fff;
  background: var(--bg-success);
  border-color: var(--bg-success);
}

.wizard-step.active {
  color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 12%, transparent);
}

.wizard-step.active .wizard-step-dot {
  color: #fff;
  background: var(--text-accent);
  border-color: var(--text-accent);
}

.wizard-step.upcoming {
  color: var(--text-muted);
}

.wizard-step-label {
  font-size: 0.85rem;
}

.wizard-content {
  padding: 1.5rem 1.6rem;
  overflow-y: auto;
}

.wizard-step-visual {
  width: 48px;
  height: 48px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
  border: 1px solid color-mix(in srgb, var(--text-accent) 25%, var(--border-color));
  border-radius: 14px;
  background: color-mix(in srgb, var(--text-accent) 10%, transparent);
  color: var(--text-accent);
}

.wizard-step-visual-icon {
  width: 24px;
  height: 24px;
}

.wizard-title {
  margin: 0 0 0.4rem;
  font-size: 1.4rem;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--text-primary);
}

.wizard-lede {
  margin: 0 0 1.1rem;
  color: var(--text-secondary);
  font-size: 0.9rem;
  line-height: 1.5;
}

.wizard-card-stack {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.wizard-card,
.status-row,
.recap-item,
.inline-state,
.progress-panel,
.model-row {
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-sidebar) 45%, var(--bg-main));
}

.wizard-card {
  padding: 0.85rem 0.95rem;
}

.wizard-card strong,
.status-row-copy strong,
.recap-item-copy strong {
  display: block;
  margin-bottom: 0.15rem;
  color: var(--text-primary);
}

.wizard-card p,
.status-row-copy p,
.recap-item-copy p,
.warning-panel p {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.86rem;
  line-height: 1.5;
}

.section-label {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.section-label-muted {
  color: var(--text-muted);
  font-weight: 400;
  text-transform: none;
}

.inline-state {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.8rem 0.95rem;
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.inline-state code,
.warning-panel code,
.recap-item-copy code,
.wizard-card code {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.78rem;
}

.inline-state-icon {
  width: 16px;
  height: 16px;
  color: var(--text-accent);
  flex-shrink: 0;
}

.warning-panel {
  display: flex;
  gap: 0.65rem;
  padding: 0.8rem 0.95rem;
  border: 1px solid color-mix(in srgb, var(--bg-warning) 50%, var(--border-color));
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-warning) 12%, var(--bg-main));
  color: #745600;
}

@media (prefers-color-scheme: dark) {
  .warning-panel {
    color: #ffd479;
  }
}

.warning-panel strong {
  display: block;
  margin-bottom: 0.2rem;
}

.warning-panel-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  margin-top: 0.15rem;
}

.troubleshooting-list {
  margin: 0.45rem 0 0;
  padding-left: 1rem;
  color: inherit;
  font-size: 0.84rem;
}

.troubleshooting-list li + li {
  margin-top: 0.2rem;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 0.85rem;
}

.status-row.success {
  border-color: color-mix(in srgb, var(--bg-success) 40%, var(--border-color));
}

.status-row.danger {
  border-color: color-mix(in srgb, var(--bg-danger) 40%, var(--border-color));
}

.status-row-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: var(--text-secondary);
}

.status-row.success .status-row-icon {
  color: #4ec98a;
}

.status-row.danger .status-row-icon {
  color: #ff8585;
}

.status-row-copy {
  min-width: 0;
  flex: 1;
}

.status-row-value {
  margin-left: auto;
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.78rem;
}

.model-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.model-row {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
  padding: 0.75rem 0.85rem;
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease;
}

.model-row:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--text-accent) 40%, var(--border-color));
}

.model-row.selected {
  border-color: var(--text-accent);
  background: color-mix(in srgb, var(--text-accent) 8%, var(--bg-main));
}

.model-row:disabled {
  opacity: 0.7;
  cursor: wait;
}

.model-row-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.18rem;
}

.model-row-name {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}

.model-row-meta {
  color: var(--text-muted);
  font-size: 0.78rem;
  line-height: 1.45;
}

.agent-command {
  word-break: break-all;
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 0.08rem 0.42rem;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font-size: 0.64rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.badge.success {
  color: #4ec98a;
  border-color: color-mix(in srgb, var(--bg-success) 40%, var(--border-color));
  background: color-mix(in srgb, var(--bg-success) 9%, var(--bg-main));
}

.selection-indicator {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 1px solid var(--border-color);
  border-radius: 50%;
  background: var(--bg-main);
}

.selection-indicator.selected {
  border-color: var(--text-accent);
  background: var(--text-accent);
}

.selection-indicator-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #fff;
}

.progress-panel {
  padding: 0.85rem 0.95rem;
}

.progress-panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.55rem;
  color: var(--text-primary);
  font-size: 0.88rem;
}

.progress-track {
  position: relative;
  height: 8px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-sidebar) 70%, var(--bg-main));
}

.progress-value {
  height: 100%;
  border-radius: 999px;
  background: var(--text-accent);
  transition: width 0.2s ease;
}

.progress-value.indeterminate {
  width: 32%;
  animation: onboarding-progress 1.2s ease-in-out infinite;
}

.progress-copy {
  margin: 0.55rem 0 0;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.inline-note,
.inline-error,
.empty-state {
  margin: 0;
  font-size: 0.84rem;
  line-height: 1.5;
}

.inline-note,
.empty-state {
  color: var(--text-secondary);
}

.inline-error {
  color: #d33;
}

@media (prefers-color-scheme: dark) {
  .inline-error {
    color: #ff9d9d;
  }
}

.recap-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.recap-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 0.85rem;
}

.recap-item-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: #4ec98a;
}

.recap-item-copy {
  flex: 1;
  min-width: 0;
}

.recap-item-value {
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.78rem;
  text-align: right;
}

.wizard-footer {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.95rem 1.25rem;
  border-top: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-sidebar) 60%, var(--bg-main));
}

.wizard-footer-spacer {
  flex: 1;
}

.wizard-footer-hint {
  font-size: 0.74rem;
  color: var(--text-muted);
}

.wizard-button {
  display: inline-flex;
  align-items: center;
  gap: 0.42rem;
  padding: 0.5rem 0.95rem;
  border: 1px solid var(--border-color);
  border-radius: 9px;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font: inherit;
  font-size: 0.84rem;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, filter 0.15s ease;
}

.wizard-button:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.wizard-button.primary {
  color: #fff;
  background: var(--text-accent);
  border-color: var(--text-accent);
}

.wizard-button.primary:hover:not(:disabled) {
  filter: brightness(1.08);
}

.wizard-button.ghost {
  background: transparent;
}

.wizard-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.wizard-button-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.spinning {
  animation: onboarding-spin 0.9s linear infinite;
}

@keyframes onboarding-spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes onboarding-progress {
  0% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(320%);
  }
}

@media (max-width: 880px) {
  .onboarding-scrim {
    padding: 1rem;
  }

  .onboarding-dialog {
    max-height: 96vh;
  }

  .wizard-body {
    grid-template-columns: 1fr;
  }

  .wizard-stepper {
    flex-direction: row;
    overflow-x: auto;
    border-right: 0;
    border-bottom: 1px solid var(--border-color);
    padding: 0.9rem;
  }

  .wizard-step {
    flex: 0 0 auto;
  }

  .wizard-footer {
    flex-wrap: wrap;
  }

  .wizard-footer-hint {
    width: 100%;
    order: 3;
  }
}
</style>
