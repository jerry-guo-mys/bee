import { useCallback, useEffect, useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import { Plus, Search, MoreVertical, Clock, ChevronRight, Workflow, User } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { cn } from '@/lib/utils';
import { createTask, getTasks, getWorkflowTemplates, startWorkflow } from '@/lib/api';
import { ADMIN_SCOPE_CHANGED_EVENT } from '@/lib/scope-storage';
import type { Task, TaskStatus, WorkflowTemplateSummary } from '@/lib/api-types';

/** Kanban column id for UI */
type ColumnId = 'pending' | 'running' | 'completed' | 'error';

function taskToColumn(status: TaskStatus): ColumnId {
  switch (status) {
    case 'todo':
      return 'pending';
    case 'in_progress':
      return 'running';
    case 'done':
      return 'completed';
    default:
      return 'pending';
  }
}

function progressForTask(t: Task): number {
  switch (t.status) {
    case 'todo':
      return 0;
    case 'in_progress':
      return 50;
    case 'done':
      return 100;
    default:
      return 0;
  }
}

const columns: { id: ColumnId; name: string; color: string }[] = [
  { id: 'pending', name: '待执行', color: 'bg-surface-200 dark:bg-surface-700' },
  { id: 'running', name: '执行中', color: 'bg-primary-200 dark:bg-primary-900/30' },
  { id: 'completed', name: '已完成', color: 'bg-success-200 dark:bg-success-900/30' },
  { id: 'error', name: '失败', color: 'bg-error-200 dark:bg-error-900/30' },
];

export default function WorkflowsPage() {
  const [viewMode, setViewMode] = useState<'kanban' | 'list'>('kanban');
  const [searchQuery, setSearchQuery] = useState('');
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [templates, setTemplates] = useState<WorkflowTemplateSummary[]>([]);
  const [templateError, setTemplateError] = useState<string | null>(null);
  const [wfTemplateId, setWfTemplateId] = useState('');
  const [wfTitle, setWfTitle] = useState('');
  const [wfDescription, setWfDescription] = useState('');
  const [wfSubmitting, setWfSubmitting] = useState(false);
  const [wfMessage, setWfMessage] = useState<string | null>(null);
  const [showWorkflowStart, setShowWorkflowStart] = useState(false);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const list = await getTasks();
      setTasks(list);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadTemplates = useCallback(async () => {
    try {
      setTemplateError(null);
      const t = await getWorkflowTemplates();
      setTemplates(t);
    } catch (e) {
      setTemplateError(e instanceof Error ? e.message : String(e));
      setTemplates([]);
    }
  }, []);

  useEffect(() => {
    load();
    loadTemplates();
  }, [load, loadTemplates]);

  useEffect(() => {
    const onScope = () => {
      load();
      loadTemplates();
    };
    window.addEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope);
    return () => window.removeEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope);
  }, [load, loadTemplates]);

  useEffect(() => {
    if (templates.length > 0 && !wfTemplateId) {
      setWfTemplateId(templates[0].id);
    }
  }, [templates, wfTemplateId]);

  const filtered = useMemo(
    () =>
      tasks.filter((t) => t.title.toLowerCase().includes(searchQuery.toLowerCase())),
    [tasks, searchQuery]
  );

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    const title = newTitle.trim();
    if (!title || submitting) return;
    setSubmitting(true);
    try {
      await createTask({ title });
      setNewTitle('');
      setCreating(false);
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  }

  async function handleStartWorkflow(e: React.FormEvent) {
    e.preventDefault();
    const title = wfTitle.trim();
    if (!wfTemplateId || !title || wfSubmitting) return;
    setWfSubmitting(true);
    setWfMessage(null);
    try {
      const res = await startWorkflow({
        template_id: wfTemplateId,
        title,
        description: wfDescription.trim() || undefined,
      });
      setWfMessage(`已创建工作流 ${res.workflow_run_id}，生成 ${res.tasks.length} 个任务`);
      setWfTitle('');
      setWfDescription('');
      await load();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setWfSubmitting(false);
    }
  }

  if (loading && tasks.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[40vh] text-surface-500">加载中…</div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">任务/Workflow</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">数据来自 /api/tasks（看板按任务状态分列）</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Button variant="outline" size="sm" onClick={() => setViewMode(viewMode === 'kanban' ? 'list' : 'kanban')}>
            {viewMode === 'kanban' ? '列表视图' : '看板视图'}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => setShowWorkflowStart((v) => !v)}
          >
            <Workflow className="w-5 h-5" />
            从模板启动
          </Button>
          <Button type="button" onClick={() => setCreating((v) => !v)}>
            <Plus className="w-5 h-5" />
            创建任务
          </Button>
        </div>
      </div>

      {error && (
        <Card className="border-error-200 dark:border-error-800">
          <CardContent className="p-4 text-sm text-error-600 dark:text-error-400">{error}</CardContent>
        </Card>
      )}

      {showWorkflowStart && (
        <Card>
          <CardContent className="p-4 space-y-4">
            <p className="text-sm text-surface-600 dark:text-surface-400">
              <code className="text-xs">GET /api/workflow-templates</code> ·{' '}
              <code className="text-xs">POST /api/workflows/start</code>
            </p>
            {templateError && (
              <p className="text-sm text-error-600 dark:text-error-400">{templateError}</p>
            )}
            {templates.length > 0 && (
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                {templates.map((tpl) => (
                  <button
                    key={tpl.id}
                    type="button"
                    onClick={() => setWfTemplateId(tpl.id)}
                    className={cn(
                      'text-left rounded-lg border p-3 transition-colors',
                      wfTemplateId === tpl.id
                        ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20'
                        : 'border-surface-200 dark:border-surface-700 hover:bg-surface-50 dark:hover:bg-surface-800'
                    )}
                  >
                    <div className="font-medium text-surface-900 dark:text-surface-100 text-sm">
                      {tpl.name}
                    </div>
                    <div className="text-xs text-surface-500 mt-1 line-clamp-2">{tpl.description}</div>
                    <Badge variant="default" className="mt-2">
                      {tpl.team_hint}
                    </Badge>
                  </button>
                ))}
              </div>
            )}
            <form onSubmit={handleStartWorkflow} className="space-y-3 max-w-xl">
              <div>
                <label className="block text-xs font-medium text-surface-500 mb-1">工作流标题</label>
                <input
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100"
                  value={wfTitle}
                  onChange={(e) => setWfTitle(e.target.value)}
                  placeholder="例如：Q1 重点客户跟进"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-surface-500 mb-1">描述（可选，写入各子任务）</label>
                <textarea
                  className="w-full px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 min-h-[72px]"
                  value={wfDescription}
                  onChange={(e) => setWfDescription(e.target.value)}
                />
              </div>
              <Button type="submit" disabled={wfSubmitting || !wfTemplateId || !wfTitle.trim()}>
                {wfSubmitting ? '启动中…' : '启动工作流'}
              </Button>
            </form>
            {wfMessage && (
              <p className="text-sm text-success-600 dark:text-success-400">{wfMessage}</p>
            )}
          </CardContent>
        </Card>
      )}

      {creating && (
        <Card>
          <CardContent className="p-4">
            <form onSubmit={handleCreate} className="flex flex-col sm:flex-row gap-3 sm:items-center">
              <input
                className="flex-1 px-3 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100"
                placeholder="任务标题"
                value={newTitle}
                onChange={(e) => setNewTitle(e.target.value)}
              />
              <Button type="submit" disabled={submitting || !newTitle.trim()}>
                {submitting ? '创建中…' : 'POST /api/tasks'}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-surface-400" />
            <input
              type="text"
              placeholder="按标题筛选…"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-700 text-surface-900 dark:text-surface-100 focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
        </CardContent>
      </Card>

      {viewMode === 'kanban' ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {columns.map((column) => {
            const columnTasks =
              column.id === 'error'
                ? []
                : filtered.filter((w) => taskToColumn(w.status) === column.id);
            return (
              <div key={column.id} className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className={cn('w-3 h-3 rounded-full', column.color)} />
                    <span className="font-medium text-surface-700 dark:text-surface-300">{column.name}</span>
                    <Badge variant="default">{columnTasks.length}</Badge>
                  </div>
                </div>
                {column.id === 'error' && (
                  <p className="text-xs text-surface-400 px-1">
                    任务模型无失败态；失败请求见「监控」追踪
                  </p>
                )}
                <div className="space-y-3">
                  {columnTasks.map((task) => {
                    const uiStatus = taskToColumn(task.status);
                    const progress = progressForTask(task);
                    return (
                      <motion.div
                        key={task.id}
                        layoutId={task.id}
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: -10 }}
                      >
                        <Card className="cursor-pointer hover:shadow-elevated transition-shadow">
                          <CardContent className="p-4 space-y-3">
                            <div className="flex items-start justify-between gap-2">
                              <div className="flex items-center gap-2 min-w-0">
                                <Workflow className="w-4 h-4 text-surface-400 shrink-0" />
                                <span className="font-medium text-sm text-surface-900 dark:text-surface-100 truncate">
                                  {task.title}
                                </span>
                              </div>
                              <button
                                type="button"
                                className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-700 shrink-0"
                                aria-label="更多"
                              >
                                <MoreVertical className="w-4 h-4 text-surface-400" />
                              </button>
                            </div>

                            <p className="text-xs text-surface-500 dark:text-surface-400 line-clamp-2">
                              {task.description || '（无描述）'}
                            </p>

                            <div className="space-y-1">
                              <div className="flex items-center justify-between text-xs">
                                <span className="text-surface-500 dark:text-surface-400">进度</span>
                                <span className="font-medium text-surface-700 dark:text-surface-300">
                                  {progress}%
                                </span>
                              </div>
                              <div className="w-full bg-surface-200 dark:bg-surface-700 rounded-full h-1.5">
                                <div
                                  className={cn(
                                    'h-1.5 rounded-full transition-all',
                                    uiStatus === 'completed'
                                      ? 'bg-success-500'
                                      : uiStatus === 'running'
                                        ? 'bg-primary-500'
                                        : 'bg-surface-400'
                                  )}
                                  style={{ width: `${progress}%` }}
                                />
                              </div>
                            </div>

                            <div className="flex items-center justify-between text-xs text-surface-400">
                              <div className="flex items-center gap-1 min-w-0">
                                <User className="w-3 h-3 shrink-0" />
                                <span className="truncate">
                                  {task.workflow_template_id
                                    ? `模板 ${task.workflow_template_id}`
                                    : task.coordinator_id || '未指定统筹'}
                                </span>
                              </div>
                              <div className="flex items-center gap-1 shrink-0">
                                <Clock className="w-3 h-3" />
                                <span className="font-mono text-[10px]">{task.id.slice(0, 8)}</span>
                              </div>
                            </div>
                          </CardContent>
                        </Card>
                      </motion.div>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <Card>
          <div className="divide-y divide-surface-200 dark:divide-surface-700">
            {filtered.length === 0 ? (
              <div className="p-8 text-center text-surface-500 text-sm">暂无任务</div>
            ) : (
              filtered.map((task) => {
                const uiStatus = taskToColumn(task.status);
                const progress = progressForTask(task);
                return (
                  <div
                    key={task.id}
                    className="p-4 hover:bg-surface-50 dark:hover:bg-surface-800 transition-colors"
                  >
                    <div className="flex items-center justify-between gap-4">
                      <div className="flex items-center gap-4 min-w-0">
                        <div
                          className={cn(
                            'w-10 h-10 rounded-lg flex items-center justify-center shrink-0',
                            uiStatus === 'completed'
                              ? 'bg-success-100 dark:bg-success-900/20'
                              : uiStatus === 'running'
                                ? 'bg-primary-100 dark:bg-primary-900/20'
                                : 'bg-surface-100 dark:bg-surface-800'
                          )}
                        >
                          <Workflow
                            className={cn(
                              'w-5 h-5',
                              uiStatus === 'completed'
                                ? 'text-success-600'
                                : uiStatus === 'running'
                                  ? 'text-primary-600'
                                  : 'text-surface-500'
                            )}
                          />
                        </div>
                        <div className="min-w-0">
                          <h3 className="font-medium text-surface-900 dark:text-surface-100 truncate">
                            {task.title}
                          </h3>
                          <p className="text-sm text-surface-500 dark:text-surface-400 truncate">
                            {task.description || task.id}
                          </p>
                        </div>
                      </div>
                      <div className="flex items-center gap-6 shrink-0">
                        <div className="text-right">
                          <Badge
                            variant={
                              uiStatus === 'completed'
                                ? 'success'
                                : uiStatus === 'running'
                                  ? 'info'
                                  : 'default'
                            }
                          >
                            {uiStatus === 'completed'
                              ? '已完成'
                              : uiStatus === 'running'
                                ? '执行中'
                                : '待执行'}
                          </Badge>
                          <p className="text-xs text-surface-400 mt-1">{progress}%</p>
                        </div>
                        <ChevronRight className="w-5 h-5 text-surface-400" />
                      </div>
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
