<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{ payload: unknown }>();

type TokType = 'key' | 'str' | 'num' | 'bool' | 'nil' | 'p';

function tokenize(obj: unknown, depth = 0): Array<[TokType, string]> {
  const pad = '  '.repeat(depth);
  const t: Array<[TokType, string]> = [];

  if (obj === null || obj === undefined) {
    t.push(['nil', 'null']);
  } else if (typeof obj === 'boolean') {
    t.push(['bool', String(obj)]);
  } else if (typeof obj === 'number') {
    t.push(['num', String(obj)]);
  } else if (typeof obj === 'string') {
    t.push(['str', JSON.stringify(obj)]);
  } else if (Array.isArray(obj)) {
    if (obj.length === 0) {
      t.push(['p', '[]']);
    } else {
      t.push(['p', '[\n']);
      obj.forEach((item, i) => {
        t.push(['p', pad + '  ']);
        t.push(...tokenize(item, depth + 1));
        if (i < obj.length - 1) t.push(['p', ',']);
        t.push(['p', '\n']);
      });
      t.push(['p', pad + ']']);
    }
  } else if (typeof obj === 'object') {
    const entries = Object.entries(obj as Record<string, unknown>);
    if (entries.length === 0) {
      t.push(['p', '{}']);
    } else {
      t.push(['p', '{\n']);
      entries.forEach(([key, val], i) => {
        t.push(['p', pad + '  ']);
        t.push(['key', JSON.stringify(key)]);
        t.push(['p', ': ']);
        t.push(...tokenize(val, depth + 1));
        if (i < entries.length - 1) t.push(['p', ',']);
        t.push(['p', '\n']);
      });
      t.push(['p', pad + '}']);
    }
  } else {
    t.push(['str', String(obj)]);
  }
  return t;
}

const tokens = computed(() => {
  try {
    return tokenize(props.payload);
  } catch {
    return [['str', String(props.payload)] as [TokType, string]];
  }
});
</script>

<template>
  <pre class="json-payload"><!--
  --><span
    v-for="token in tokens"
    :class="`t-${token[0]}`"
  >{{ token[1] }}</span></pre>
</template>

<style scoped>
.json-payload {
  margin: 0;
  padding: 0.5rem 0.75rem 0.75rem 2rem;
  background: var(--color-bg-app);
  font-family: var(--font-code);
  font-size: var(--text-tiny);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
}

.t-key { color: var(--accent-prompt); }
.t-str { color: var(--sage); }
.t-num { color: var(--amber); }
.t-bool { color: var(--plum); }
.t-nil { color: var(--text-dim); font-style: italic; }
.t-p { color: var(--text-dim); }
</style>
