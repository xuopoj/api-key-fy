import { defineStore } from 'pinia'
import { ref } from 'vue'
import { keysApi, type ApiKey, type CreateKeyResponse, type KeyPayload, type KeyStats } from '@/api'

export const useKeysStore = defineStore('keys', () => {
  const items = ref<ApiKey[]>([])
  const stats = ref<Record<string, KeyStats>>({})
  const loading = ref(false)
  const error = ref<string | null>(null)
  const newKeySecret = ref<string | null>(null) // shown once after creation

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      const [keys, keyStats] = await Promise.all([keysApi.list(), keysApi.stats()])
      items.value = keys
      stats.value = Object.fromEntries(keyStats.map(s => [s.api_key_id, s]))
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function create(payload: KeyPayload): Promise<CreateKeyResponse> {
    const result = await keysApi.create(payload)
    newKeySecret.value = result.key
    const { key: _, ...meta } = result
    items.value.unshift(meta)
    return result
  }

  async function remove(id: string) {
    await keysApi.revoke(id)
    items.value = items.value.filter(k => k.id !== id)
  }

  async function setApps(id: string, app_ids: string[]) {
    await keysApi.setApps(id, app_ids)
  }

  function clearNewKeySecret() {
    newKeySecret.value = null
  }

  return { items, stats, loading, error, newKeySecret, fetch, create, remove, setApps, clearNewKeySecret }
})
