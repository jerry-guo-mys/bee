import { useCallback, useEffect, useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import { Bot, Workflow, BarChart3, AlertCircle, TrendingUp, Clock } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { cn } from '@/lib/utils';
import { getAssistants, getMetrics, getTasks, getTracesRecent } from '@/lib/api';
import { ADMIN_SCOPE_CHANGED_EVENT } from '@/lib/scope-storage';
import type { MetricsResponse, Task, TraceSummary } from '@/lib/api-types';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}

function traceActivityMessage(t: TraceSummary): string {
  const head = t.input_summary?.trim() || `请求 ${t.request_id.slice(0, 8)}…`;
  return head.length > 80 ? `${head.slice(0, 80)}…` : head;
}

function traceStatusUi(
  s: TraceSummary['status']
): { label: string; variant: 'success' | 'error' | 'warning' | 'info'; dot: string } {
  switch (s) {
    case 'success':
      return { label: 'success', variant: 'success', dot: 'bg-success-500' };
    case 'failure':
      return { label: 'error', variant: 'error', dot: 'bg-error-500' };
    case 'cancelled':
      return { label: 'warning', variant: 'warning', dot: 'bg-warning-500' };
    default:
      return { label: 'running', variant: 'info', dot: 'bg-primary-500' };
  }
}

export default function Dashboard() {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [assistantCount, setAssistantCount] = useState(0);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [m, tr, tk, asst] = await Promise.all([
        getMetrics(),
        getTracesRecent(10),
        getTasks(),
        getAssistants().catch(() => []),
      ]);
      setMetrics(m);
      setTraces(tr.traces ?? []);
      setTasks(tk);
      setAssistantCount(asst.length);
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

  const runningTasks = useMemo(() => tasks.filter((t) => t.status === 'in_progress').length, [tasks]);
  const errorSignals = useMemo(() => {
    if (!metrics) return 0;
    return (
      metrics.llm.failed_calls +
      metrics.tools.failed_executions +
      metrics.behavior.total_errors
    );
  }, [metrics]);

  const stats = useMemo(() => {
    if (!metrics) {
      return [
        { name: '活跃 Agent', value: '—', icon: Bot, color: 'text-primary-600', bgColor: 'bg-primary-100 dark:bg-primary-900/20' },
        { name: '运行中任务', value: '—', icon: Workflow, color: 'text-success-600', bgColor: 'bg-success-100 dark:bg-success-900/20' },
        { name: '会话请求', value: '—', icon: BarChart3, color: 'text-warning-600', bgColor: 'bg-warning-100 dark:bg-warning-900/20' },
        { name: '失败/错误信号', value: '—', icon: AlertCircle, color: 'text-error-600', bgColor: 'bg-error-100 dark:bg-error-900/20' },
      ];
    }
    return [
      {
        name: '活跃助手',
        value: String(assistantCount),
        icon: Bot,
        color: 'text-primary-600',
        bgColor: 'bg-primary-100 dark:bg-primary-900/20',
      },
      {
        name: '运行中任务',
        value: String(runningTasks),
        icon: Workflow,
        color: 'text-success-600',
        bgColor: 'bg-success-100 dark:bg-success-900/20',
      },
      {
        name: '会话请求',
        value: String(metrics.session.total_requests),
        icon: BarChart3,
        color: 'text-warning-600',
        bgColor: 'bg-warning-100 dark:bg-warning-900/20',
      },
      {
        name: '失败/错误信号',
        value: String(errorSignals),
        icon: AlertCircle,
        color: 'text-error-600',
        bgColor: 'bg-error-100 dark:bg-error-900/20',
      },
    ];
  }, [metrics, assistantCount, runningTasks, errorSignals]);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[40vh] text-surface-500 dark:text-surface-400">
        加载中…
      </div>
    );
  }

  if (error) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-error-600 dark:text-error-400 font-medium">无法加载 Dashboard</p>
          <p className="text-sm text-surface-500 mt-2">{error}</p>
          <p className="text-xs text-surface-400 mt-4">
            请确认已启动 bee-web（默认 8080），且 Vite 代理指向该端口。
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">Dashboard</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">数据来自 bee-web /api/metrics、/api/traces、/api/tasks</p>
        </div>
        <Badge variant="info">实时指标</Badge>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {stats.map((stat, index) => (
          <motion.div
            key={stat.name}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.1 }}
          >
            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className={cn('w-12 h-12 rounded-xl flex items-center justify-center', stat.bgColor)}>
                    <stat.icon className={cn('w-6 h-6', stat.color)} />
                  </div>
                  <TrendingUp className="w-4 h-4 text-surface-300" aria-hidden />
                </div>
                <div className="mt-4">
                  <p className="text-2xl font-bold text-surface-900 dark:text-surface-100">{stat.value}</p>
                  <p className="text-sm text-surface-500 dark:text-surface-400">{stat.name}</p>
                </div>
              </CardContent>
            </Card>
          </motion.div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>最近追踪</CardTitle>
            <CardDescription>来自 /api/traces/recent（全局 TraceCollector）</CardDescription>
          </CardHeader>
          <CardContent>
            {traces.length === 0 ? (
              <p className="text-sm text-surface-500 dark:text-surface-400">暂无追踪记录</p>
            ) : (
              <div className="space-y-4">
                {traces.map((t) => {
                  const ui = traceStatusUi(t.status);
                  return (
                    <div
                      key={t.request_id}
                      className="flex items-start gap-4 p-3 rounded-lg hover:bg-surface-50 dark:hover:bg-surface-800 transition-colors"
                    >
                      <div className={cn('w-2 h-2 rounded-full mt-2', ui.dot)} />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm text-surface-700 dark:text-surface-300 break-words">
                          {traceActivityMessage(t)}
                        </p>
                        <p className="text-xs text-surface-400 mt-1">
                          {t.duration_ms != null ? `${t.duration_ms} ms` : '—'} · {t.span_count} spans
                        </p>
                      </div>
                      <Badge variant={ui.variant}>{ui.label}</Badge>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>快速统计</CardTitle>
            <CardDescription>来自 /api/metrics</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {metrics && (
              <>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Clock className="w-5 h-5 text-surface-400" />
                    <span className="text-sm text-surface-600 dark:text-surface-400">LLM 平均延迟</span>
                  </div>
                  <span className="text-sm font-semibold text-surface-900 dark:text-surface-100">
                    {Math.round(metrics.llm.average_latency_ms)}ms
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <TrendingUp className="w-5 h-5 text-surface-400" />
                    <span className="text-sm text-surface-600 dark:text-surface-400">任务完成率</span>
                  </div>
                  <span className="text-sm font-semibold text-success-600">
                    {(metrics.behavior.completion_rate * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Bot className="w-5 h-5 text-surface-400" />
                    <span className="text-sm text-surface-600 dark:text-surface-400">活跃会话</span>
                  </div>
                  <span className="text-sm font-semibold text-surface-900 dark:text-surface-100">
                    {metrics.session.active_sessions}
                  </span>
                </div>
                <div className="pt-4 border-t border-surface-200 dark:border-surface-700">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm text-surface-600 dark:text-surface-400">Token（累计 prompt+completion）</span>
                    <span className="text-sm font-semibold text-surface-900 dark:text-surface-100">
                      {formatTokens(
                        metrics.llm.total_prompt_tokens + metrics.llm.total_completion_tokens
                      )}
                    </span>
                  </div>
                  <div className="w-full bg-surface-200 dark:bg-surface-700 rounded-full h-2">
                    <div
                      className="bg-primary-600 h-2 rounded-full transition-all"
                      style={{
                        width: `${Math.min(
                          100,
                          (metrics.llm.successful_calls / Math.max(1, metrics.llm.total_calls)) * 100
                        )}%`,
                      }}
                    />
                  </div>
                  <p className="text-xs text-surface-400 mt-1">LLM 成功调用占比（示意条）</p>
                </div>
              </>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
