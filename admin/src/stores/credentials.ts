import { defineStore } from 'pinia'
import { ref } from 'vue'
import { credentialsApi, type Credential, type CredentialPayload } from '@/api'

export const useCredentialsStore = defineStore('credentials', () => {
  const items = ref<Credential[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      items.value = await credentialsApi.list()
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function create(payload: CredentialPayload) {
    const cred = await credentialsApi.create(payload)
    items.value.unshift(cred)
    return cred
  }

  async function update(id: string, payload: CredentialPayload) {
    const cred = await credentialsApi.update(id, payload)
    const idx = items.value.findIndex(c => c.id === id)
    if (idx !== -1) items.value[idx] = cred
    return cred
  }

  async function remove(id: string) {
    await credentialsApi.delete(id)
    items.value = items.value.filter(c => c.id !== id)
  }

  return { items, loading, error, fetch, create, update, remove }
})
