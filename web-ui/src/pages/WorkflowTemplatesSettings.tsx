import { useCallback, useEffect, useState } from 'react';
import { GitBranch, Plus, RefreshCw } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import {
  addAdminWorkflowTemplateVersion,
  createAdminWorkflowTemplate,
  getAdminWorkflowTemplates,
  publishAdminWorkflowTemplate,
} from '@/lib/api';
import { ADMIN_SCOPE_CHANGED_EVENT } from '@/lib/scope-storage';
import type { AdminWorkflowTemplateDetail } from '@/lib/api-types';

const DEFAULT_DEFINITION = `{
  "steps": [
    { "title": "步骤一", "default_agent_template_id": null },
    { "title": "步骤二", "default_agent_template_id": null }
  ]
}`;

export default function WorkflowTemplatesSettingsPage() {
  const [templates, setTemplates] = useState<AdminWorkflowTemplateDetail[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [slug, setSlug] = useState('');
  const [name, setName] = useState('');
  const [defJson, setDefJson] = useState(DEFAULT_DEFINITION);
  const [creating, setCreating] = useState(false);
  const [publishVersion, setPublishVersion] = useState<Record<string, string>>({});

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const res = await getAdminWorkflowTemplates();
      setTemplates(res.templates ?? []);
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

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    const s = slug.trim();
    const n = name.trim();
    if (!s || !n || creating) return;
    let definition: unknown;
    try {
      definition = JSON.parse(defJson);
    } catch {
      setError('definition 不是合法 JSON');
      return;
    }
    setCreating(true);
    setError(null);
    try {
      await createAdminWorkflowTemplate({ slug: s, name: n, definition });
      setSlug('');
      setName('');
      setDefJson(DEFAULT_DEFINITION);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }

  async function handlePublish(templateId: string) {
    const raw = publishVersion[templateId]?.trim();
    const v = raw ? parseInt(raw, 10) : NaN;
    if (!Number.isFinite(v) || v < 1) {
      setError('请填写要发布的版本号（正整数）');
      return;
    }
    setError(null);
    try {
      await publishAdminWorkflowTemplate(templateId, v);
      setPublishVersion((m) => ({ ...m, [templateId]: '' }));
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleAddVersion(templateId: string, definition: unknown) {
    setError(null);
    try {
      await addAdminWorkflowTemplateVersion(templateId, definition);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="max-w-5xl space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100 flex items-center gap-2">
          <GitBranch className="w-7 h-7" />
          流程模板（专家）
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1 text-sm">
          创建草稿、发布版本后，业务工作台 <code className="text-xs">/workbench/runs</code> 的模板列表会与内置模板合并（同 slug
          时<strong>租户模板覆盖内置</strong>）。需在「系统设置」中配置与后端一致的 tenant_id。
        </p>
      </div>

      {error && (
        <Card className="border-error-200 dark:border-error-800">
          <CardContent className="p-4 text-sm text-error-600 dark:text-error-400">{error}</CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">新建草稿模板</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label className="block text-xs font-medium text-surface-500 mb-1">slug（唯一，字母数字下划线横线）</label>
                <input
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 font-mono text-sm"
                  value={slug}
                  onChange={(e) => setSlug(e.target.value)}
                  placeholder="my_onboarding"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-surface-500 mb-1">名称</label>
                <input
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-sm"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="展示名"
                />
              </div>
            </div>
            <div>
              <label className="block text-xs font-medium text-surface-500 mb-1">definition JSON</label>
              <textarea
                className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 font-mono text-xs min-h-[160px]"
                value={defJson}
                onChange={(e) => setDefJson(e.target.value)}
              />
            </div>
            <Button type="submit" disabled={creating}>
              <Plus className="w-4 h-4 mr-1" />
              {creating ? '创建中…' : '创建（含 v1 草稿）'}
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-surface-900 dark:text-surface-100">已有模板</h2>
        <Button type="button" variant="outline" size="sm" onClick={() => load()} disabled={loading}>
          <RefreshCw className={`w-4 h-4 mr-1 ${loading ? 'animate-spin' : ''}`} />
          刷新
        </Button>
      </div>

      {loading && templates.length === 0 ? (
        <p className="text-surface-500 text-sm">加载中…</p>
      ) : templates.length === 0 ? (
        <p className="text-surface-500 text-sm">暂无数据（或当前租户下未创建模板）</p>
      ) : (
        <div className="space-y-4">
          {templates.map((t) => (
            <Card key={t.id}>
              <CardContent className="p-4 space-y-3">
                <div className="flex flex-wrap items-center gap-2 justify-between">
                  <div>
                    <span className="font-semibold text-surface-900 dark:text-surface-100">{t.name}</span>
                    <code className="ml-2 text-xs text-surface-500">{t.slug}</code>
                  </div>
                  <Badge variant="default">{t.status}</Badge>
                </div>
                <p className="text-xs text-surface-500 font-mono break-all">id: {t.id}</p>
                <div className="text-sm text-surface-600 dark:text-surface-400">
                  <span className="font-medium">版本：</span>
                  {t.versions.length === 0 ? (
                    '无'
                  ) : (
                    <ul className="list-disc pl-5 mt-1">
                      {t.versions.map((v) => (
                        <li key={v.version}>
                          v{v.version}
                          {v.published_at ? (
                            <span className="text-success-600 dark:text-success-400 ml-1">（已发布）</span>
                          ) : (
                            <span className="text-amber-600 dark:text-amber-400 ml-1">（草稿）</span>
                          )}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
                <div className="flex flex-wrap gap-2 items-end">
                  <div>
                    <label className="block text-xs text-surface-500 mb-1">发布版本号</label>
                    <input
                      className="w-24 px-2 py-1.5 rounded border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-sm"
                      value={publishVersion[t.id] ?? ''}
                      onChange={(e) =>
                        setPublishVersion((m) => ({
                          ...m,
                          [t.id]: e.target.value,
                        }))
                      }
                      placeholder="1"
                    />
                  </div>
                  <Button type="button" size="sm" variant="outline" onClick={() => handlePublish(t.id)}>
                    发布
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      try {
                        void handleAddVersion(t.id, JSON.parse(DEFAULT_DEFINITION));
                      } catch {
                        setError('内置 definition JSON 无效');
                      }
                    }}
                  >
                    追加新版本（默认两步草稿）
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
