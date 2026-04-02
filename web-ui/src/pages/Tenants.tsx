import { useCallback, useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Building2, Plus, PauseCircle, Archive, RefreshCw } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import {
  getTenants,
  createTenant,
  suspendTenant,
  restoreTenant,
  archiveTenant,
  type Tenant,
  type TenantStatus,
} from '@/lib/api';

export default function TenantsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newTenantName, setNewTenantName] = useState('');
  const [creating, setCreating] = useState(false);

  const loadTenants = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getTenants();
      setTenants(data.tenants ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadTenants();
  }, [loadTenants]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTenantName.trim()) return;
    try {
      setCreating(true);
      await createTenant({ name: newTenantName.trim() });
      setNewTenantName('');
      setShowCreateDialog(false);
      await loadTenants();
    } catch (err) {
      alert(`创建失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const handleSuspend = async (id: string) => {
    if (!confirm('确定要暂停此租户吗？')) return;
    try {
      await suspendTenant(id);
      await loadTenants();
    } catch (err) {
      alert(`操作失败：${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handleRestore = async (id: string) => {
    try {
      await restoreTenant(id);
      await loadTenants();
    } catch (err) {
      alert(`操作失败：${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handleArchive = async (id: string) => {
    if (!confirm('确定要归档此租户吗？此操作不可逆。')) return;
    try {
      await archiveTenant(id);
      await loadTenants();
    } catch (err) {
      alert(`操作失败：${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const filteredTenants = tenants.filter((t) =>
    statusFilter === 'all' ? true : t.status === statusFilter
  );

  const statusLabels: Record<TenantStatus, string> = {
    active: '活跃',
    suspended: '暂停',
    archived: '归档',
  };

  const statusVariants: Record<TenantStatus, 'success' | 'warning' | 'error' | 'info'> = {
    active: 'success',
    suspended: 'warning',
    archived: 'info',
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">租户管理</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            管理多租户系统的租户，包括创建、暂停、恢复和归档操作
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          创建租户
        </Button>
      </div>

      {/* 筛选器 */}
      <div className="flex gap-2">
        {['all', 'active', 'suspended', 'archived'].map((status) => (
          <Button
            key={status}
            variant={statusFilter === status ? 'primary' : 'outline'}
            size="sm"
            onClick={() => setStatusFilter(status)}
          >
            {status === 'all' ? '全部' : statusLabels[status as TenantStatus]}
          </Button>
        ))}
      </div>

      {/* 错误提示 */}
      {error && (
        <Card className="border-error-200 dark:border-error-800 bg-error-50 dark:bg-error-900/20">
          <CardContent className="py-4">
            <p className="text-error-600 dark:text-error-400">{error}</p>
          </CardContent>
        </Card>
      )}

      {/* 租户列表 */}
      {loading ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            加载中...
          </CardContent>
        </Card>
      ) : filteredTenants.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            <Building2 className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>暂无租户数据</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredTenants.map((tenant) => (
            <motion.div
              key={tenant.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
            >
              <Card>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                        <Building2 className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                      </div>
                      <div>
                        <CardTitle className="text-lg">{tenant.name}</CardTitle>
                        <CardDescription className="text-xs font-mono">
                          {tenant.id.slice(0, 12)}...
                        </CardDescription>
                      </div>
                    </div>
                    <Badge variant={statusVariants[tenant.status]}>
                      {statusLabels[tenant.status]}
                    </Badge>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-surface-500">组织数量</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {tenant.organization_count}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-surface-500">创建时间</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {new Date(tenant.created_at).toLocaleDateString('zh-CN')}
                      </span>
                    </div>
                  </div>

                  {/* 操作按钮 */}
                  <div className="flex gap-2 mt-4 pt-4 border-t border-surface-200 dark:border-surface-700">
                    {tenant.status === 'active' && (
                      <>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleSuspend(tenant.id)}
                          className="flex-1"
                        >
                          <PauseCircle className="w-3 h-3 mr-1" />
                          暂停
                        </Button>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => handleArchive(tenant.id)}
                          className="flex-1"
                        >
                          <Archive className="w-3 h-3 mr-1" />
                          归档
                        </Button>
                      </>
                    )}
                    {tenant.status === 'suspended' && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleRestore(tenant.id)}
                        className="flex-1"
                      >
                        <RefreshCw className="w-3 h-3 mr-1" />
                        恢复
                      </Button>
                    )}
                    {tenant.status === 'archived' && (
                      <span className="text-xs text-surface-400 w-full text-center">
                        已归档，无法操作
                      </span>
                    )}
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      )}

      {/* 创建租户对话框 */}
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
                <CardTitle>创建租户</CardTitle>
                <CardDescription>
                  创建一个新的租户，后续可以在此租户下添加组织和团队
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreate} className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      租户名称
                    </label>
                    <input
                      type="text"
                      value={newTenantName}
                      onChange={(e) => setNewTenantName(e.target.value)}
                      placeholder="例如：My Company"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      autoFocus
                      required
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
