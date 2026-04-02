import { useCallback, useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { UserCheck, Plus, Mail, Shield } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import {
  getMembers,
  getTenants,
  getOrganizations,
  type Member,
  type Tenant,
  type Organization,
} from '@/lib/api';

type RoleType = 'owner' | 'admin' | 'member' | 'viewer';

const roleLabels: Record<string, string> = {
  owner: '所有者',
  admin: '管理员',
  member: '成员',
  viewer: '访客',
};

const roleVariants: Record<string, 'success' | 'default' | 'warning' | 'info'> = {
  owner: 'success',
  admin: 'default',
  member: 'warning',
  viewer: 'info',
};

const statusLabels: Record<string, string> = {
  active: '活跃',
  pending: '待激活',
  suspended: '已暂停',
};

const statusVariants: Record<string, 'success' | 'warning' | 'error'> = {
  active: 'success',
  pending: 'warning',
  suspended: 'error',
};

export default function MembersPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [members, setMembers] = useState<Member[]>([]);
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [tenantFilter, setTenantFilter] = useState<string>('all');
  const [orgFilter, setOrgFilter] = useState<string>('all');
  const [roleFilter, setRoleFilter] = useState<string>('all');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [creating, setCreating] = useState(false);

  // 创建成员相关状态
  const [newMemberEmail, setNewMemberEmail] = useState('');
  const [newMemberRole, setNewMemberRole] = useState<RoleType>('member');
  const [newMemberTenantId, setNewMemberTenantId] = useState('');
  const [newMemberOrgId, setNewMemberOrgId] = useState('');

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [membersData, tenantsData, orgsData] = await Promise.all([
        getMembers(),
        getTenants(),
        getOrganizations(),
      ]);
      setMembers(membersData.members ?? []);
      setTenants(tenantsData.tenants ?? []);
      setOrganizations(orgsData.organizations ?? []);
      if (tenantsData.tenants?.length > 0 && !newMemberTenantId) {
        setNewMemberTenantId(tenantsData.tenants[0].id);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [newMemberTenantId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newMemberEmail.trim() || !newMemberOrgId) {
      alert('请填写完整的成员信息');
      return;
    }
    try {
      setCreating(true);
      // TODO: 实现 createMember API
      alert('成员创建功能待实现');
      setShowCreateDialog(false);
      setNewMemberEmail('');
      setNewMemberRole('member');
      await loadData();
    } catch (err) {
      alert(`创建失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const filteredMembers = members.filter((member) => {
    if (tenantFilter !== 'all' && member.tenant_id !== tenantFilter) return false;
    if (orgFilter !== 'all' && member.organization_id !== orgFilter) return false;
    if (roleFilter !== 'all' && member.role !== roleFilter) return false;
    return true;
  });

  const getOrgName = (orgId: string) => {
    return organizations.find((o) => o.id === orgId)?.name || 'Unknown';
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">成员管理</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            管理租户和组织下的成员，包括角色分配和状态控制
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          添加成员
        </Button>
      </div>

      {/* 筛选器 */}
      <div className="flex flex-wrap gap-2 items-center">
        <span className="text-sm text-surface-500">租户:</span>
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

        <span className="text-sm text-surface-500 ml-2">组织:</span>
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

        <span className="text-sm text-surface-500 ml-2">角色:</span>
        <select
          value={roleFilter}
          onChange={(e) => setRoleFilter(e.target.value)}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
        >
          <option value="all">全部角色</option>
          <option value="owner">所有者</option>
          <option value="admin">管理员</option>
          <option value="member">成员</option>
          <option value="viewer">访客</option>
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

      {/* 成员列表 */}
      {loading ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            加载中...
          </CardContent>
        </Card>
      ) : filteredMembers.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            <UserCheck className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>暂无成员数据</p>
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
                      成员
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      邮箱
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      角色
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      状态
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      所属组织
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-surface-500 uppercase tracking-wider">
                      加入时间
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-surface-200 dark:divide-surface-700">
                  {filteredMembers.map((member) => (
                    <tr
                      key={member.id}
                      className="hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors"
                    >
                      <td className="px-4 py-3 whitespace-nowrap">
                        <div className="flex items-center">
                          <div className="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center mr-3">
                            <span className="text-sm font-medium text-primary-700 dark:text-primary-400">
                              {member.email?.charAt(0).toUpperCase() || 'U'}
                            </span>
                          </div>
                          <div>
                            <div className="text-sm font-medium text-surface-900 dark:text-surface-100">
                              {member.user_id.slice(0, 12)}...
                            </div>
                            <div className="text-xs text-surface-500">
                              ID: {member.id.slice(0, 8)}
                            </div>
                          </div>
                        </div>
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <div className="flex items-center text-sm text-surface-600 dark:text-surface-400">
                          <Mail className="w-4 h-4 mr-2" />
                          {member.email || '未设置'}
                        </div>
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <Badge variant={roleVariants[member.role] || 'info'}>
                          <Shield className="w-3 h-3 mr-1" />
                          {roleLabels[member.role] || member.role}
                        </Badge>
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        <Badge variant={statusVariants[member.status] || 'info'}>
                          {statusLabels[member.status] || member.status}
                        </Badge>
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-600 dark:text-surface-400">
                        {getOrgName(member.organization_id)}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-surface-500">
                        {new Date(member.created_at).toLocaleDateString('zh-CN')}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}

      {/* 添加成员对话框 */}
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
                <CardTitle>添加成员</CardTitle>
                <CardDescription>
                  邀请新成员加入组织，并分配角色
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreate} className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      选择租户
                    </label>
                    <select
                      value={newMemberTenantId}
                      onChange={(e) => {
                        setNewMemberTenantId(e.target.value);
                        setNewMemberOrgId('');
                      }}
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
                      选择组织
                    </label>
                    <select
                      value={newMemberOrgId}
                      onChange={(e) => setNewMemberOrgId(e.target.value)}
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      required
                    >
                      <option value="">请先选择租户</option>
                      {organizations
                        .filter((org) => org.tenant_id === newMemberTenantId)
                        .map((org) => (
                          <option key={org.id} value={org.id}>
                            {org.name}
                          </option>
                        ))}
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      成员邮箱
                    </label>
                    <input
                      type="email"
                      value={newMemberEmail}
                      onChange={(e) => setNewMemberEmail(e.target.value)}
                      placeholder="example@company.com"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      autoFocus
                      required
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      角色
                    </label>
                    <select
                      value={newMemberRole}
                      onChange={(e) => setNewMemberRole(e.target.value as RoleType)}
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                    >
                      <option value="owner">所有者</option>
                      <option value="admin">管理员</option>
                      <option value="member">成员</option>
                      <option value="viewer">访客</option>
                    </select>
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
                      {creating ? '发送邀请...' : '发送邀请'}
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
