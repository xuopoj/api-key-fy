<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useCredentialsStore } from '@/stores/credentials'
import type { CredentialPayload, Credential } from '@/api'

const store = useCredentialsStore()
onMounted(() => store.fetch())

const showForm = ref(false)
const editing = ref<Credential | null>(null)
const error = ref<string | null>(null)

const blank = (): CredentialPayload => ({
  name: '', is_shared: false,
  iam_username: '', iam_password: '',
  iam_domain: '', iam_project: '', iam_region: '',
  iam_endpoint: undefined,
})

const form = ref<CredentialPayload>(blank())

function openCreate() {
  editing.value = null
  form.value = blank()
  error.value = null
  showForm.value = true
}

function openEdit(c: Credential) {
  editing.value = c
  form.value = { ...c, iam_password: '' }
  error.value = null
  showForm.value = true
}

async function submit() {
  error.value = null
  try {
    if (editing.value) {
      await store.update(editing.value.id, form.value)
    } else {
      await store.create(form.value)
    }
    showForm.value = false
  } catch (e: any) {
    error.value = e.message
  }
}

async function remove(id: string) {
  if (!confirm('确认删除此凭证？')) return
  try { await store.remove(id) } catch (e: any) { alert(e.message) }
}
</script>

<template>
  <div class="p-8 space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900">凭证管理</h1>
      <button @click="openCreate" class="bg-blue-600 hover:bg-blue-700 text-white text-sm px-4 py-2 rounded-lg">+ 新建凭证</button>
    </div>

    <!-- Table -->
    <div class="bg-white rounded-xl shadow-sm">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 text-gray-500 text-xs uppercase">
          <tr>
            <th class="px-6 py-3 text-left">名称</th>
            <th class="px-6 py-3 text-left">IAM 用户</th>
            <th class="px-6 py-3 text-left">账号 / 项目</th>
            <th class="px-6 py-3 text-left">区域</th>
            <th class="px-6 py-3 text-left">共享</th>
            <th class="px-6 py-3"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100">
          <tr v-for="c in store.items" :key="c.id" class="hover:bg-gray-50">
            <td class="px-6 py-3 font-medium">{{ c.name }}</td>
            <td class="px-6 py-3 text-gray-600">{{ c.iam_username }}</td>
            <td class="px-6 py-3 text-gray-600">{{ c.iam_domain }} / {{ c.iam_project }}</td>
            <td class="px-6 py-3 text-gray-600">{{ c.iam_region }}</td>
            <td class="px-6 py-3">
              <span v-if="c.is_shared" class="bg-green-50 text-green-700 text-xs px-2 py-0.5 rounded">共享</span>
              <span v-else class="text-gray-400 text-xs">私有</span>
            </td>
            <td class="px-6 py-3 text-right space-x-3">
              <button @click="openEdit(c)" class="text-blue-600 hover:underline text-xs">编辑</button>
              <button @click="remove(c.id)" class="text-red-600 hover:underline text-xs">删除</button>
            </td>
          </tr>
          <tr v-if="!store.items.length">
            <td colspan="6" class="px-6 py-8 text-center text-gray-400">暂无凭证</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Modal -->
    <div v-if="showForm" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl shadow-xl w-full max-w-lg p-6 space-y-4">
        <h2 class="text-lg font-semibold">{{ editing ? '编辑' : '新建' }}凭证</h2>
        <form @submit.prevent="submit" class="space-y-3">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="label">名称</label>
              <input v-model="form.name" required class="input" />
            </div>
            <div class="flex items-center gap-2 pt-6">
              <input v-model="form.is_shared" type="checkbox" id="shared" class="rounded" />
              <label for="shared" class="text-sm">共享（所有用户可见）</label>
            </div>
            <div>
              <label class="label">IAM 用户名</label>
              <input v-model="form.iam_username" required class="input" />
            </div>
            <div>
              <label class="label">密码{{ editing ? '（留空保持不变）' : '' }}</label>
              <input v-model="form.iam_password" type="password" :required="!editing" class="input" />
            </div>
            <div>
              <label class="label">账号名称（Domain）</label>
              <input v-model="form.iam_domain" required class="input" />
            </div>
            <div>
              <label class="label">项目（Project）</label>
              <input v-model="form.iam_project" required class="input" placeholder="ap-southeast-1" />
            </div>
            <div>
              <label class="label">区域（Region）</label>
              <input v-model="form.iam_region" required class="input" placeholder="ap-southeast-1" />
            </div>
            <div>
              <label class="label">自定义 IAM 端点 <span class="text-gray-400">（可选）</span></label>
              <input v-model="form.iam_endpoint" class="input" placeholder="https://iam.internal.company.com" />
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
