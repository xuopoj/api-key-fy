<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const username = ref('')
const password = ref('')
const domain = ref(import.meta.env.VITE_DEFAULT_DOMAIN ?? '')
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50">
    <div class="w-full max-w-sm bg-white rounded-xl shadow p-8 space-y-6">
      <h1 class="text-2xl font-bold text-gray-900">API-Key-FY</h1>
      <p class="text-sm text-gray-500">使用华为云 IAM 账号登录</p>

      <form @submit.prevent="auth.login(username, password, domain)" class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">账号 / Domain</label>
          <input v-model="domain" required class="w-full border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" placeholder="账号名称" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">IAM 用户名</label>
          <input v-model="username" required class="w-full border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" placeholder="用户名" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">密码</label>
          <input v-model="password" type="password" required class="w-full border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" />
        </div>

        <p v-if="auth.error" class="text-sm text-red-600">{{ auth.error }}</p>

        <button type="submit" :disabled="auth.loading" class="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 rounded-lg text-sm disabled:opacity-50">
          {{ auth.loading ? '登录中…' : '登录' }}
        </button>
      </form>
    </div>
  </div>
</template>
