# Setup Store Pattern

```ts
import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

export const useCartStore = defineStore('cart', () => {
  const items = ref<{ id: string; price: number; quantity: number }[]>([])

  const total = computed(() =>
    items.value.reduce((sum, item) => sum + item.price * item.quantity, 0)
  )

  function addItem(id: string, price: number) {
    const existing = items.value.find((item) => item.id === id)
    if (existing) {
      existing.quantity += 1
      return
    }

    items.value.push({ id, price, quantity: 1 })
  }

  function $reset() {
    items.value = []
  }

  return {
    items,
    total,
    addItem,
    $reset,
  }
})
```
