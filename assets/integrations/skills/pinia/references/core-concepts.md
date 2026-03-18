# Pinia Core Concepts

This note keeps the highest-value local guidance for everyday [[Pinia]] work.

## Mental model

[[Pinia]] is global/shared [[state-management]] for [[Vue.js]]. A store is the place for state and business logic that should outlive or outscope an individual component.

Three pillars:

- **State** — reactive shared data
- **Getters** — derived state
- **Actions** — synchronous or asynchronous business logic

## Choosing store syntax

### Option Store

Choose this first when:

- the store shape is straightforward
- you want easy readability
- actions naturally operate on `this`

Example shape:

```ts
defineStore('counter', {
  state: () => ({ count: 0 }),
  getters: {
    doubleCount: (state) => state.count * 2,
  },
  actions: {
    increment() {
      this.count++
    },
  },
})
```

### Setup Store

Choose this when:

- you need Composition API primitives directly
- you want watchers or composables inside the store
- you need a more custom internal composition model

Important: return all state that should be part of the store, especially when [[SSR]], devtools, or plugins matter.

## Reactivity rules

- Reading and writing `store.count` is normal.
- Direct destructuring of state/getters breaks reactivity.
- Use `storeToRefs(store)` when extracting reactive values.
- Actions can be destructured directly.

## State mutation tools

- direct mutation: `store.count++`
- grouped mutations: `store.$patch({ ... })`
- complex grouped mutations: `store.$patch((state) => { ... })`
- Option Store reset: `store.$reset()`

## Actions

Actions are where business logic belongs.

- they can be async
- they can call other stores
- they are invoked as methods on the store
- Option Store actions should not be arrow functions when using `this`

Useful observation hooks:

- `store.$onAction()` to monitor action calls and outcomes
- `store.$subscribe()` to monitor state changes

## Plugins

Use `pinia.use()` to extend store behavior globally.

Common uses:

- persistence
- analytics
- injected helpers like router or toast APIs
- custom store options

SSR-sensitive plugin state should be handled carefully so it is represented on both `store` and `store.$state` when required.

## SSR rules

- serialize `pinia.state.value` on the server
- escape serialized state safely
- hydrate `pinia.state.value` before any client-side `useStore()` call
- when using stores outside `setup()`, pass the pinia instance explicitly if needed

## Use a store only when

- the state is shared across features, routes, or distant components
- the logic belongs to domain state, not one component's local UI concern

Avoid stores for:

- one-off modal visibility
- temporary form-only state tied to one component
- presentation-only toggles with no broader reuse
