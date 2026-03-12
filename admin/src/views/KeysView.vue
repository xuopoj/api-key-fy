<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useKeysStore } from '@/stores/keys'
import { useAppsStore } from '@/stores/apps'
import { keysApi } from '@/api'
import type { KeyPayload } from '@/api'

const store = useKeysStore()
const apps = useAppsStore()
onMounted(() => Promise.all([store.fetch(), apps.fetch()]))

const showForm = ref(false)
const showSecret = ref(false)
const copied = ref(false)
const error = ref<string | null>(null)

const blank = (): KeyPayload => ({
  name: '',
  expires_at: undefined, rate_limit_rpm: 60,
  logging_enabled: true, app_ids: [],
})
const form = ref<KeyPayload>(blank())

function openCreate() {
  form.value = blank()
  error.value = null
  showForm.value = true
}

async function submit() {
  error.value = null
  try {
    await store.create(form.value)
    showForm.value = false
    showSecret.value = true
  } catch (e: any) {
    error.value = e.message
  }
}

async function remove(id: string) {
  if (!confirm('确认删除此 API 密钥？此操作不可撤回。')) return
  try { await store.remove(id) } catch (e: any) { alert(e.message) }
}

async function copyKey() {
  if (store.newKeySecret) {
    await navigator.clipboard.writeText(store.newKeySecret)
    copied.value = true
    setTimeout(() => copied.value = false, 2000)
  }
}

function dismissSecret() {
  store.clearNewKeySecret()
  showSecret.value = false
  copied.value = false
}

function toggleApp(id: string) {
  const idx = form.value.app_ids.indexOf(id)
  if (idx === -1) form.value.app_ids.push(id)
  else form.value.app_ids.splice(idx, 1)
}

const copyingId = ref<string | null>(null)
async function copyExistingKey(id: string) {
  copyingId.value = id
  try {
    const { key } = await keysApi.reveal(id)
    await navigator.clipboard.writeText(key)
    setTimeout(() => { if (copyingId.value === id) copyingId.value = null }, 2000)
  } catch {
    copyingId.value = null
    alert('复制失败')
  }
}
</script>

<template>
  <div class="p-8 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">API 密钥</h1>
      <button @click="openCreate" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">+ 新建密钥</button>
    </div>

    <!-- New key secret reveal -->
    <div v-if="showSecret && store.newKeySecret" class="bg-green-50 border border-green-200 rounded-xl p-5 space-y-3">
      <p class="text-sm font-medium text-green-800">密钥已创建。</p>
      <div class="flex items-center gap-3">
        <code class="flex-1 bg-white border rounded-lg px-3 py-2 text-sm font-mono break-all">{{ store.newKeySecret }}</code>
        <button @click="copyKey" class="text-sm px-3 py-2 rounded-lg text-white transition-colors" :class="copied ? 'bg-green-800' : 'bg-green-600 hover:bg-green-700'">{{ copied ? '已复制！' : '复制' }}</button>
      </div>
      <button @click="dismissSecret" class="text-xs text-green-700 hover:underline">关闭</button>
    </div>

    <div class="bg-white rounded-xl shadow-sm">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 text-xs uppercase">
          <tr>
            <th class="px-6 py-3 text-left">名称</th>
            <th class="px-6 py-3 text-left">前缀</th>
            <th class="px-6 py-3 text-left">请求次数</th>
            <th class="px-6 py-3 text-left">最后使用</th>
            <th class="px-6 py-3 text-left">速率限制</th>
            <th class="px-6 py-3 text-left">过期时间</th>
            <th class="px-6 py-3 text-left">状态</th>
            <th class="px-6 py-3"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="k in store.items" :key="k.id" class="hover:bg-gray-50">
            <td class="px-6 py-3 font-medium">{{ k.name }}</td>
            <td class="px-6 py-3 font-mono text-xs text-gray-600">{{ k.key_prefix }}…</td>
            <td class="px-6 py-3 text-gray-600 text-xs">{{ store.stats[k.id]?.total_requests ?? 0 }}</td>
            <td class="px-6 py-3 text-gray-400 text-xs">{{ store.stats[k.id]?.last_used_at ? new Date(store.stats[k.id].last_used_at!).toLocaleString() : '—' }}</td>
            <td class="px-6 py-3 text-gray-600">{{ k.rate_limit_rpm }} rpm</td>
            <td class="px-6 py-3 text-gray-600 text-xs">{{ k.expires_at ? new Date(k.expires_at).toLocaleDateString() : '永不过期' }}</td>
            <td class="px-6 py-3">
              <span class="bg-green-50 text-green-700 text-xs px-2 py-0.5 rounded">有效</span>
            </td>
            <td class="px-6 py-3 text-right space-x-3">
              <button @click="copyExistingKey(k.id)" class="text-gray-500 hover:text-gray-800 text-xs">{{ copyingId === k.id ? '已复制！' : '复制密钥' }}</button>
              <button @click="remove(k.id)" class="text-red-600 hover:underline text-xs">删除</button>
            </td>
          </tr>
          <tr v-if="!store.items.length">
            <td colspan="8" class="px-6 py-8 text-center text-gray-400">暂无 API 密钥</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create modal -->
    <div v-if="showForm" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-lg p-6 space-y-4 max-h-screen overflow-y-auto">
        <h2 class="text-lg font-semibold">新建 API 密钥</h2>
        <form @submit.prevent="submit" class="space-y-3">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="label">名称</label>
              <input v-model="form.name" required class="input" />
            </div>
            <div>
              <label class="label">速率限制（次/分钟）</label>
              <input v-model.number="form.rate_limit_rpm" type="number" min="1" class="input" />
            </div>
            <div>
              <label class="label">过期时间 <span class="text-gray-400 font-normal">（可选）</span></label>
              <input v-model="form.expires_at" type="datetime-local" class="input" />
            </div>
            <div class="col-span-2 flex items-center gap-2">
              <input v-model="form.logging_enabled" type="checkbox" id="logging" class="rounded" />
              <label for="logging" class="text-sm">启用请求日志</label>
            </div>
          </div>

          <div>
            <label class="label">允许访问的应用</label>
            <div class="border rounded-lg divide-y max-h-40 overflow-y-auto">
              <label v-for="a in apps.items" :key="a.id" class="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer">
                <input type="checkbox" :checked="form.app_ids.includes(a.id)" @change="toggleApp(a.id)" class="rounded" />
                <span class="text-sm">{{ a.name }}</span>
                <span class="text-xs text-gray-400">{{ a.type === 'http_path' ? `/${a.slug}` : `:${a.listen_port}` }}</span>
              </label>
              <div v-if="!apps.items.length" class="px-3 py-4 text-center text-sm text-gray-400">暂无应用，请先创建</div>
            </div>
          </div>

          <p v-if="error" class="text-sm text-red-600">{{ error }}</p>
          <div class="flex justify-end gap-3 pt-2">
            <button type="button" @click="showForm = false" class="text-sm px-4 py-2 rounded-lg border hover:bg-gray-50">取消</button>
            <button type="submit" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">创建密钥</button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference "@/assets/main.css";
.label { @apply block text-sm font-medium text-gray-700 mb-1; }
.input { @apply w-full border rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500; }
</style>
