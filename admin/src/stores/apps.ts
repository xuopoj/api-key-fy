import { defineStore } from 'pinia'
import { ref } from 'vue'
import { appsApi, type App, type AppPayload } from '@/api'

export const useAppsStore = defineStore('apps', () => {
  const items = ref<App[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      items.value = await appsApi.list()
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function create(payload: AppPayload) {
    const app = await appsApi.create(payload)
    items.value.unshift(app)
    return app
  }

  async function update(id: string, payload: AppPayload) {
    const app = await appsApi.update(id, payload)
    const idx = items.value.findIndex(a => a.id === id)
    if (idx !== -1) items.value[idx] = app
    return app
  }

  async function remove(id: string) {
    await appsApi.delete(id)
    items.value = items.value.filter(a => a.id !== id)
  }

  return { items, loading, error, fetch, create, update, remove }
})
