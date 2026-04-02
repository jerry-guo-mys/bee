import { useCallback, useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Users, Plus, GitBranch } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import {
  getTeams,
  getOrganizations,
  createTeam,
  type Team,
  type Organization,
} from '@/lib/api';

export default function TeamsPage() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [orgFilter, setOrgFilter] = useState<string>('all');
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [newTeamName, setNewTeamName] = useState('');
  const [newTeamCode, setNewTeamCode] = useState('');
  const [newTeamDescription, setNewTeamDescription] = useState('');
  const [newTeamOrgId, setNewTeamOrgId] = useState('');
  const [newTeamParentId, setNewTeamParentId] = useState('');
  const [creating, setCreating] = useState(false);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [teamsData, orgsData] = await Promise.all([
        getTeams(),
        getOrganizations(),
      ]);
      setTeams(teamsData.teams ?? []);
      setOrganizations(orgsData.organizations ?? []);
      if (orgsData.organizations?.length > 0 && !newTeamOrgId) {
        setNewTeamOrgId(orgsData.organizations[0].id);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [newTeamOrgId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTeamName.trim() || !newTeamOrgId) return;
    try {
      setCreating(true);
      await createTeam({
        organization_id: newTeamOrgId,
        name: newTeamName.trim(),
        code: newTeamCode.trim() || undefined,
        description: newTeamDescription.trim() || undefined,
        parent_team_id: newTeamParentId || undefined,
      });
      setNewTeamName('');
      setNewTeamCode('');
      setNewTeamDescription('');
      setNewTeamParentId('');
      setShowCreateDialog(false);
      await loadData();
    } catch (err) {
      alert(`创建失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCreating(false);
    }
  };

  const filteredTeams = teams.filter((team) =>
    orgFilter === 'all' ? true : team.organization_id === orgFilter
  );

  const getOrgName = (orgId: string) => {
    return organizations.find((o) => o.id === orgId)?.name || 'Unknown';
  };

  const getParentTeamName = (parentId: string) => {
    if (!parentId) return '无';
    return teams.find((t) => t.id === parentId)?.name || 'Unknown';
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">团队管理</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            管理组织下的团队，支持层级结构和成员统计
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          创建团队
        </Button>
      </div>

      {/* 筛选器 */}
      <div className="flex gap-2 items-center">
        <span className="text-sm text-surface-500">按组织筛选:</span>
        <select
          value={orgFilter}
          onChange={(e) => setOrgFilter(e.target.value)}
          className="px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500 text-sm"
        >
          <option value="all">全部组织</option>
          {organizations.map((org) => (
            <option key={org.id} value={org.id}>
              {org.name}
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

      {/* 团队列表 */}
      {loading ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            加载中...
          </CardContent>
        </Card>
      ) : filteredTeams.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-surface-500">
            <Users className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>暂无团队数据</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredTeams.map((team) => (
            <motion.div
              key={team.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
            >
              <Card>
                <CardHeader>
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                        <Users className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                      </div>
                      <div>
                        <CardTitle className="text-lg">{team.name}</CardTitle>
                        <CardDescription className="text-xs font-mono">
                          {team.code || '无代码'}
                        </CardDescription>
                      </div>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span className="text-surface-500">所属组织</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {getOrgName(team.organization_id)}
                      </span>
                    </div>
                    {team.parent_team_id && (
                      <div className="flex justify-between">
                        <span className="text-surface-500">父团队</span>
                        <span className="font-medium text-surface-900 dark:text-surface-100">
                          <GitBranch className="w-4 h-4 inline mr-1" />
                          {getParentTeamName(team.parent_team_id)}
                        </span>
                      </div>
                    )}
                    <div className="flex justify-between">
                      <span className="text-surface-500">成员数量</span>
                      <span className="font-medium text-surface-900 dark:text-surface-100">
                        {team.member_count}
                      </span>
                    </div>
                    {team.description && (
                      <div className="pt-2 border-t border-surface-200 dark:border-surface-700">
                        <p className="text-surface-500 text-xs">{team.description}</p>
                      </div>
                    )}
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      )}

      {/* 创建团队对话框 */}
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
                <CardTitle>创建团队</CardTitle>
                <CardDescription>
                  在指定组织下创建一个新的团队，可设置父团队实现层级管理
                </CardDescription>
              </CardHeader>
              <CardContent>
                <form onSubmit={handleCreate} className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      选择组织
                    </label>
                    <select
                      value={newTeamOrgId}
                      onChange={(e) => setNewTeamOrgId(e.target.value)}
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      required
                    >
                      {organizations.map((org) => (
                        <option key={org.id} value={org.id}>
                          {org.name}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      团队名称
                    </label>
                    <input
                      type="text"
                      value={newTeamName}
                      onChange={(e) => setNewTeamName(e.target.value)}
                      placeholder="例如：Backend Team"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      autoFocus
                      required
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      团队代码 (可选)
                    </label>
                    <input
                      type="text"
                      value={newTeamCode}
                      onChange={(e) => setNewTeamCode(e.target.value)}
                      placeholder="例如：BE-001"
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      团队描述 (可选)
                    </label>
                    <textarea
                      value={newTeamDescription}
                      onChange={(e) => setNewTeamDescription(e.target.value)}
                      placeholder="团队职责描述..."
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                      rows={2}
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                      父团队 (可选)
                    </label>
                    <select
                      value={newTeamParentId}
                      onChange={(e) => setNewTeamParentId(e.target.value)}
                      className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
                    >
                      <option value="">无 (作为顶级团队)</option>
                      {teams
                        .filter((t) => t.organization_id === newTeamOrgId)
                        .map((team) => (
                          <option key={team.id} value={team.id}>
                            {team.name}
                          </option>
                        ))}
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
