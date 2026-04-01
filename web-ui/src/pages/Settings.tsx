import { useCallback, useEffect, useState } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import {
  DEFAULT_ADMIN_SCOPE,
  dispatchAdminScopeChanged,
  loadAdminScope,
  saveAdminScope,
  type AdminScope,
} from '@/lib/scope-storage';

export default function SettingsPage() {
  const [scope, setScope] = useState<AdminScope>(() => ({
    ...DEFAULT_ADMIN_SCOPE,
    team_id: '',
  }));
  const [savedAt, setSavedAt] = useState<string | null>(null);

  useEffect(() => {
    setScope(loadAdminScope());
  }, []);

  const update = useCallback((patch: Partial<AdminScope>) => {
    setScope((prev) => ({ ...prev, ...patch }));
  }, []);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const next: AdminScope = {
      tenant_id: scope.tenant_id.trim() || DEFAULT_ADMIN_SCOPE.tenant_id,
      organization_id: scope.organization_id.trim() || DEFAULT_ADMIN_SCOPE.organization_id,
      user_id: scope.user_id.trim() || DEFAULT_ADMIN_SCOPE.user_id,
      team_id: scope.team_id.trim(),
    };
    saveAdminScope(next);
    setScope(next);
    setSavedAt(new Date().toLocaleTimeString());
    dispatchAdminScopeChanged();
  }

  return (
    <div className="max-w-2xl space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">系统设置</h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">
          管理 API 作用域（与后端 <code className="text-xs">WebScopeParams</code> 对齐），写入本机{' '}
          <code className="text-xs">localStorage</code>。带权限校验的接口会读取这些字段。
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>租户 / 组织 / 用户</CardTitle>
          <CardDescription>
            修改后已打开的页面不会自动重载；保存后会通知各页在合适时机重新拉取数据，或请手动刷新。
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                tenant_id
              </label>
              <input
                className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 font-mono text-sm"
                value={scope.tenant_id}
                onChange={(e) => update({ tenant_id: e.target.value })}
                autoComplete="off"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                organization_id
              </label>
              <input
                className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 font-mono text-sm"
                value={scope.organization_id}
                onChange={(e) => update({ organization_id: e.target.value })}
                autoComplete="off"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                user_id（管理身份，用于审计等）
              </label>
              <input
                className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 font-mono text-sm"
                value={scope.user_id}
                onChange={(e) => update({ user_id: e.target.value })}
                autoComplete="off"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                team_id（可选，留空表示组织级）
              </label>
              <input
                className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 font-mono text-sm"
                value={scope.team_id}
                onChange={(e) => update({ team_id: e.target.value })}
                placeholder="例如 sales-team-1"
                autoComplete="off"
              />
            </div>
            <div className="flex items-center gap-3 pt-2">
              <Button type="submit">保存作用域</Button>
              {savedAt && (
                <span className="text-sm text-success-600 dark:text-success-400">
                  已保存（{savedAt}）
                </span>
              )}
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
