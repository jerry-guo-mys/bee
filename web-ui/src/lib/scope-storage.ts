/** 管理 API 作用域（与后端 WebScopeParams 对齐），持久化到 localStorage */

export const ADMIN_SCOPE_CHANGED_EVENT = 'bee:admin-scope-changed'

export function dispatchAdminScopeChanged() {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent(ADMIN_SCOPE_CHANGED_EVENT))
  }
}

export const DEFAULT_ADMIN_SCOPE = {
  tenant_id: 'tenant-default',
  organization_id: 'org-default',
  user_id: 'user-default',
  team_id: '' as string,
} as const

const STORAGE_KEY = 'bee-admin-scope'

export type AdminScope = {
  tenant_id: string
  organization_id: string
  user_id: string
  team_id: string
}

export function loadAdminScope(): AdminScope {
  if (typeof localStorage === 'undefined') {
    return { ...DEFAULT_ADMIN_SCOPE, team_id: '' }
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return { ...DEFAULT_ADMIN_SCOPE, team_id: '' }
    const o = JSON.parse(raw) as Partial<AdminScope>
    return {
      tenant_id: (o.tenant_id || DEFAULT_ADMIN_SCOPE.tenant_id).trim(),
      organization_id: (o.organization_id || DEFAULT_ADMIN_SCOPE.organization_id).trim(),
      user_id: (o.user_id || DEFAULT_ADMIN_SCOPE.user_id).trim(),
      team_id: (o.team_id ?? '').trim(),
    }
  } catch {
    return { ...DEFAULT_ADMIN_SCOPE, team_id: '' }
  }
}

export function saveAdminScope(scope: AdminScope): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(scope))
}

/** 作为 URL 查询参数传给需 OrgAdmin 等的接口 */
export function scopeToQueryParams(scope: AdminScope): Record<string, string> {
  const q: Record<string, string> = {
    tenant_id: scope.tenant_id,
    organization_id: scope.organization_id,
    user_id: scope.user_id,
  }
  if (scope.team_id) q.team_id = scope.team_id
  return q
}
