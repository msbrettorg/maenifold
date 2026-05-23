<script setup lang="ts">
import type { MemoryFile } from '../lib/workflow-types'
import { nextTick, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'
import { getMarkdownIt } from '../composables/useMarkdownIt'
import { useMemoryApi } from '../composables/useMemoryApi'
import { useMermaid } from '../composables/useMermaid'
import { ToolEnvelopeError } from '../lib/mcp-response'
import { collectFolderPaths } from '../lib/knowledge-helpers'
import { ALLOWED_URI_REGEXP, sanitize } from '../lib/sanitizer'
import { useAppShellStore } from '../stores/app-shell'
import { useKnowledgeStore } from '../stores/knowledge'
import MemoryPreviewCard from './MemoryPreviewCard.vue'
import { HoverCardContent, HoverCardRoot, HoverCardTrigger } from './ui/hover-card'
import WikilinkCreateDialog from './WikilinkCreateDialog.vue'

const props = withDefaults(defineProps<{ content: string; clamp?: boolean; wikilinkAction?: 'open-note' | 'search-knowledge' }>(), { clamp: false, wikilinkAction: 'open-note' })
interface MemoryPreview { title: string; firstParagraph: string; modified: string | null }
interface HoveredMemoryTarget { cacheKey: string; identifier: string; element: HTMLElement }

const containerRef = useTemplateRef<HTMLElement>('containerRef')
const previewPanelRef = useTemplateRef<HTMLElement>('previewPanelRef')
const rendered = ref('')
const isUnmounted = ref(false)
const createDialogOpen = ref(false), unresolvedConcept = ref('')
const hoverCardOpen = ref(false), hoveredPreview = ref<MemoryPreview | null>(null), hoveredPreviewKey = ref<string | null>(null), hoverAnchor = ref<HTMLElement>()
const appShellStore = useAppShellStore()
const knowledgeStore = useKnowledgeStore()
const memoryApi = useMemoryApi()
const { renderMermaidBlocks } = useMermaid()
const wikilinkExistsCache = new Map<string, boolean>()
const memoryPreviewCache = new Map<string, MemoryPreview>()
const memoryPreviewRequests = new Map<string, Promise<MemoryPreview | null>>()
const forbiddenTags = ['script', 'iframe', 'object', 'embed', 'form', 'input', 'button', 'textarea']
const hoverCardDelay = 300
let renderVersion = 0
let unresolvedCheckTimer: number | null = null
let hoverTimer: number | null = null

function isNotFoundError(error: unknown): boolean { return error instanceof ToolEnvelopeError && error.code === 'NOT_FOUND' }
function toMemoryUri(concept: string): string { return `memory://${concept}` }
function memoryKeys(cacheKey: string, identifier: string): string[] { return cacheKey === identifier ? [cacheKey] : [cacheKey, identifier] }
function getCachedExists(...keys: string[]): boolean | undefined { for (const key of keys) { const cached = wikilinkExistsCache.get(key); if (cached !== undefined) return cached } }
function getFirstParagraph(content: string): string { return content.split(/\n\s*\n/u).map((paragraph) => paragraph.replace(/\s+/gu, ' ').trim()).find(Boolean) ?? '' }
function buildMemoryPreview(memory: MemoryFile): MemoryPreview { return { title: memory.title || memory.uri, firstParagraph: getFirstParagraph(memory.content), modified: memory.modified } }
function clearHoverTimer(): void { if (hoverTimer !== null) window.clearTimeout(hoverTimer); hoverTimer = null }
function hidePreview(): void { clearHoverTimer(); hoverCardOpen.value = false; hoveredPreview.value = null; hoveredPreviewKey.value = null; hoverAnchor.value = undefined }
function sameHoveredTarget(left: HoveredMemoryTarget | null, right: HoveredMemoryTarget | null): boolean { return left?.cacheKey === right?.cacheKey && left?.identifier === right?.identifier }

function cacheMemoryState(cacheKey: string, identifier: string, exists: boolean, preview?: MemoryPreview): void {
  for (const key of memoryKeys(cacheKey, identifier)) {
    wikilinkExistsCache.set(key, exists)
    if (preview) memoryPreviewCache.set(key, preview)
  }
}

async function readMemoryPreview(cacheKey: string, identifier: string): Promise<MemoryPreview | null> {
  const cached = memoryPreviewCache.get(cacheKey) ?? memoryPreviewCache.get(identifier)
  if (cached) { cacheMemoryState(cacheKey, identifier, true, cached); return cached }
  const pending = memoryPreviewRequests.get(identifier)
  if (pending) return pending
  const request = memoryApi.readMemory(identifier).then((memory) => {
    const preview = buildMemoryPreview(memory)
    cacheMemoryState(cacheKey, identifier, true, preview)
    return preview
  }).catch((error) => {
    if (isNotFoundError(error)) { cacheMemoryState(cacheKey, identifier, false); return null }
    throw error
  }).finally(() => { memoryPreviewRequests.delete(identifier) })
  memoryPreviewRequests.set(identifier, request)
  return request
}

async function conceptExists(concept: string): Promise<boolean | null> {
  const identifier = toMemoryUri(concept)
  const cached = getCachedExists(concept, identifier)
  if (cached !== undefined) return cached
  try { return (await readMemoryPreview(concept, identifier)) !== null } catch { return null }
}

async function runWithLimit<T>(items: T[], limit: number, fn: (item: T) => Promise<void>): Promise<void> {
  let nextIndex = 0
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => { while (nextIndex < items.length) await fn(items[nextIndex++]) }))
}

async function markUnresolvedWikilinks(): Promise<void> {
  const chips = Array.from(containerRef.value?.querySelectorAll<HTMLElement>('[data-wikilink]') ?? [])
  await runWithLimit(chips, 4, async (chip) => {
    const concept = chip.getAttribute('data-wikilink')
    if (!concept) return
    const exists = await conceptExists(concept)
    if (exists === false) chip.setAttribute('data-unresolved', 'true')
    else chip.removeAttribute('data-unresolved')
  })
}

function scheduleUnresolvedCheck(): void {
  if (unresolvedCheckTimer !== null) window.clearTimeout(unresolvedCheckTimer)
  unresolvedCheckTimer = window.setTimeout(() => { void markUnresolvedWikilinks() }, 60)
}

function resolveHoveredMemoryTarget(element: Element | null): HoveredMemoryTarget | null {
  if (!element || !containerRef.value) return null
  const wikilink = element.closest<HTMLElement>('[data-wikilink]')
  if (wikilink && containerRef.value.contains(wikilink)) {
    const concept = wikilink.getAttribute('data-wikilink')
    if (!concept) return null
    return { cacheKey: concept, identifier: toMemoryUri(concept), element: wikilink }
  }
  const memoryLink = element.closest<HTMLAnchorElement>('a[href^="memory://"]')
  const href = memoryLink?.getAttribute('href') ?? ''
  if (!memoryLink || !containerRef.value.contains(memoryLink) || href === 'memory://' || href === 'memory:///') return null
  return { cacheKey: href, identifier: href, element: memoryLink }
}

function schedulePreview(target: HoveredMemoryTarget): void {
  clearHoverTimer()
  hoverAnchor.value = target.element
  hoveredPreviewKey.value = target.cacheKey
  hoverTimer = window.setTimeout(() => { void openPreview(target) }, hoverCardDelay)
}

async function openPreview(target: HoveredMemoryTarget): Promise<void> {
  hoveredPreviewKey.value = target.cacheKey
  hoverAnchor.value = target.element
  const preview = await readMemoryPreview(target.cacheKey, target.identifier)
  if (hoveredPreviewKey.value !== target.cacheKey || hoverAnchor.value !== target.element || !preview) return
  hoveredPreview.value = preview
  hoverCardOpen.value = true
}

function restoreForeignObjectText(safeHtml: string, originalHtml: string): string {
  if (!originalHtml.includes('<foreignObject') && !originalHtml.includes('<foreignobject')) return safeHtml
  const parser = new DOMParser()
  const originalDocument = parser.parseFromString(`<body>${originalHtml}</body>`, 'text/html')
  const safeDocument = parser.parseFromString(`<body>${safeHtml}</body>`, 'text/html')
  const originalForeignObjects = Array.from(originalDocument.body.querySelectorAll('foreignObject, foreignobject'))
  Array.from(safeDocument.body.querySelectorAll('foreignObject, foreignobject')).forEach((safeNode, index) => {
    const originalNode = originalForeignObjects[index]
    if (!originalNode) return
    const sanitizedInnerHtml = sanitize(originalNode.innerHTML, { USE_PROFILES: { html: true }, FORBID_TAGS: [...forbiddenTags, 'style'], ALLOWED_URI_REGEXP })
    const container = document.createElement('div')
    container.innerHTML = sanitizedInnerHtml
    if (container.textContent) safeNode.textContent = container.textContent
  })
  return safeDocument.body.innerHTML
}

async function updateRendered(): Promise<void> {
  const thisVersion = ++renderVersion
  const source = props.content ?? ''
  hidePreview()
  if (!source) { rendered.value = ''; return }
  try {
    const md = await getMarkdownIt()
    if (thisVersion !== renderVersion) return
    const html = md.render(source)
    const safe = sanitize(html, {
      USE_PROFILES: { html: true, svg: true, svgFilters: true },
      ADD_TAGS: ['style', 'foreignObject', 'use'],
      ADD_ATTR: ['class', 'style', 'data-wikilink', 'data-mermaid-processed', 'disabled', 'checked', 'xmlns', 'viewBox', 'fill', 'stroke', 'd', 'cx', 'cy', 'r', 'width', 'height', 'x', 'y', 'rx', 'ry', 'transform', 'points', 'x1', 'y1', 'x2', 'y2', 'id', 'href', 'xlink:href', 'xmlns:xlink'],
      FORBID_TAGS: forbiddenTags,
      ALLOWED_URI_REGEXP,
    })
    if (thisVersion !== renderVersion) return
    rendered.value = restoreForeignObjectText(safe, html)
  } catch {
    if (thisVersion !== renderVersion) return
    rendered.value = source.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/\n/g, '<br>')
    return
  }
  await nextTick()
  if (thisVersion !== renderVersion || !containerRef.value) return
  await renderMermaidBlocks(containerRef.value, () => !isUnmounted.value)
  scheduleUnresolvedCheck()
}

async function handleClick(event: MouseEvent): Promise<void> {
  if (!(event.target instanceof Element)) return
  const linkTarget = event.target.closest<HTMLAnchorElement>('a')
  const href = linkTarget?.getAttribute('href') ?? ''
  if (href.startsWith('memory://')) {
    event.preventDefault()
    if (href === 'memory://' || href === 'memory:///') return
    if (event.metaKey || event.ctrlKey) knowledgeStore.openInNewTab(href)
    else knowledgeStore.openNote(href)
    appShellStore.setActiveTab('knowledge')
    return
  }
  const target = event.target.closest<HTMLElement>('[data-wikilink]')
  const concept = target?.getAttribute('data-wikilink')
  if (!concept) return
  event.preventDefault()
  if (props.wikilinkAction === 'search-knowledge') {
    knowledgeStore.setSearchQuery(concept)
    appShellStore.setActiveTab('knowledge')
    return
  }
  const uri = toMemoryUri(concept)
  const exists = await conceptExists(concept)
  if (exists === false) { unresolvedConcept.value = concept; createDialogOpen.value = true; return }
  if (exists === null) return
  if (event.metaKey || event.ctrlKey) { knowledgeStore.openInNewTab(uri); return }
  knowledgeStore.openNote(uri)
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key !== ' ' || !(event.target instanceof Element)) return
  const target = event.target.closest<HTMLElement>('[data-wikilink]')
  if (!target) return
  event.preventDefault()
  target.click()
}

function handleMouseOver(event: MouseEvent): void {
  if (!(event.target instanceof Element)) return
  const next = resolveHoveredMemoryTarget(event.target)
  const previous = event.relatedTarget instanceof Element ? resolveHoveredMemoryTarget(event.relatedTarget) : null
  if (!next || sameHoveredTarget(next, previous)) return
  schedulePreview(next)
}

function handleMouseOut(event: MouseEvent): void {
  if (!(event.target instanceof Element)) return
  const previous = resolveHoveredMemoryTarget(event.target)
  const next = event.relatedTarget instanceof Element ? resolveHoveredMemoryTarget(event.relatedTarget) : null
  if (!previous || sameHoveredTarget(previous, next)) return
  if (event.relatedTarget instanceof Element && previewPanelRef.value?.contains(event.relatedTarget)) { clearHoverTimer(); return }
  hidePreview()
}

function handlePreviewLeave(event: PointerEvent): void {
  const next = event.relatedTarget instanceof Element ? resolveHoveredMemoryTarget(event.relatedTarget) : null
  if (next && hoveredPreviewKey.value === next.cacheKey) return
  hidePreview()
}

function handleCreated(): void {
  wikilinkExistsCache.set(unresolvedConcept.value, true)
  void knowledgeStore.loadRootFolders()
  void markUnresolvedWikilinks()
}

watch(() => props.content, updateRendered, { immediate: true })
onBeforeUnmount(() => { isUnmounted.value = true })

defineExpose({ markUnresolvedWikilinks, wikilinkExistsCache, memoryPreviewCache })
</script>

<template>
  <article ref="containerRef" class="prose prose-sm min-w-0 max-w-full [overflow-wrap:anywhere] dark:prose-invert [&_a[data-wikilink][data-unresolved=true]]:border-dashed [&_a[data-wikilink][data-unresolved=true]]:opacity-70 [&_pre]:max-w-full [&_pre]:overflow-x-auto [&_pre]:whitespace-pre [&_pre_code]:whitespace-pre [&_table]:block [&_table]:max-w-full [&_table]:overflow-x-auto" :class="clamp ? 'line-clamp-6' : ''" @click="handleClick" @keydown="handleKeydown" @mouseover="handleMouseOver" @mouseout="handleMouseOut" v-html="rendered" />
  <HoverCardRoot :open="hoverCardOpen" :open-delay="hoverCardDelay" :close-delay="0">
    <HoverCardTrigger as-child :reference="hoverAnchor"><span aria-hidden="true" class="sr-only">Memory preview</span></HoverCardTrigger>
    <HoverCardContent v-if="hoveredPreview" side="top" align="start"><div ref="previewPanelRef" @pointerenter="clearHoverTimer" @pointerleave="handlePreviewLeave"><MemoryPreviewCard :uri="hoveredPreviewKey ?? ''" :title="hoveredPreview.title" :first-paragraph="hoveredPreview.firstParagraph" :modified="hoveredPreview.modified" /></div></HoverCardContent>
  </HoverCardRoot>
  <WikilinkCreateDialog v-model:open="createDialogOpen" :concept="unresolvedConcept" :available-folders="collectFolderPaths(knowledgeStore.folders)" @created="handleCreated" />
</template>
