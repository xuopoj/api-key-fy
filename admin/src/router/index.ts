import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('@/views/LoginView.vue'),
      meta: { public: true },
    },
    {
      path: '/',
      component: () => import('@/views/LayoutView.vue'),
      children: [
        { path: '', name: 'dashboard', component: () => import('@/views/DashboardView.vue') },
        { path: 'credentials', name: 'credentials', component: () => import('@/views/CredentialsView.vue') },
        { path: 'apps', name: 'apps', component: () => import('@/views/AppsView.vue') },
        { path: 'keys', name: 'keys', component: () => import('@/views/KeysView.vue') },
        { path: 'logs', name: 'logs', component: () => import('@/views/LogsView.vue') },
      ],
    },
  ],
})

router.beforeEach(async (to) => {
  const auth = useAuthStore()
  if (!auth.user) await auth.fetchMe()
  if (!to.meta.public && !auth.user) return '/login'
})

export default router
