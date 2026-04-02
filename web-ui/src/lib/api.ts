/**
 * bee-web / bee-admin HTTP client. Dev server proxies `/api` → backend (see vite.config.ts).
 */
import type {
  AssistantInfo,
  AuditLogRecord,
  CreateAgentBody,
  CreateTaskBody,
  DynamicAgent,
  MetricsResponse,
  PutToolPolicyBody,
  StartWorkflowBody,
  Task,
  TaskStatus,
  ToolAccessPolicy,
  ToolInfo,
  TracesRecentResponse,
  WorkflowRunResult,
  WorkflowTemplateSummary,
  // SaaS Admin types
  Tenant,
  TenantDetail,
  TenantListResponse,
  CreateTenantBody,
  Organization,
  OrganizationListResponse,
  CreateOrganizationBody,
  Team,
  TeamListResponse,
  CreateTeamBody,
  Member,
  MemberListResponse,
  GetMembersParams,
  AuditLogParams,
  TenantStatus,
} from './api-types'

// Re-export types for convenience
export type {
  Tenant,
  TenantDetail,
  TenantListResponse,
  CreateTenantBody,
  Organization,
  OrganizationListResponse,
  CreateOrganizationBody,
  Team,
  TeamListResponse,
  CreateTeamBody,
  Member,
  MemberListResponse,
  GetMembersParams,
  AuditLogParams,
  TenantStatus,
  AuditLogRecord,
}
import { loadAdminScope, scopeToQueryParams, type AdminScope } from './scope-storage'

const PREFIX = import.meta.env.BASE_URL === '/' ? '' : import.meta.env.BASE_URL.replace(/\/$/, '')

function buildUrl(path: string, params?: Record<string, string | number | undefined>): string {
  const url = `${PREFIX}${path.startsWith('/') ? path : `/${path}`}`
  if (!params || Object.keys(params).length === 0) return url
  const q = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== '') q.set(k, String(v))
  }
  const qs = q.toString()
  return qs ? `${url}?${qs}` : url
}

/** 当前管理作用域（设置页可改，存 localStorage） */
export function managementScopeQuery(): Record<string, string> {
  return scopeToQueryParams(loadAdminScope())
}

async function handleResponse<T>(res: Response): Promise<T> {
  const text = await res.text()
  if (!res.ok) {
    throw new Error(text || `${res.status} ${res.statusText}`)
  }
  if (!text) return {} as T
  try {
    return JSON.parse(text) as T
  } catch {
    throw new Error(`Expected JSON, got: ${text.slice(0, 120)}`)
  }
}

export async function getMetrics(): Promise<MetricsResponse> {
  const res = await fetch(buildUrl('/api/metrics'))
  return handleResponse<MetricsResponse>(res)
}

export async function getTracesRecent(limit = 20): Promise<TracesRecentResponse> {
  const res = await fetch(buildUrl('/api/traces/recent', { limit }))
  return handleResponse<TracesRecentResponse>(res)
}

export async function getTasks(params?: { status?: TaskStatus }): Promise<Task[]> {
  const q: Record<string, string> = { ...managementScopeQuery() }
  if (params?.status) q.status = params.status
  const res = await fetch(buildUrl('/api/tasks', q))
  return handleResponse<Task[]>(res)
}

export async function getAssistants(): Promise<AssistantInfo[]> {
  const res = await fetch(buildUrl('/api/assistants'))
  return handleResponse<AssistantInfo[]>(res)
}

export async function getDynamicAgents(): Promise<DynamicAgent[]> {
  const res = await fetch(buildUrl('/api/agents'))
  return handleResponse<DynamicAgent[]>(res)
}

export async function createAgent(body: CreateAgentBody): Promise<DynamicAgent> {
  const res = await fetch(buildUrl('/api/agents'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return handleResponse<DynamicAgent>(res)
}

export async function createTask(body: CreateTaskBody): Promise<Task> {
  const s = loadAdminScope()
  const payload: Record<string, unknown> = {
    title: body.title,
    tenant_id: s.tenant_id,
    organization_id: s.organization_id,
  }
  if (body.description) payload.description = body.description
  if (body.assignee_ids?.length) payload.assignee_ids = body.assignee_ids
  if (s.team_id) payload.team_id = s.team_id

  const res = await fetch(buildUrl('/api/tasks'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return handleResponse<Task>(res)
}

export async function getAuditLogs(limit = 50): Promise<AuditLogRecord[]> {
  const res = await fetch(
    buildUrl('/api/audit-logs', {
      ...managementScopeQuery(),
      limit,
    })
  )
  return handleResponse<AuditLogRecord[]>(res)
}

export async function getTools(): Promise<ToolInfo[]> {
  const res = await fetch(buildUrl('/api/tools'))
  return handleResponse<ToolInfo[]>(res)
}

export async function getToolPolicies(): Promise<ToolAccessPolicy[]> {
  const res = await fetch(buildUrl('/api/tool-policies', managementScopeQuery()))
  return handleResponse<ToolAccessPolicy[]>(res)
}

function scopeBody(s: AdminScope): Record<string, string> {
  const o: Record<string, string> = {
    tenant_id: s.tenant_id,
    organization_id: s.organization_id,
    user_id: s.user_id,
  }
  if (s.team_id) o.team_id = s.team_id
  return o
}

export async function putToolPolicy(body: PutToolPolicyBody): Promise<ToolAccessPolicy> {
  const s = loadAdminScope()
  const payload: Record<string, unknown> = {
    ...scopeBody(s),
    tenant_id: body.tenant_id ?? s.tenant_id,
    organization_id: body.organization_id ?? s.organization_id,
    allowed_tool_ids: body.allowed_tool_ids,
    denied_tool_ids: body.denied_tool_ids,
  }
  const team = body.team_id !== undefined ? body.team_id : s.team_id
  if (team) payload.team_id = team
  else delete payload.team_id

  const res = await fetch(buildUrl('/api/tool-policies'), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return handleResponse<ToolAccessPolicy>(res)
}

export async function getWorkflowTemplates(): Promise<WorkflowTemplateSummary[]> {
  const res = await fetch(buildUrl('/api/workflow-templates', managementScopeQuery()))
  return handleResponse<WorkflowTemplateSummary[]>(res)
}

export async function startWorkflow(body: StartWorkflowBody): Promise<WorkflowRunResult> {
  const s = loadAdminScope()
  const payload: Record<string, unknown> = {
    ...scopeBody(s),
    template_id: body.template_id,
    title: body.title,
    tenant_id: body.tenant_id ?? s.tenant_id,
    organization_id: body.organization_id ?? s.organization_id,
  }
  if (body.description) payload.description = body.description
  const team = body.team_id !== undefined ? body.team_id : s.team_id
  if (team) payload.team_id = team

  const res = await fetch(buildUrl('/api/workflows/start'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return handleResponse<WorkflowRunResult>(res)
}

// ==================== SaaS Admin APIs (Phase 1+2) ====================

// Tenant APIs
export async function getTenants(params?: { status?: string }): Promise<TenantListResponse> {
  const q: Record<string, string> = {}
  if (params?.status) q.status = params.status
  const res = await fetch(buildUrl('/api/admin/tenants', q))
  return handleResponse<TenantListResponse>(res)
}

export async function getTenantDetail(id: string): Promise<TenantDetail> {
  const res = await fetch(buildUrl(`/api/admin/tenants/${id}`))
  return handleResponse<TenantDetail>(res)
}

export async function createTenant(body: CreateTenantBody): Promise<Tenant> {
  const res = await fetch(buildUrl('/api/admin/tenants'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return handleResponse<Tenant>(res)
}

export async function suspendTenant(id: string): Promise<void> {
  const res = await fetch(buildUrl(`/api/admin/tenants/${id}/suspend`), {
    method: 'POST',
  })
  if (!res.ok) throw new Error(await res.text())
}

export async function restoreTenant(id: string): Promise<void> {
  const res = await fetch(buildUrl(`/api/admin/tenants/${id}/restore`), {
    method: 'POST',
  })
  if (!res.ok) throw new Error(await res.text())
}

export async function archiveTenant(id: string): Promise<void> {
  const res = await fetch(buildUrl(`/api/admin/tenants/${id}/archive`), {
    method: 'POST',
  })
  if (!res.ok) throw new Error(await res.text())
}

// Organization APIs
export async function getOrganizations(params?: { tenant_id?: string }): Promise<OrganizationListResponse> {
  const q: Record<string, string> = {}
  if (params?.tenant_id) q.tenant_id = params.tenant_id
  const res = await fetch(buildUrl('/api/admin/organizations', q))
  return handleResponse<OrganizationListResponse>(res)
}

export async function createOrganization(body: CreateOrganizationBody): Promise<Organization> {
  const res = await fetch(buildUrl('/api/admin/organizations'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return handleResponse<Organization>(res)
}

// Team APIs
export async function getTeams(params?: { organization_id?: string }): Promise<TeamListResponse> {
  const q: Record<string, string> = {}
  if (params?.organization_id) q.organization_id = params.organization_id
  const res = await fetch(buildUrl('/api/admin/teams', q))
  return handleResponse<TeamListResponse>(res)
}

export async function createTeam(body: CreateTeamBody): Promise<Team> {
  const res = await fetch(buildUrl('/api/admin/teams'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  return handleResponse<Team>(res)
}

// Member APIs
export async function getMembers(params?: GetMembersParams): Promise<MemberListResponse> {
  const q: Record<string, string> = { ...managementScopeQuery() }
  if (params?.tenant_id) q.tenant_id = params.tenant_id
  if (params?.organization_id) q.organization_id = params.organization_id
  if (params?.role) q.role = params.role
  const res = await fetch(buildUrl('/api/admin/members', q))
  return handleResponse<MemberListResponse>(res)
}

// Audit Log APIs
export async function getAuditLogsEnhanced(params?: AuditLogParams): Promise<AuditLogRecord[]> {
  const q: Record<string, string> = { ...managementScopeQuery() }
  if (params?.tenant_id) q.tenant_id = params.tenant_id
  if (params?.organization_id) q.organization_id = params.organization_id
  if (params?.limit) q.limit = String(params.limit)
  const res = await fetch(buildUrl('/api/admin/audit-logs', q))
  return handleResponse<AuditLogRecord[]>(res)
}
