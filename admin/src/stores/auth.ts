import { defineStore } from 'pinia'
import { ref } from 'vue'
import { authApi, type User } from '@/api'
import router from '@/router'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchMe() {
    try {
      user.value = await authApi.me()
    } catch {
      user.value = null
    }
  }

  async function login(iam_username: string, iam_password: string, iam_domain: string) {
    loading.value = true
    error.value = null
    try {
      user.value = await authApi.login({ iam_username, iam_password, iam_domain })
      router.push('/')
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function logout() {
    await authApi.logout()
    user.value = null
    router.push('/login')
  }

  return { user, loading, error, login, logout, fetchMe }
})
