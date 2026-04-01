import { useCallback, useEffect, useMemo, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { getToolPolicies, getTools, putToolPolicy } from '@/lib/api';
import { ADMIN_SCOPE_CHANGED_EVENT, loadAdminScope } from '@/lib/scope-storage';
import type { ToolAccessPolicy, ToolInfo } from '@/lib/api-types';
import { cn } from '@/lib/utils';

function policyMatchesScope(p: ToolAccessPolicy, tenantId: string, orgId: string, teamId: string) {
  if (p.tenant_id !== tenantId) return false;
  const pOrg = p.organization_id ?? '';
  if (pOrg !== orgId) return false;
  const pTeam = p.team_id ?? '';
  if (teamId) return pTeam === teamId;
  return pTeam === '';
}

export default function ToolPoliciesPage() {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [policies, setPolicies] = useState<ToolAccessPolicy[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [allowed, setAllowed] = useState<Set<string>>(new Set());
  const [denied, setDenied] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [tList, pList] = await Promise.all([getTools(), getToolPolicies()]);
      setTools(tList);
      setPolicies(pList);
      const s = loadAdminScope();
      const match = pList.find((p) =>
        policyMatchesScope(p, s.tenant_id, s.organization_id, s.team_id)
      );
      setAllowed(new Set(match?.allowed_tool_ids ?? []));
      setDenied(new Set(match?.denied_tool_ids ?? []));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const onScope = () => load();
    window.addEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope);
    return () => window.removeEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope);
  }, [load]);

  const s = loadAdminScope();
  const scopeLabel = useMemo(() => {
    const parts = [s.tenant_id, s.organization_id];
    if (s.team_id) parts.push(`team:${s.team_id}`);
    return parts.join(' / ');
  }, [s.tenant_id, s.organization_id, s.team_id]);

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await putToolPolicy({
        allowed_tool_ids: [...allowed],
        denied_tool_ids: [...denied],
      });
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  if (loading && tools.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[40vh] text-surface-500">加载中…</div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">工具策略</h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">
          对应 <code className="text-xs">GET/PUT /api/tool-policies</code>。当前作用域：{' '}
          <span className="font-mono text-sm">{scopeLabel}</span>
        </p>
      </div>

      {error && (
        <Card className="border-error-200 dark:border-error-800">
          <CardContent className="p-4 text-sm text-error-600 dark:text-error-400">{error}</CardContent>
        </Card>
      )}

      <form onSubmit={handleSave} className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>允许 / 拒绝列表</CardTitle>
            <CardDescription>
              勾选「允许」与「拒绝」；同一工具不建议同时出现在两侧（以后端生效逻辑为准）。保存为当前设置中的作用域写入策略。
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="grid grid-cols-[1fr_auto_auto] gap-2 text-xs font-medium text-surface-500 uppercase tracking-wide px-2">
              <span>工具</span>
              <span className="text-center">允许</span>
              <span className="text-center">拒绝</span>
            </div>
            <div className="divide-y divide-surface-200 dark:divide-surface-700 max-h-[50vh] overflow-y-auto rounded-lg border border-surface-200 dark:border-surface-700">
              {tools.map((t) => (
                <div
                  key={t.id}
                  className="grid grid-cols-[1fr_auto_auto] gap-2 items-center px-2 py-2 text-sm"
                >
                  <div className="min-w-0">
                    <div className="font-mono text-surface-900 dark:text-surface-100 truncate">
                      {t.id}
                    </div>
                    {t.description && (
                      <div className="text-xs text-surface-500 line-clamp-2">{t.description}</div>
                    )}
                  </div>
                  <label className="flex justify-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={allowed.has(t.id)}
                      onChange={() => {
                        setAllowed((prev) => {
                          const next = new Set(prev);
                          if (next.has(t.id)) next.delete(t.id);
                          else {
                            next.add(t.id);
                            setDenied((d) => {
                              const nd = new Set(d);
                              nd.delete(t.id);
                              return nd;
                            });
                          }
                          return next;
                        });
                      }}
                      className="rounded border-surface-300"
                    />
                  </label>
                  <label className="flex justify-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={denied.has(t.id)}
                      onChange={() => {
                        setDenied((prev) => {
                          const next = new Set(prev);
                          if (next.has(t.id)) next.delete(t.id);
                          else {
                            next.add(t.id);
                            setAllowed((a) => {
                              const na = new Set(a);
                              na.delete(t.id);
                              return na;
                            });
                          }
                          return next;
                        });
                      }}
                      className="rounded border-surface-300"
                    />
                  </label>
                </div>
              ))}
            </div>
            <div className="pt-4">
              <Button type="submit" disabled={saving}>
                {saving ? '保存中…' : 'PUT /api/tool-policies'}
              </Button>
            </div>
          </CardContent>
        </Card>
      </form>

      <Card>
        <CardHeader>
          <CardTitle>已有策略（本租户下）</CardTitle>
          <CardDescription>来自当前查询结果，便于对照。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {policies.length === 0 ? (
            <p className="text-sm text-surface-500">暂无记录（或未命中 SaaS 库）</p>
          ) : (
            policies.map((p) => {
              const active = policyMatchesScope(p, s.tenant_id, s.organization_id, s.team_id);
              return (
                <div
                  key={p.id}
                  className={cn(
                    'rounded-lg border p-3 text-sm',
                    active
                      ? 'border-primary-300 bg-primary-50/50 dark:border-primary-800 dark:bg-primary-900/10'
                      : 'border-surface-200 dark:border-surface-700'
                  )}
                >
                  <div className="flex flex-wrap items-center gap-2 mb-2">
                    <span className="font-mono text-xs">{p.id}</span>
                    {active && <Badge variant="info">当前作用域</Badge>}
                  </div>
                  <div className="text-xs text-surface-500 space-y-1">
                    <div>
                      org: {p.organization_id ?? '—'} · team: {p.team_id ?? '（组织级）'}
                    </div>
                    <div>允许: {(p.allowed_tool_ids ?? []).join(', ') || '—'}</div>
                    <div>拒绝: {(p.denied_tool_ids ?? []).join(', ') || '—'}</div>
                  </div>
                </div>
              );
            })
          )}
        </CardContent>
      </Card>
    </div>
  );
}
