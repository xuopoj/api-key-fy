const BASE = '/admin/api'

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method,
    credentials: 'include',
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error ?? res.statusText)
  }
  if (res.status === 204) return undefined as T
  return res.json()
}

const get = <T>(path: string) => request<T>('GET', path)
const post = <T>(path: string, body?: unknown) => request<T>('POST', path, body)
const put = <T>(path: string, body: unknown) => request<T>('PUT', path, body)
const del = <T>(path: string) => request<T>('DELETE', path)

// ── Types ─────────────────────────────────────────────────────────────────────

export interface User {
  id: string
  iam_username: string
  iam_domain: string
  display_name?: string
}

export interface Credential {
  id: string
  name: string
  is_shared: boolean
  iam_username: string
  iam_domain: string
  iam_project: string
  iam_region: string
  iam_endpoint?: string
  created_at: string
  updated_at: string
}

export interface App {
  id: string
  name: string
  slug?: string
  type: 'http_path' | 'http_port'
  credential_id: string
  base_url: string
  upstream_base_path: string
  strip_prefix: boolean
  listen_port?: number
  user_whitelist: string[]
  created_at: string
  updated_at: string
}

export interface ApiKey {
  id: string
  name: string
  key_prefix: string
  expires_at?: string
  rate_limit_rpm: number
  logging_enabled: boolean
  revoked_at?: string
  created_at: string
}

export interface CreateKeyResponse extends ApiKey {
  key: string // only returned once
}

export interface RequestLog {
  id: number
  api_key_id?: string
  app_id?: string
  method: string
  path: string
  status_code?: number
  latency_ms?: number
  request_size_bytes?: number
  response_size_bytes?: number
  created_at: string
}

// ── Auth ─────────────────────────────────────────────────────────────────────

export const authApi = {
  login: (body: { iam_username: string; iam_password: string; iam_domain: string }) =>
    post<User>('/auth/login', body),
  logout: () => post<void>('/auth/logout'),
  me: () => get<User>('/auth/me'),
}

// ── Credentials ───────────────────────────────────────────────────────────────

export interface CredentialPayload {
  name: string
  is_shared: boolean
  iam_username: string
  iam_password: string
  iam_domain: string
  iam_project: string
  iam_region: string
  iam_endpoint?: string
}

export const credentialsApi = {
  list: () => get<Credential[]>('/credentials'),
  create: (body: CredentialPayload) => post<Credential>('/credentials', body),
  update: (id: string, body: CredentialPayload) => put<Credential>(`/credentials/${id}`, body),
  delete: (id: string) => del<void>(`/credentials/${id}`),
}

// ── Apps ─────────────────────────────────────────────────────────────────────

export interface AppPayload {
  name: string
  slug?: string
  type: 'http_path' | 'http_port'
  credential_id: string
  base_url: string
  upstream_base_path?: string
  strip_prefix?: boolean
  listen_port?: number
  user_whitelist?: string[]
}

export const appsApi = {
  list: () => get<App[]>('/apps'),
  create: (body: AppPayload) => post<App>('/apps', body),
  update: (id: string, body: AppPayload) => put<App>(`/apps/${id}`, body),
  delete: (id: string) => del<void>(`/apps/${id}`),
  getKeys: (id: string) => get<ApiKey[]>(`/apps/${id}/keys`),
  setKeys: (id: string, key_ids: string[]) => put<void>(`/apps/${id}/keys`, key_ids),
}

// ── API Keys ─────────────────────────────────────────────────────────────────

export interface KeyPayload {
  name: string
  expires_at?: string
  rate_limit_rpm?: number
  logging_enabled?: boolean
  app_ids: string[]
}

export interface KeyStats {
  api_key_id: string
  total_requests: number
  last_used_at?: string
}

export const keysApi = {
  list: () => get<ApiKey[]>('/keys'),
  stats: () => get<KeyStats[]>('/keys/stats'),
  create: (body: KeyPayload) => post<CreateKeyResponse>('/keys', body),
  reveal: (id: string) => get<{ key: string }>(`/keys/${id}/reveal`),
  revoke: (id: string) => del<void>(`/keys/${id}`),
  setApps: (id: string, app_ids: string[]) => put<void>(`/keys/${id}/apps`, app_ids),
}

// ── Logs ─────────────────────────────────────────────────────────────────────

export const logsApi = {
  list: (params?: { api_key_id?: string; app_id?: string; limit?: number; offset?: number }) => {
    const q = new URLSearchParams()
    if (params?.api_key_id) q.set('api_key_id', params.api_key_id)
    if (params?.app_id) q.set('app_id', params.app_id)
    if (params?.limit) q.set('limit', String(params.limit))
    if (params?.offset) q.set('offset', String(params.offset))
    const qs = q.toString()
    return get<RequestLog[]>(`/logs${qs ? `?${qs}` : ''}`)
  },
}
