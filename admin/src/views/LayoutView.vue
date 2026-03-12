<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()

const nav = [
  { to: '/', label: '概览', exact: true },
  { to: '/credentials', label: '凭证管理' },
  { to: '/apps', label: '应用管理' },
  { to: '/keys', label: 'API 密钥' },
  { to: '/logs', label: '请求日志' },
]
</script>

<template>
  <div class="min-h-screen flex">
    <!-- Sidebar -->
    <aside class="w-56 bg-gray-900 text-white flex flex-col">
      <div class="px-6 py-5 font-bold text-lg tracking-tight border-b border-gray-700">
        API-Key-FY
      </div>
      <nav class="flex-1 px-3 py-4 space-y-1">
        <RouterLink
          v-for="item in nav"
          :key="item.to"
          :to="item.to"
          :exact="item.exact"
          class="block px-3 py-2 rounded-lg text-sm text-gray-300 hover:bg-gray-800 hover:text-white transition-colors"
          active-class="bg-gray-800 text-white"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <div class="px-6 py-4 border-t border-gray-700 text-sm">
        <p class="text-gray-400 truncate">{{ auth.user?.iam_username }}</p>
        <button @click="auth.logout()" class="mt-2 text-gray-400 hover:text-white text-xs">退出登录</button>
      </div>
    </aside>

    <!-- Main -->
    <main class="flex-1 bg-gray-50 overflow-auto">
      <RouterView />
    </main>
  </div>
</template>
