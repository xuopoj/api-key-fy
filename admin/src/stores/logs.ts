import { defineStore } from 'pinia'
import { ref } from 'vue'
import { logsApi, type RequestLog } from '@/api'

export const useLogsStore = defineStore('logs', () => {
  const items = ref<RequestLog[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const offset = ref(0)
  const limit = 50

  async function fetch(params?: { api_key_id?: string; app_id?: string; reset?: boolean }) {
    if (params?.reset) offset.value = 0
    loading.value = true
    error.value = null
    try {
      const rows = await logsApi.list({ ...params, limit, offset: offset.value })
      if (params?.reset) {
        items.value = rows
      } else {
        items.value.push(...rows)
      }
      offset.value += rows.length
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  return { items, loading, error, offset, limit, fetch }
})
