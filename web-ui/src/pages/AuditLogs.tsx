import { useCallback, useEffect, useState } from 'react';
import { FileText, Search, Filter, ExternalLink } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import {
  getAuditLogsEnhanced,
  getTenants,
  getOrganizations,
  type AuditLogRecord,
  type Tenant,
  type Organization,
} from '@/lib/api';

const actionLabels: Record<string, string> = {
  create: '创建',
  update: '更新',
  delete: '删除',
  suspend: '暂停',
  restore: '恢复',
  archive: '归档',
  invite: '邀请',
  join: '加入',
  leave: '离开',
  remove: '移除',
};

const resourceTypeLabels: Record<string, string> = {
  tenant: '租户',
  organization: '组织',
  team: '团队',
  member: '成员',
  user: '用户',
  tool_policy: '工具策略',
  workflow: '工作流',
  task: '任务',
};

export default function AuditLogsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<AuditLogRecord[]>([]);
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [tenantFilter, setTenantFilter] = useState<string>('all');
  const [orgFilter, setOrgFilter] = useState<string>('all');
  const [limitFilter, setLimitFilter] = useState<number>(50);
  const [expandedLogId, setExpandedLogId] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [logsData, tenantsData, orgsData] = await Promise.all([
        getAuditLogsEnhanced({
          tenant_id: tenantFilter === 'all' ? undefined : tenantFilter,
          organization_id: orgFilter === 'all' ? undefined : orgFilter,
          limit: limitFilter,
        }),
        getTenants(),
        getOrganizations(),
      ]);
      setLogs(logsData || []);
      setTenants(tenantsData.tenants ?? []);
      setOrganizations(orgsData.organizations ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [tenantFilter, orgFilter, limitFilter]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const getOrgName = (orgId: string) => {
    if (!orgId) return '-';
    return organizations.find((o) => o.id === orgId)?.name || 'Unknown';
  };

  const getActionLabel = (action: string) => {
    return actionLabels[action] || action;
  };

  const getResourceTypeLabel = (resourceType: string) => {
    return resourceTypeLabels[resourceType] || resourceType;
  };

  const toggleExpand = (logId: string) => {
    setExpandedLogId(expandedLogId === logId ? null : logId);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">审计日志</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            追踪和查看系统内的所有操作记录，包括创建、更新、删除等
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2">
            <Search className="w-4 h-4 text-surface-400" />
            <span className="text-sm text-surface-500">共 {logs.length} 条记录</span>
          </div>
        </div>
      </div>

      {/* 筛选器 */}
      <div className="flex flex-wrap gap-2 items-center">
        <div className="flex items-center gap-2">
          <Filter className="w-4 h-4 text-surface-400" />
          <span className="text-sm text-surface-500">筛选:</span>
        </div>

        <select
          value={tenantFilter}
          onChange={(e) => {
            setTenantFilter(e.target.value);
            setOrgFilter('all');
          }}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
        >
          <option value="all">全部租户</option>
          {tenants.map((tenant) => (
            <option key={tenant.id} value={tenant.id}>
              {tenant.name}
            </option>
          ))}
        </select>

        <select
          value={orgFilter}
          onChange={(e) => setOrgFilter(e.target.value)}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
          disabled={tenantFilter === 'all'}
        >
          <option value="all">全部组织</option>
          {organizations
            .filter((org) => tenantFilter === 'all' || org.tenant_id === tenantFilter)
            .map((org) => (
              <option key={org.id} value={org.id}>
                {org.name}
              </option>
            ))}
        </select>

        <select
          value={limitFilter}
          onChange={(e) => setLimitFilter(Number(e.target.value))}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
        >
          <option value={20}>最近 20 条</option>
          <option value={50}>最近 50 条</option>
          <option value={100}>最近 100 条</option>
          <option value={500}>最近 500 条</option>
        </select>
      </div>

      {/* 错误提示 */}
      {error && (
        <Card className="border-error-200 dark:border-error-800 bg-error-50 dark:bg-error-900/20">
          <CardContent className="py-4">
            <p className="text-error-600 dark:text-error-400">{error}</p>
          </CardContent>
        </Card>
      )}

      {/* 日志列表 */}
      {loading ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            加载中...
          </CardContent>
        </Card>
      ) : logs.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            <FileText className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>暂无审计日志数据</p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead className="bg-surface-50 dark:bg-surface-800 border-b border-surface-200 dark:border-surface-700">
                  <tr>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      操作
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      资源类型
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      资源 ID
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      用户
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      组织
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      时间
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      详情
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-surface-200 dark:divide-surface-700">
                  {logs.map((log) => (
                    <>
                      <tr
                        key={log.id}
                        className="hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors cursor-pointer"
                        onClick={() => toggleExpand(log.id)}
                      >
                        <td className="px-4 py-3 whitespace-nowrap">
                          <Badge variant="info">
                            {getActionLabel(log.action)}
                          </Badge>
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-600 dark:text-surface-400">
                          {getResourceTypeLabel(log.resource_type)}
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap text-sm font-mono text-surface-500">
                          {log.resource_id.slice(0, 12)}...
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-600 dark:text-surface-400">
                          {log.user_id ? log.user_id.slice(0, 12) + '...' : 'System'}
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-600 dark:text-surface-400">
                          {getOrgName(log.organization_id ?? '')}
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-500">
                          {new Date(log.created_at).toLocaleString('zh-CN')}
                        </td>
                        <td className="px-4 py-3 whitespace-nowrap">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation();
                              toggleExpand(log.id);
                            }}
                          >
                            <ExternalLink className="w-4 h-4" />
                          </Button>
                        </td>
                      </tr>
                      {expandedLogId === log.id && log.detail_json && (
                        <tr className="bg-surface-50 dark:bg-surface-800/30">
                          <td colSpan={7} className="px-4 py-3">
                            <div className="text-xs font-mono bg-surface-900 dark:bg-surface-950 text-surface-100 p-3 rounded-lg overflow-x-auto">
                              <pre>{JSON.stringify(JSON.parse(log.detail_json), null, 2)}</pre>
                            </div>
                          </td>
                        </tr>
                      )}
                    </>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
