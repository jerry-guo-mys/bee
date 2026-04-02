/** Types aligned with bee-web `/api/*` JSON responses */

export interface LlmMetricsJson {
  total_calls: number
  successful_calls: number
  failed_calls: number
  total_prompt_tokens: number
  total_completion_tokens: number
  total_latency_ms: number
  average_latency_ms: number
  error_rate: number
}

export interface ToolsMetricsJson {
  total_executions: number
  successful_executions: number
  failed_executions: number
  policy_rewrites: number
  policy_blocks: number
  average_execution_time_ms: number
}

export interface SessionMetricsJson {
  total_requests: number
  active_sessions: number
}

export interface BehaviorMetricsJson {
  total_errors: number
  tasks_completed_first_try: number
  tasks_total: number
  completion_rate: number
  error_rate: number
}

export interface MetricsResponse {
  llm: LlmMetricsJson
  tools: ToolsMetricsJson
  session: SessionMetricsJson
  behavior: BehaviorMetricsJson
}

export type TraceStatus = 'running' | 'success' | 'failure' | 'cancelled'

export interface TraceSummary {
  request_id: string
  session_id?: string | null
  status: TraceStatus
  duration_ms?: number | null
  span_count: number
  input_summary?: string | null
  llm_calls_count?: number | null
  tool_executions_count?: number | null
}

export interface TracesRecentResponse {
  traces: TraceSummary[]
}

export type TaskStatus = 'todo' | 'in_progress' | 'done'

export interface Task {
  id: string
  tenant_id?: string | null
  organization_id?: string | null
  team_id?: string | null
  title: string
  description?: string | null
  status: TaskStatus
  assignee_ids: string[]
  group_id?: string | null
  coordinator_id?: string | null
  workflow_template_id?: string | null
  workflow_run_id?: string | null
  workflow_template_version?: number | null
  internal_group?: boolean
  created_at: string
  updated_at: string
}

export interface TaskBoardColumn {
  status: string
  tasks: Task[]
}

export interface AssistantInfo {
  id: string
  name: string
  description: string
  skills?: string[] | null
}

export interface DynamicAgent {
  id: string
  role: string
  parent_id?: string | null
  guidance?: string | null
  created_at: string
}

export interface AuditLogRecord {
  id: string
  tenant_id: string
  organization_id?: string | null
  team_id?: string | null
  user_id?: string | null
  action: string
  resource_type: string
  resource_id: string
  detail_json?: string | null
  created_at: string
}

export interface CreateAgentBody {
  role: string
  guidance?: string
}

export interface CreateTaskBody {
  title: string
  description?: string
  assignee_ids?: string[]
}

export interface ToolInfo {
  id: string
  name: string
  description: string
}

export interface ToolAccessPolicy {
  id: string
  tenant_id: string
  organization_id?: string | null
  team_id?: string | null
  allowed_tool_ids: string[]
  denied_tool_ids: string[]
  created_at: string
  updated_at: string
}

export interface WorkflowTemplateSummary {
  id: string
  name: string
  description: string
  team_hint: string
  steps: string[]
  /** 0 = 内置；租户模板为已发布版本号 */
  version?: number
  source?: string
}

export interface StartWorkflowBody {
  template_id: string
  title: string
  description?: string
  tenant_id?: string
  organization_id?: string
  team_id?: string
}

export interface WorkflowRunResult {
  workflow_run_id: string
  workflow_template_id: string
  workflow_template_version?: number
  tasks: Task[]
}

/** GET /api/admin/workflow-templates */
export interface AdminWorkflowVersionSummary {
  version: number
  published_at?: string | null
  created_at: string
}

export interface AdminWorkflowTemplateDetail {
  id: string
  slug: string
  name: string
  description?: string | null
  status: string
  created_at: string
  updated_at: string
  versions: AdminWorkflowVersionSummary[]
}

export interface AdminWorkflowTemplatesListResponse {
  templates: AdminWorkflowTemplateDetail[]
}

export interface AdminWorkflowTemplateCreateResponse {
  id: string
  slug: string
}

export interface PutToolPolicyBody {
  tenant_id?: string
  organization_id?: string
  team_id?: string
  allowed_tool_ids: string[]
  denied_tool_ids: string[]
}

// ==================== SaaS Admin Types (Phase 1+2) ====================

export type TenantStatus = 'active' | 'suspended' | 'archived'

export interface Tenant {
  id: string
  name: string
  status: TenantStatus
  organization_count: number
  created_at: string
}

export interface TenantDetail extends Tenant {
  updated_at: string
  organizations: Organization[]
}

export interface TenantListResponse {
  tenants: Tenant[]
  total: number
}

export interface Organization {
  id: string
  tenant_id: string
  name: string
  slug: string
  member_count: number
  member_limit?: number | null
  created_at: string
}

export interface OrganizationListResponse {
  organizations: Organization[]
  total: number
}

export interface Team {
  id: string
  organization_id: string
  name: string
  code?: string | null
  description?: string | null
  parent_team_id?: string | null
  member_count: number
  created_at: string
}

export interface TeamListResponse {
  teams: Team[]
  total: number
}

export interface Member {
  id: string
  tenant_id: string
  organization_id: string
  team_id?: string | null
  user_id: string
  email?: string | null
  role: string
  status: string
  created_at: string
}

export interface MemberListResponse {
  members: Member[]
  total: number
}

// Request types
export interface CreateTenantBody {
  name: string
}

export interface CreateOrganizationBody {
  tenant_id: string
  name: string
  slug?: string
}

export interface CreateTeamBody {
  organization_id: string
  name: string
  code?: string
  description?: string
  parent_team_id?: string
}

export interface GetMembersParams {
  tenant_id?: string
  organization_id?: string
  role?: string
}

export interface AuditLogParams {
  tenant_id?: string
  organization_id?: string
  limit?: number
}
