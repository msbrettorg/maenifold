import { defineStore } from 'pinia'

export const useCounterStore = defineStore('counter', {
  state: () => ({
    count: 0,
    name: 'Counter',
  }),
  getters: {
    doubleCount: (state) => state.count * 2,
  },
  actions: {
    increment() {
      this.count++
    },
    async incrementLater(delayMs = 250) {
      await new Promise((resolve) => setTimeout(resolve, delayMs))
      this.increment()
    },
  },
})
