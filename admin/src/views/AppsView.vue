<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useAppsStore } from '@/stores/apps'
import { useCredentialsStore } from '@/stores/credentials'
import { useKeysStore } from '@/stores/keys'
import { appsApi } from '@/api'
import type { AppPayload, App } from '@/api'

const store = useAppsStore()
const creds = useCredentialsStore()
const keysStore = useKeysStore()
onMounted(() => Promise.all([store.fetch(), creds.fetch(), keysStore.fetch()]))

const showForm = ref(false)
const editing = ref<App | null>(null)
const error = ref<string | null>(null)

const blank = (): AppPayload => ({
  name: '', type: 'http_path', credential_id: '',
  base_url: '', upstream_base_path: '/', strip_prefix: true,
  slug: undefined, listen_port: undefined, user_whitelist: [],
})
const form = ref<AppPayload>(blank())
const whitelistInput = ref('')
const selectedKeyIds = ref<string[]>([])

const isHttpPath = computed(() => form.value.type === 'http_path')

function openCreate() {
  editing.value = null
  form.value = blank()
  whitelistInput.value = ''
  selectedKeyIds.value = []
  error.value = null
  showForm.value = true
}

async function openEdit(a: App) {
  editing.value = a
  form.value = {
    name: a.name, type: a.type, credential_id: a.credential_id,
    base_url: a.base_url, upstream_base_path: a.upstream_base_path,
    strip_prefix: a.strip_prefix, slug: a.slug ?? undefined,
    listen_port: a.listen_port ?? undefined,
    user_whitelist: [...a.user_whitelist],
  }
  whitelistInput.value = a.user_whitelist.join('\n')
  const linked = await appsApi.getKeys(a.id)
  selectedKeyIds.value = linked.map(k => k.id)
  error.value = null
  showForm.value = true
}

async function submit() {
  error.value = null
  form.value.user_whitelist = whitelistInput.value.split('\n').map(s => s.trim()).filter(Boolean)
  try {
    if (editing.value) {
      await store.update(editing.value.id, form.value)
      await appsApi.setKeys(editing.value.id, selectedKeyIds.value)
    } else {
      const app = await store.create(form.value)
      if (selectedKeyIds.value.length) {
        await appsApi.setKeys(app.id, selectedKeyIds.value)
      }
    }
    showForm.value = false
  } catch (e: any) {
    error.value = e.message
  }
}

async function remove(id: string) {
  if (!confirm('确认删除此应用？')) return
  try { await store.remove(id) } catch (e: any) { alert(e.message) }
}

// ── Key assignment modal ──────────────────────────────────────────────────────
const showKeys = ref(false)
const keysApp = ref<App | null>(null)
const linkedKeyIds = ref<string[]>([])

async function openKeys(a: App) {
  keysApp.value = a
  const linked = await appsApi.getKeys(a.id)
  linkedKeyIds.value = linked.map(k => k.id)
  showKeys.value = true
}

function toggleLinkedKey(id: string) {
  const idx = linkedKeyIds.value.indexOf(id)
  if (idx === -1) linkedKeyIds.value.push(id)
  else linkedKeyIds.value.splice(idx, 1)
}

async function saveKeys() {
  if (!keysApp.value) return
  try {
    await appsApi.setKeys(keysApp.value.id, linkedKeyIds.value)
    showKeys.value = false
  } catch (e: any) { alert(e.message) }
}
</script>

<template>
  <div class="p-8 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">应用管理</h1>
      <button @click="openCreate" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">+ 新建应用</button>
    </div>

    <div class="bg-white rounded-xl shadow-sm">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 text-xs uppercase">
          <tr>
            <th class="px-6 py-3 text-left">名称</th>
            <th class="px-6 py-3 text-left">类型</th>
            <th class="px-6 py-3 text-left">路由</th>
            <th class="px-6 py-3 text-left">后端地址</th>
            <th class="px-6 py-3"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="a in store.items" :key="a.id" class="hover:bg-gray-50">
            <td class="px-6 py-3 font-medium">{{ a.name }}</td>
            <td class="px-6 py-3">
              <span class="text-xs px-2 py-0.5 rounded font-medium" :class="a.type === 'http_path' ? 'bg-blue-50 text-blue-700' : 'bg-purple-50 text-purple-700'">
                {{ a.type === 'http_path' ? '路径' : '端口' }}
              </span>
            </td>
            <td class="px-6 py-3 font-mono text-gray-600 text-xs">
              {{ a.type === 'http_path' ? `/${a.slug}` : `:${a.listen_port}` }}
            </td>
            <td class="px-6 py-3 text-gray-600 text-xs truncate max-w-xs">
              {{ a.base_url }}{{ a.upstream_base_path !== '/' ? a.upstream_base_path : '' }}
            </td>
            <td class="px-6 py-3 text-right space-x-3">
              <button @click="openKeys(a)" class="text-purple-600 hover:underline text-xs">密钥</button>
              <button @click="openEdit(a)" class="text-blue-600 hover:underline text-xs">编辑</button>
              <button @click="remove(a.id)" class="text-red-600 hover:underline text-xs">删除</button>
            </td>
          </tr>
          <tr v-if="!store.items.length">
            <td colspan="5" class="px-6 py-8 text-center text-gray-400">暂无应用</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Keys modal -->
    <div v-if="showKeys" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-md p-6 space-y-4">
        <h2 class="text-lg font-semibold">关联密钥 — {{ keysApp?.name }}</h2>
        <div class="border rounded-lg divide-y max-h-60 overflow-y-auto">
          <label v-for="k in keysStore.items.filter(k => !k.revoked_at)" :key="k.id"
            class="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer">
            <input type="checkbox" :checked="linkedKeyIds.includes(k.id)" @change="toggleLinkedKey(k.id)" class="rounded" />
            <span class="text-sm font-medium">{{ k.name }}</span>
            <span class="text-xs text-gray-400 font-mono">{{ k.key_prefix }}…</span>
          </label>
          <div v-if="!keysStore.items.filter(k => !k.revoked_at).length" class="px-3 py-4 text-center text-sm text-gray-400">暂无有效密钥</div>
        </div>
        <div class="flex justify-end gap-3 pt-2">
          <button @click="showKeys = false" class="text-sm px-4 py-2 rounded-lg border hover:bg-gray-50">取消</button>
          <button @click="saveKeys" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">保存</button>
        </div>
      </div>
    </div>

    <!-- Modal -->
    <div v-if="showForm" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-lg p-6 space-y-4 max-h-screen overflow-y-auto">
        <h2 class="text-lg font-semibold">{{ editing ? '编辑' : '新建' }}应用</h2>
        <form @submit.prevent="submit" class="space-y-3">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="label">名称</label>
              <input v-model="form.name" required class="input" />
            </div>
            <div>
              <label class="label">类型</label>
              <select v-model="form.type" class="input">
                <option value="http_path">路径模式（共享端口，路径路由）</option>
                <option value="http_port">端口模式（独立端口）</option>
              </select>
            </div>
            <div>
              <label class="label">凭证</label>
              <select v-model="form.credential_id" required class="input">
                <option value="" disabled>选择凭证</option>
                <option v-for="c in creds.items" :key="c.id" :value="c.id">{{ c.name }}</option>
              </select>
            </div>
            <div v-if="isHttpPath">
              <label class="label">Slug <span class="text-gray-400 font-normal">（URL 路径前缀）</span></label>
              <input v-model="form.slug" required class="input" placeholder="my-model" />
              <p class="text-xs text-gray-400 mt-1">访问路径：/{{ form.slug || '…' }}/…</p>
            </div>
            <div v-else>
              <label class="label">监听端口</label>
              <input v-model.number="form.listen_port" type="number" required class="input" placeholder="8100" />
            </div>
            <div class="col-span-2">
              <label class="label">后端基础 URL</label>
              <input v-model="form.base_url" required class="input" placeholder="https://model.huaweicloud.com" />
            </div>
            <div class="col-span-2">
              <label class="label">
                上游基础路径
                <span class="text-gray-400 font-normal ml-1 text-xs">
                  — 拼接到每个转发路径前
                  （如 <code>/v1/infer</code> → 后端收到 <code>/v1/infer/your/path</code>）
                </span>
              </label>
              <input v-model="form.upstream_base_path" class="input" placeholder="/" />
            </div>
            <div v-if="isHttpPath" class="col-span-2 flex items-center gap-2">
              <input v-model="form.strip_prefix" type="checkbox" id="strip" class="rounded" />
              <label for="strip" class="text-sm">
                转发时去除 slug 前缀
                <span class="text-gray-400 text-xs ml-1">
                  {{ form.strip_prefix ? `（如 /${form.slug || 'slug'}/foo → /foo）` : `（如 /${form.slug || 'slug'}/foo → /${form.slug || 'slug'}/foo）` }}
                </span>
              </label>
            </div>
            <div v-if="!isHttpPath" class="col-span-2">
              <label class="label">用户白名单 <span class="text-gray-400 font-normal">（IAM 用户名，每行一个；留空表示所有用户）</span></label>
              <textarea v-model="whitelistInput" rows="3" class="input resize-none font-mono text-xs" placeholder="alice&#10;bob" />
            </div>
            <div class="col-span-2">
              <label class="label">关联密钥 <span class="text-gray-400 font-normal">（可选）</span></label>
              <div class="border rounded-lg divide-y max-h-36 overflow-y-auto">
                <label v-for="k in keysStore.items.filter(k => !k.revoked_at)" :key="k.id"
                  class="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer">
                  <input type="checkbox" :checked="selectedKeyIds.includes(k.id)"
                    @change="() => { const i = selectedKeyIds.indexOf(k.id); i === -1 ? selectedKeyIds.push(k.id) : selectedKeyIds.splice(i, 1) }"
                    class="rounded" />
                  <span class="text-sm">{{ k.name }}</span>
                  <span class="text-xs text-gray-400 font-mono">{{ k.key_prefix }}…</span>
                </label>
                <div v-if="!keysStore.items.filter(k => !k.revoked_at).length" class="px-3 py-4 text-center text-sm text-gray-400">暂无有效密钥</div>
              </div>
            </div>
          </div>
          <p v-if="error" class="text-sm text-red-600">{{ error }}</p>
          <div class="flex justify-end gap-3 pt-2">
            <button type="button" @click="showForm = false" class="text-sm px-4 py-2 rounded-lg border hover:bg-gray-50">取消</button>
            <button type="submit" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">保存</button>
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
