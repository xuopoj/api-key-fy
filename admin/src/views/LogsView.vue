<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useLogsStore } from '@/stores/logs'
import { useAppsStore } from '@/stores/apps'
import { useKeysStore } from '@/stores/keys'

const store = useLogsStore()
const apps = useAppsStore()
const keys = useKeysStore()

const filterAppId = ref<string>('')
const filterKeyId = ref<string>('')

onMounted(() => {
  Promise.all([apps.fetch(), keys.fetch()])
  store.fetch({ reset: true })
})

function applyFilter() {
  store.fetch({
    reset: true,
    app_id: filterAppId.value || undefined,
    api_key_id: filterKeyId.value || undefined,
  })
}

function loadMore() {
  store.fetch({
    app_id: filterAppId.value || undefined,
    api_key_id: filterKeyId.value || undefined,
  })
}
</script>

<template>
  <div class="p-8 space-y-6">
    <h1 class="text-2xl font-bold text-gray-900">请求日志</h1>

    <!-- Filters -->
    <div class="flex gap-3">
      <select v-model="filterAppId" @change="applyFilter" class="border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
        <option value="">全部应用</option>
        <option v-for="a in apps.items" :key="a.id" :value="a.id">{{ a.name }}</option>
      </select>
      <select v-model="filterKeyId" @change="applyFilter" class="border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
        <option value="">全部密钥</option>
        <option v-for="k in keys.items" :key="k.id" :value="k.id">{{ k.name }} ({{ k.key_prefix }}…)</option>
      </select>
    </div>

    <div class="bg-white rounded-xl shadow-sm">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 text-xs uppercase">
          <tr>
            <th class="px-6 py-3 text-left">时间</th>
            <th class="px-6 py-3 text-left">应用</th>
            <th class="px-6 py-3 text-left">方法</th>
            <th class="px-6 py-3 text-left">路径</th>
            <th class="px-6 py-3 text-left">状态码</th>
            <th class="px-6 py-3 text-left">延迟</th>
            <th class="px-6 py-3 text-left">大小</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="log in store.items" :key="log.id" class="hover:bg-gray-50">
            <td class="px-6 py-3 text-gray-400 text-xs whitespace-nowrap">{{ new Date(log.created_at).toLocaleString() }}</td>
            <td class="px-6 py-3 text-xs text-gray-600">{{ log.app_id ? (apps.items.find(a => a.id === log.app_id)?.name ?? '—') : '—' }}</td>
            <td class="px-6 py-3 font-mono font-medium text-xs">{{ log.method }}</td>
            <td class="px-6 py-3 font-mono text-gray-600 text-xs truncate max-w-xs">{{ log.path }}</td>
            <td class="px-6 py-3">
              <span :class="log.status_code && log.status_code < 400 ? 'text-green-700 bg-green-50' : 'text-red-700 bg-red-50'" class="px-2 py-0.5 rounded text-xs font-medium">
                {{ log.status_code ?? '—' }}
              </span>
            </td>
            <td class="px-6 py-3 text-gray-500 text-xs">{{ log.latency_ms != null ? `${log.latency_ms}ms` : '—' }}</td>
            <td class="px-6 py-3 text-gray-400 text-xs">{{ log.request_size_bytes != null ? `${log.request_size_bytes}B` : '—' }}</td>
          </tr>
          <tr v-if="!store.items.length && !store.loading">
            <td colspan="7" class="px-6 py-8 text-center text-gray-400">暂无日志</td>
          </tr>
        </tbody>
      </table>

      <div class="px-6 py-4 border-t flex items-center justify-between">
        <p class="text-sm text-gray-400">共 {{ store.items.length }} 条</p>
        <button
          v-if="store.items.length >= store.limit"
          @click="loadMore"
          :disabled="store.loading"
          class="text-sm px-4 py-2 border rounded-lg hover:bg-gray-50 disabled:opacity-50"
        >
          {{ store.loading ? '加载中…' : '加载更多' }}
        </button>
      </div>
    </div>
  </div>
</template>
