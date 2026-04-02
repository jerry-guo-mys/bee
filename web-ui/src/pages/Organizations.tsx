import { useCallback, useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Building, Plus, Users } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import {
  getOrganizations,
  getTenants,
  createOrganization,
  type Organization,
  type Tenant,
} from '@/lib/api';

export default function OrganizationsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [tenantFilter, setTenantFilter] = useState<string>('all');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newOrgName, setNewOrgName] = useState('');
  const [newOrgSlug, setNewOrgSlug] = useState('');
  const [newOrgTenantId, setNewOrgTenantId] = useState('');
  const [creating, setCreating] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [orgsData, tenantsData] = await Promise.all([
        getOrganizations(),
        getTenants(),
      ]);
      setOrganizations(orgsData.organizations ?? []);
      setTenants(tenantsData.tenants ?? []);
      if (tenantsData.tenants?.length > 0 && !newOrgTenantId) {
        setNewOrgTenantId(tenantsData.tenants[0].id);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [newOrgTenantId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newOrgName.trim() || !newOrgTenantId) return;
    try {
      setCreating(true);
      await createOrganization({
        tenant_id: newOrgTenantId,
        name: newOrgName.trim(),
        slug: newOrgSlug.trim() || undefined,
      });
      setNewOrgName('');
      setNewOrgSlug('');
      setShowCreateDialog(false);
      await loadData();
    } catch (err) {
      alert(`创建失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const filteredOrgs = organizations.filter((org) =>
    tenantFilter === 'all' ? true : org.tenant_id === tenantFilter
  );

  const getTenantName = (tenantId: string) => {
    return tenants.find((t) => t.id === tenantId)?.name || 'Unknown';
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">组织管理</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            管理租户下的组织，包括创建、查看和统计信息
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          创建组织
        </Button>
      </div>

      {/* 筛选器 */}
      <div className="flex gap-2 items-center">
        <span className="text-sm text-surface-500">按租户筛选:</span>
        <select
          value={tenantFilter}
          onChange={(e) => setTenantFilter(e.target.value)}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
        >
          <option value="all">全部租户</option>
          {tenants.map((tenant) => (
            <option key={tenant.id} value={tenant.id}>
              {tenant.name}
            </option>
          ))}
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

      {/* 组织列表 */}
      {loading ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            加载中...
          </CardContent>
        </Card>
      ) : filteredOrgs.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            <Building className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>暂无组织数据</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredOrgs.map((org) => (
            <motion.div
              key={org.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
            >
              <Card>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                        <Building className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                      </div>
                      <div>
                        <CardTitle className="text-lg">{org.name}</CardTitle>
                        <CardDescription className="text-xs font-mono">
                          {org.slug || '无 slug'}
                        </CardDescription>
                      </div>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-surface-500">所属租户</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {getTenantName(org.tenant_id)}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-surface-500">成员数量</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        <Users className="w-4 h-4 inline mr-1" />
                        {org.member_count}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-surface-500">创建时间</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {new Date(org.created_at).toLocaleDateString('zh-CN')}
                      </span>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      )}

      {/* 创建组织对话框 */}
      {showCreateDialog && (
        <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            className="bg-white dark:bg-surface-800 rounded-xl shadow-xl max-w-md w-full"
          >
            <Card className="border-0 shadow-none">
              <CardHeader>
                <CardTitle>创建组织</CardTitle>
                <CardDescription>
                  在指定租户下创建一个新的组织
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreate} className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      选择租户
                    </label>
                    <select
                      value={newOrgTenantId}
                      onChange={(e) => setNewOrgTenantId(e.target.value)}
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      required
                    >
                      {tenants.map((tenant) => (
                        <option key={tenant.id} value={tenant.id}>
                          {tenant.name}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      组织名称
                    </label>
                    <input
                      type="text"
                      value={newOrgName}
                      onChange={(e) => setNewOrgName(e.target.value)}
                      placeholder="例如：Engineering Dept"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      autoFocus
                      required
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      组织 Slug (可选)
                    </label>
                    <input
                      type="text"
                      value={newOrgSlug}
                      onChange={(e) => setNewOrgSlug(e.target.value)}
                      placeholder="例如：engineering"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                  </div>
                  <div className="flex gap-3 pt-4">
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => setShowCreateDialog(false)}
                      className="flex-1"
                    >
                      取消
                    </Button>
                    <Button type="submit" disabled={creating} className="flex-1">
                      {creating ? '创建中...' : '创建'}
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          </motion.div>
        </div>
      )}
    </div>
  );
}
