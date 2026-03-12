<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useAppsStore } from '@/stores/apps'
import { useKeysStore } from '@/stores/keys'
import { useCredentialsStore } from '@/stores/credentials'
import { useLogsStore } from '@/stores/logs'

const apps = useAppsStore()
const keys = useKeysStore()
const creds = useCredentialsStore()
const logs = useLogsStore()

onMounted(async () => {
  await Promise.all([apps.fetch(), keys.fetch(), creds.fetch(), logs.fetch({ reset: true })])
})

const activeKeys = computed(() => keys.items.filter(k => !k.revoked_at).length)
const recentLogs = computed(() => logs.items.slice(0, 10))
</script>

<template>
  <div class="p-8 space-y-8">
    <h1 class="text-2xl font-bold text-gray-900">概览</h1>

    <!-- Stats -->
    <div class="grid grid-cols-4 gap-4">
      <div class="bg-white rounded-xl shadow-sm p-5">
        <p class="text-sm text-gray-500">应用数量</p>
        <p class="text-3xl font-bold mt-1">{{ apps.items.length }}</p>
      </div>
      <div class="bg-white rounded-xl shadow-sm p-5">
        <p class="text-sm text-gray-500">有效密钥</p>
        <p class="text-3xl font-bold mt-1">{{ activeKeys }}</p>
      </div>
      <div class="bg-white rounded-xl shadow-sm p-5">
        <p class="text-sm text-gray-500">凭证数量</p>
        <p class="text-3xl font-bold mt-1">{{ creds.items.length }}</p>
      </div>
      <div class="bg-white rounded-xl shadow-sm p-5">
        <p class="text-sm text-gray-500">近期请求</p>
        <p class="text-3xl font-bold mt-1">{{ logs.items.length }}</p>
      </div>
    </div>

    <!-- Recent logs -->
    <div class="bg-white rounded-xl shadow-sm">
      <div class="px-6 py-4 border-b font-medium text-gray-700">近期请求</div>
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 text-xs uppercase">
          <tr>
            <th class="px-6 py-3 text-left">方法</th>
            <th class="px-6 py-3 text-left">路径</th>
            <th class="px-6 py-3 text-left">状态码</th>
            <th class="px-6 py-3 text-left">延迟</th>
            <th class="px-6 py-3 text-left">时间</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="log in recentLogs" :key="log.id" class="hover:bg-gray-50">
            <td class="px-6 py-3 font-mono font-medium">{{ log.method }}</td>
            <td class="px-6 py-3 font-mono text-gray-600 truncate max-w-xs">{{ log.path }}</td>
            <td class="px-6 py-3">
              <span :class="log.status_code && log.status_code < 400 ? 'text-green-700 bg-green-50' : 'text-red-700 bg-red-50'" class="px-2 py-0.5 rounded text-xs font-medium">
                {{ log.status_code ?? '—' }}
              </span>
            </td>
            <td class="px-6 py-3 text-gray-500">{{ log.latency_ms != null ? `${log.latency_ms}ms` : '—' }}</td>
            <td class="px-6 py-3 text-gray-400 text-xs">{{ new Date(log.created_at).toLocaleTimeString() }}</td>
          </tr>
          <tr v-if="!recentLogs.length">
            <td colspan="5" class="px-6 py-8 text-center text-gray-400">暂无请求记录</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
