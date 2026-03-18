import type { PiniaPluginContext } from 'pinia'

export function persistPlugin({ store }: PiniaPluginContext) {
  const key = `pinia:${store.$id}`
  const existing = localStorage.getItem(key)

  if (existing) {
    store.$patch(JSON.parse(existing))
  }

  store.$subscribe((_mutation, state) => {
    localStorage.setItem(key, JSON.stringify(state))
  })
}
