---
name: pinia
description: Build and debug Vue.js state management with Pinia. Use when agents need to create, modify, or troubleshoot Pinia stores, actions, getters, plugins, storeToRefs usage, and SSR hydration in Vue 3 or TypeScript applications.
---

# Pinia

[[Pinia]] is the official intuitive store for [[Vue.js]]. Use it when application state and business logic should live outside individual components but still remain type-safe, reactive, and modular.

This skill is intentionally **hybrid**:

- **Local**: the core mental model, common patterns, and sharp edges you need for most day-to-day store work.
- **Dynamic**: the official [[Pinia]] docs for exact API details, cookbook guidance, migration notes, and version-specific behavior.

## When to use this skill

Use this skill when you need to:

- create or refactor `src/stores/*.ts` files
- choose between **Option Stores** and **Setup Stores**
- model [[state-management]] with `state`, `getters`, and `actions`
- safely destructure store state with `storeToRefs()`
- add [[Pinia plugins]] with `pinia.use()`
- debug store reactivity, subscriptions, or action flows
- implement or troubleshoot [[SSR]] hydration with [[Pinia]]

## Project structure

Typical [[Vue.js]] + [[Pinia]] layout:

```text
src/
├── main.ts              # createApp() + createPinia() + app.use(pinia)
├── stores/
│   ├── counter.ts       # one store per file
│   ├── auth.ts
│   └── cart.ts
├── plugins/
│   └── pinia-persist.ts # optional pinia.use() plugin
└── components/
    └── CartSummary.vue  # consumes stores
```

Prefer **one store per file** and instantiate stores only where needed.

## Quick start

Install and register [[Pinia]]:

```bash
npm install pinia
```

```ts
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)
app.mount('#app')
```

See `sample_codes/getting-started/create-pinia.md` and `sample_codes/getting-started/counter-store.md`.

## Core concepts

### 1. `defineStore()`

Every store is defined with `defineStore()` and needs a **unique id**.

- First argument: store id, e.g. `'cart'`
- Second argument: either an **Options object** or a **Setup function**

### 2. State, getters, actions

Pinia's mental model maps closely to component concepts:

- **state** → shared reactive data
- **getters** → derived/computed values
- **actions** → business logic and async workflows

Use stores for cross-component or cross-route state. Keep truly local UI state inside components.

### 3. Option Stores vs Setup Stores

**Option Stores** are simpler and usually the default choice:

- use `state`, `getters`, `actions`
- actions access the store through `this`
- best when you want clarity and conventional structure

**Setup Stores** are more flexible:

- return `ref()`, `computed()`, and functions directly
- work naturally with composables and watchers
- require more care for [[SSR]] and returned state

Rule of thumb:

- choose **Option Stores** for most business state
- choose **Setup Stores** when you truly need composables, watchers, or more custom composition

### 4. `storeToRefs()`

Do **not** destructure reactive store state directly.

```ts
const store = useCartStore()
const { items, total } = storeToRefs(store)
const { checkout } = store
```

Use `storeToRefs(store)` for reactive state/getters. Actions can be destructured directly from the store.

### 5. Plugins

Use `pinia.use()` to extend stores globally.

Plugins can:

- add properties or methods to stores
- observe state or actions
- implement persistence or analytics
- add custom store options

If plugin state must participate in [[SSR]], it must be added in a way that preserves both store access and `$state` serialization.

### 6. SSR hydration

For server-rendered apps:

- serialize `pinia.state.value` on the server
- hydrate `pinia.state.value` on the client
- hydrate **before** calling any `useStore()` function client-side

See `sample_codes/common-patterns/ssr-hydration.md`.

## Common patterns

### Option Store

See `sample_codes/getting-started/counter-store.ts`.

Use when you want a clean, standard store shape.

### Setup Store

See `sample_codes/common-patterns/setup-store.ts`.

Use when the store needs Composition API primitives directly.

### Persistence / side effects plugin

See `sample_codes/common-patterns/persist-plugin.ts`.

Good for local storage sync, analytics hooks, or wrapping store behavior.

### SSR hydration bootstrap

See `sample_codes/common-patterns/ssr-hydration.ts`.

Use when hydrating server-rendered state into the client app.

## Sharp edges and anti-patterns

- **Do not destructure store state directly** — use `storeToRefs()`.
- **Do not add undeclared state later** — define all state keys in `state()` up front.
- **Do not use arrow functions for Option Store actions** when relying on `this`.
- **Do not hide private state in Setup Stores** by omitting returned refs that are part of store state.
- **Do not call `useStore()` before hydration** in SSR client bootstrapping.
- **Do not overuse stores** for component-local UI toggles or ephemeral view-only state.

## Local reference map

Start here before browsing external docs:

| Topic | Local file |
|---|---|
| Core mental model and trade-offs | `references/core-concepts.md` |
| Official links and lookup prompts | `references/official-links.md` |
| Bootstrap Pinia in a Vue app | `sample_codes/getting-started/create-pinia.md` |
| Basic Option Store | `sample_codes/getting-started/counter-store.md` |
| Setup Store pattern | `sample_codes/common-patterns/setup-store.md` |
| Persistence plugin | `sample_codes/common-patterns/persist-plugin.md` |
| SSR hydration | `sample_codes/common-patterns/ssr-hydration.md` |

## Dynamic lookup guidance

When local guidance is not enough, fetch the official [[Pinia]] docs directly.

### Recommended official pages

- Overview: `https://pinia.vuejs.org/`
- Getting started: `https://pinia.vuejs.org/getting-started.html`
- Defining a store: `https://pinia.vuejs.org/core-concepts/`
- State: `https://pinia.vuejs.org/core-concepts/state.html`
- Getters: `https://pinia.vuejs.org/core-concepts/getters.html`
- Actions: `https://pinia.vuejs.org/core-concepts/actions.html`
- Plugins: `https://pinia.vuejs.org/core-concepts/plugins.html`
- SSR: `https://pinia.vuejs.org/ssr/`
- Cookbook: `https://pinia.vuejs.org/cookbook/`

### Use dynamic fetches when

- you need exact API signatures or edge-case behavior
- you are working on version migration
- you need cookbook recipes like testing, HMR, or composable integration
- you need Nuxt-specific Pinia guidance

### Suggested queries

- `Pinia storeToRefs destructuring`
- `Pinia setup store SSR`
- `Pinia plugin custom properties types`
- `Pinia subscribe patch action`
- `Pinia testing cookbook`

## Practical guidance for agents

When changing a [[Vue.js]] codebase with [[Pinia]]:

1. Identify whether the state is truly global/shared.
2. Prefer **Option Stores** unless flexibility is clearly needed.
3. Keep business logic in actions, not scattered across components.
4. Preserve reactivity when extracting data with `storeToRefs()`.
5. Treat [[SSR]] hydration order as a correctness issue, not a polish issue.
6. If adding plugins, verify whether plugin-added state must serialize into `$state`.

When accuracy matters more than convenience, prefer the official Pinia docs above.
