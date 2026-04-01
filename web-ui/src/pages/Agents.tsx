import { useCallback, useEffect, useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import {
  Search,
  Plus,
  MoreVertical,
  Bot,
  Settings,
  Sparkles,
  Wrench,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { createAgent, getAssistants, getDynamicAgents } from '@/lib/api';
import type { AssistantInfo, DynamicAgent } from '@/lib/api-types';

type AgentCardModel = AssistantInfo & { dynamicMeta?: DynamicAgent };

export default function AgentsPage() {
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [agents, setAgents] = useState<AgentCardModel[]>([]);
  const [creating, setCreating] = useState(false);
  const [newRole, setNewRole] = useState('');
  const [newGuidance, setNewGuidance] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [assistants, dynamic] = await Promise.all([getAssistants(), getDynamicAgents()]);
      const byId = new Map(dynamic.map((d) => [d.id, d]));
      const merged: AgentCardModel[] = assistants.map((a) => ({
        ...a,
        dynamicMeta: byId.get(a.id),
      }));
      setAgents(merged);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const filteredAgents = useMemo(
    () =>
      agents.filter(
        (agent) =>
          agent.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          agent.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
          agent.id.toLowerCase().includes(searchQuery.toLowerCase())
      ),
    [agents, searchQuery]
  );

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    const role = newRole.trim();
    if (!role || submitting) return;
    setSubmitting(true);
    try {
      await createAgent({
        role,
        guidance: newGuidance.trim() || undefined,
      });
      setNewRole('');
      setNewGuidance('');
      setCreating(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  }

  if (loading && agents.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[40vh] text-surface-500">加载中…</div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">Agent 管理</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">数据来自 /api/assistants、/api/agents</p>
        </div>
        <Button type="button" onClick={() => setCreating((v) => !v)}>
          <Plus className="w-5 h-5" />
          创建 Agent
        </Button>
      </div>

      {error && (
        <Card className="border-error-200 dark:border-error-800">
          <CardContent className="p-4 text-sm text-error-600 dark:text-error-400">{error}</CardContent>
        </Card>
      )}

      {creating && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">新建动态 Agent</CardTitle>
            <CardDescription>POST /api/agents · 对应 create 工具创建的 sub-agent</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="flex flex-col sm:flex-row gap-3 sm:items-end">
              <div className="flex-1 space-y-1">
                <label className="text-xs text-surface-500">role（必填）</label>
                <input
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100"
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value)}
                  placeholder="例如：代码审查专员"
                />
              </div>
              <div className="flex-[2] space-y-1">
                <label className="text-xs text-surface-500">guidance（可选）</label>
                <input
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100"
                  value={newGuidance}
                  onChange={(e) => setNewGuidance(e.target.value)}
                  placeholder="行为说明"
                />
              </div>
              <Button type="submit" disabled={submitting || !newRole.trim()}>
                {submitting ? '提交中…' : '创建'}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4">
          <div className="flex items-center gap-4">
            <div className="relative flex-1 max-w-md">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-surface-400" />
              <input
                type="text"
                placeholder="搜索名称、描述或 id…"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-10 pr-4 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {filteredAgents.length === 0 ? (
        <p className="text-sm text-surface-500 dark:text-surface-400">没有匹配的助手</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6">
          {filteredAgents.map((agent, index) => (
            <motion.div
              key={agent.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.05 }}
            >
              <Card className="hover:shadow-elevated transition-shadow duration-200">
                <CardHeader className="pb-3">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-3 min-w-0">
                      <div className="w-12 h-12 rounded-xl bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center shrink-0">
                        <Bot className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                      </div>
                      <div className="min-w-0">
                        <CardTitle className="text-lg truncate">{agent.name}</CardTitle>
                        <div className="flex flex-wrap gap-1 mt-1">
                          {agent.dynamicMeta ? (
                            <Badge variant="info">动态 Agent</Badge>
                          ) : (
                            <Badge variant="success">助手</Badge>
                          )}
                          <span className="text-xs text-surface-400 font-mono truncate max-w-[180px]">
                            {agent.id}
                          </span>
                        </div>
                      </div>
                    </div>
                    <button
                      type="button"
                      className="p-1 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 shrink-0"
                      aria-label="更多"
                    >
                      <MoreVertical className="w-5 h-5 text-surface-400" />
                    </button>
                  </div>
                  <CardDescription className="mt-3 line-clamp-3">{agent.description}</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <div>
                    <div className="flex items-center gap-2 text-xs text-surface-500 dark:text-surface-400 mb-2">
                      <Sparkles className="w-3 h-3" />
                      <span>技能</span>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {(agent.skills?.length ? agent.skills : ['（未配置）']).map((skill) => (
                        <span
                          key={skill}
                          className="px-2 py-1 bg-surface-100 dark:bg-surface-700 rounded-md text-xs text-surface-600 dark:text-surface-300"
                        >
                          {skill}
                        </span>
                      ))}
                    </div>
                  </div>

                  <div>
                    <div className="flex items-center gap-2 text-xs text-surface-500 dark:text-surface-400 mb-2">
                      <Wrench className="w-3 h-3" />
                      <span>工具</span>
                    </div>
                    <p className="text-xs text-surface-500 dark:text-surface-400">
                      工具列表见 /api/tools；策略见 /api/tool-policies
                    </p>
                  </div>

                  {agent.dynamicMeta && (
                    <div className="pt-3 border-t border-surface-200 dark:border-surface-700 text-xs text-surface-500 space-y-1">
                      {agent.dynamicMeta.parent_id && (
                        <p>
                          parent:{' '}
                          <span className="font-mono text-surface-600 dark:text-surface-300">
                            {agent.dynamicMeta.parent_id}
                          </span>
                        </p>
                      )}
                      <p>创建于 {agent.dynamicMeta.created_at}</p>
                    </div>
                  )}

                  <div className="flex gap-2 pt-2">
                    <Button variant="secondary" size="sm" className="flex-1" type="button" disabled>
                      <Settings className="w-4 h-4" />
                      配置
                    </Button>
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
