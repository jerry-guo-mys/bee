import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
} from 'recharts';
import {
  Download,
  RefreshCw,
  TrendingUp,
  TrendingDown,
  AlertCircle,
  CheckCircle2,
  Clock,
  Activity,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { cn } from '@/lib/utils';
import { getAuditLogs, getMetrics, getTracesRecent } from '@/lib/api';
import { ADMIN_SCOPE_CHANGED_EVENT } from '@/lib/scope-storage';
import type { AuditLogRecord, MetricsResponse, TraceSummary } from '@/lib/api-types';

type MainTab = 'overview' | 'traces' | 'audit';

export default function MonitoringPage() {
  const [tab, setTab] = useState<MainTab>('overview');
  const [timeRange, setTimeRange] = useState('24h');
  const [logFilter, setLogFilter] = useState<'all' | 'error' | 'warning'>('all');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [auditLogs, setAuditLogs] = useState<AuditLogRecord[]>([]);
  const [auditError, setAuditError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      setAuditError(null);
      const [m, tr] = await Promise.all([getMetrics(), getTracesRecent(100)]);
      setMetrics(m);
      setTraces(tr.traces ?? []);
      try {
        const logs = await getAuditLogs(80);
        setAuditLogs(logs);
      } catch (e) {
        setAuditLogs([]);
        setAuditError(e instanceof Error ? e.message : String(e));
      }
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

  const pseudoTrend = useMemo(() => {
    const total = metrics?.session.total_requests ?? 0;
    const errs = metrics ? metrics.llm.failed_calls + metrics.tools.failed_executions : 0;
    return ['00:00', '06:00', '12:00', '18:00', '24:00'].map((time, i) => ({
      time,
      requests: Math.round((total * (i + 1)) / 5) || 0,
      errors: Math.round((errs * (i + 1)) / 5) || 0,
    }));
  }, [metrics]);

  const tokenPie = useMemo(() => {
    if (!metrics) return [];
    const p = metrics.llm.total_prompt_tokens;
    const c = metrics.llm.total_completion_tokens;
    if (p + c === 0) {
      return [{ name: '无数据', value: 1, color: '#a1a1aa' }];
    }
    return [
      { name: 'prompt', value: p, color: '#3b82f6' },
      { name: 'completion', value: c, color: '#8b5cf6' },
    ];
  }, [metrics]);

  const barSummary = useMemo(() => {
    if (!metrics) return [];
    return [
      { name: 'LLM 调用', value: metrics.llm.total_calls },
      { name: '工具执行', value: metrics.tools.total_executions },
      { name: '会话请求', value: metrics.session.total_requests },
    ];
  }, [metrics]);

  const failureTraces = useMemo(
    () => traces.filter((t) => t.status === 'failure' || t.status === 'cancelled'),
    [traces]
  );

  const filteredTraceLogs = useMemo(() => {
    if (logFilter === 'all') return failureTraces;
    if (logFilter === 'error') return failureTraces.filter((t) => t.status === 'failure');
    return failureTraces.filter((t) => t.status === 'cancelled');
  }, [failureTraces, logFilter]);

  if (loading && !metrics) {
    return (
      <div className="flex items-center justify-center min-h-[40vh] text-surface-500">加载中…</div>
    );
  }

  if (error) {
    return (
      <Card>
        <CardContent className="p-6">
          <p className="text-error-600 font-medium">无法加载监控数据</p>
          <p className="text-sm text-surface-500 mt-2">{error}</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">监控日志</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">/api/metrics、/api/traces、/api/audit-logs</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <a
            href="/api/metrics/prometheus"
            target="_blank"
            rel="noreferrer"
            className={cn(
              'inline-flex items-center justify-center gap-2 font-medium rounded-lg px-3 py-1.5 text-sm',
              'border-2 border-surface-300 text-surface-700 hover:bg-surface-50',
              'dark:border-surface-600 dark:text-surface-300 dark:hover:bg-surface-800'
            )}
          >
            <Download className="w-4 h-4" />
            Prometheus
          </a>
          <Button variant="outline" size="sm" type="button" onClick={() => load()} disabled={loading}>
            <RefreshCw className={cn('w-4 h-4 mr-2', loading && 'animate-spin')} />
            刷新
          </Button>
        </div>
      </div>

      <div className="flex flex-wrap gap-2">
        {(
          [
            ['overview', '总览'],
            ['traces', '追踪'],
            ['audit', '审计'],
          ] as const
        ).map(([id, label]) => (
          <Button
            key={id}
            variant={tab === id ? 'primary' : 'outline'}
            size="sm"
            type="button"
            onClick={() => setTab(id)}
          >
            {label}
          </Button>
        ))}
      </div>

      {tab === 'overview' && metrics && (
        <>
          <div className="flex items-center gap-2 flex-wrap">
            {['1h', '6h', '24h', '7d', '30d'].map((range) => (
              <Button
                key={range}
                variant={timeRange === range ? 'primary' : 'outline'}
                size="sm"
                type="button"
                onClick={() => setTimeRange(range)}
              >
                {range}
              </Button>
            ))}
            <span className="text-xs text-surface-400">时序图为占位，后端暂无按区间聚合接口</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/20 flex items-center justify-center">
                      <Activity className="w-5 h-5 text-primary-600" />
                    </div>
                    <div>
                      <p className="text-sm text-surface-500 dark:text-surface-400">会话请求</p>
                      <p className="text-2xl font-bold text-surface-900 dark:text-surface-100">
                        {metrics.session.total_requests}
                      </p>
                    </div>
                  </div>
                  <TrendingUp className="w-4 h-4 text-surface-300" />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-lg bg-success-100 dark:bg-success-900/20 flex items-center justify-center">
                      <CheckCircle2 className="w-5 h-5 text-success-600" />
                    </div>
                    <div>
                      <p className="text-sm text-surface-500 dark:text-surface-400">LLM 成功率</p>
                      <p className="text-2xl font-bold text-surface-900 dark:text-surface-100">
                        {((1 - metrics.llm.error_rate) * 100).toFixed(1)}%
                      </p>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-lg bg-warning-100 dark:bg-warning-900/20 flex items-center justify-center">
                      <Clock className="w-5 h-5 text-warning-600" />
                    </div>
                    <div>
                      <p className="text-sm text-surface-500 dark:text-surface-400">LLM 平均延迟</p>
                      <p className="text-2xl font-bold text-surface-900 dark:text-surface-100">
                        {Math.round(metrics.llm.average_latency_ms)}ms
                      </p>
                    </div>
                  </div>
                  <TrendingDown className="w-4 h-4 text-surface-300" />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="p-6">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-lg bg-error-100 dark:bg-error-900/20 flex items-center justify-center">
                      <AlertCircle className="w-5 h-5 text-error-600" />
                    </div>
                    <div>
                      <p className="text-sm text-surface-500 dark:text-surface-400">LLM 失败调用</p>
                      <p className="text-2xl font-bold text-surface-900 dark:text-surface-100">
                        {metrics.llm.failed_calls}
                      </p>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>请求趋势（占位）</CardTitle>
                <CardDescription>由当前累计值线性拆分，非真实时间序列</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={300}>
                  <AreaChart data={pseudoTrend}>
                    <defs>
                      <linearGradient id="colorRequestsMon" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" opacity={0.3} />
                    <XAxis dataKey="time" stroke="#6b7280" fontSize={12} />
                    <YAxis stroke="#6b7280" fontSize={12} />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1f2937',
                        border: '1px solid #374151',
                        borderRadius: '8px',
                      }}
                    />
                    <Area
                      type="monotone"
                      dataKey="requests"
                      stroke="#3b82f6"
                      strokeWidth={2}
                      fillOpacity={1}
                      fill="url(#colorRequestsMon)"
                    />
                    <Line type="monotone" dataKey="errors" stroke="#ef4444" strokeWidth={2} dot={false} />
                  </AreaChart>
                </ResponsiveContainer>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Token 占比</CardTitle>
                <CardDescription>prompt / completion 累计</CardDescription>
              </CardHeader>
              <CardContent>
                <ResponsiveContainer width="100%" height={200}>
                  <PieChart>
                    <Pie
                      data={tokenPie}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={5}
                      dataKey="value"
                    >
                      {tokenPie.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip />
                  </PieChart>
                </ResponsiveContainer>
                <div className="space-y-2 mt-4">
                  {tokenPie.map((row) => (
                    <div key={row.name} className="flex items-center justify-between text-sm">
                      <div className="flex items-center gap-2">
                        <div className="w-3 h-3 rounded-full" style={{ backgroundColor: row.color }} />
                        <span className="text-surface-600 dark:text-surface-400">{row.name}</span>
                      </div>
                      <span className="font-medium text-surface-900 dark:text-surface-100">{row.value}</span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>调用与执行量</CardTitle>
            </CardHeader>
            <CardContent className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={barSummary}>
                  <CartesianGrid strokeDasharray="3 3" opacity={0.3} />
                  <XAxis dataKey="name" stroke="#6b7280" fontSize={11} />
                  <YAxis stroke="#6b7280" fontSize={11} />
                  <Tooltip />
                  <Bar dataKey="value" fill="#3b82f6" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>
        </>
      )}

      {tab === 'traces' && (
        <Card>
          <CardHeader>
            <CardTitle>最近追踪</CardTitle>
            <CardDescription>/api/traces/recent</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3 max-h-[480px] overflow-y-auto">
              {traces.length === 0 ? (
                <p className="text-sm text-surface-500">暂无追踪</p>
              ) : (
                traces.map((t) => (
                  <div
                    key={t.request_id}
                    className="p-3 rounded-lg border border-surface-200 dark:border-surface-700 flex flex-col gap-1"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-mono text-xs text-surface-500">{t.request_id}</span>
                      <Badge variant={t.status === 'success' ? 'success' : t.status === 'failure' ? 'error' : 'info'}>
                        {t.status}
                      </Badge>
                    </div>
                    <p className="text-sm text-surface-700 dark:text-surface-300">
                      {t.input_summary || '—'}
                    </p>
                    <p className="text-xs text-surface-400">
                      {t.duration_ms != null ? `${t.duration_ms} ms` : '—'} · spans {t.span_count}
                    </p>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {tab === 'audit' && (
        <Card>
          <CardHeader>
            <CardTitle>审计日志</CardTitle>
            <CardDescription>/api/audit-logs（需组织管理员权限）</CardDescription>
          </CardHeader>
          <CardContent>
            {auditError && (
              <p className="text-sm text-warning-600 dark:text-warning-400 mb-4">{auditError}</p>
            )}
            <div className="space-y-3 max-h-[480px] overflow-y-auto">
              {auditLogs.length === 0 ? (
                <p className="text-sm text-surface-500">暂无审计记录或无权访问</p>
              ) : (
                auditLogs.map((log) => (
                  <div
                    key={log.id}
                    className="flex items-start gap-3 p-3 rounded-lg hover:bg-surface-50 dark:hover:bg-surface-800"
                  >
                    <div className="w-2 h-2 rounded-full mt-2 bg-success-500" />
                    <div className="flex-1 min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="text-sm font-medium text-surface-900 dark:text-surface-100">
                          {log.action}
                        </span>
                        <Badge variant="default">{log.resource_type}</Badge>
                      </div>
                      <p className="text-xs text-surface-500 mt-0.5 font-mono truncate">
                        {log.resource_id}
                      </p>
                      <p className="text-xs text-surface-400 mt-1">{log.created_at}</p>
                    </div>
                  </div>
                ))
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {tab === 'overview' && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between flex-wrap gap-2">
                <div>
                  <CardTitle>追踪异常</CardTitle>
                  <CardDescription>失败 / 取消的追踪摘要</CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant={logFilter === 'all' ? 'primary' : 'outline'}
                    size="sm"
                    type="button"
                    onClick={() => setLogFilter('all')}
                  >
                    全部
                  </Button>
                  <Button
                    variant={logFilter === 'error' ? 'danger' : 'outline'}
                    size="sm"
                    type="button"
                    onClick={() => setLogFilter('error')}
                  >
                    失败
                  </Button>
                  <Button
                    variant={logFilter === 'warning' ? 'primary' : 'outline'}
                    size="sm"
                    type="button"
                    onClick={() => setLogFilter('warning')}
                  >
                    取消
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="space-y-3 max-h-[400px] overflow-y-auto">
                {filteredTraceLogs.length === 0 ? (
                  <p className="text-sm text-surface-500">暂无失败或取消的追踪</p>
                ) : (
                  filteredTraceLogs.map((t) => (
                    <div
                      key={t.request_id}
                      className={cn(
                        'p-3 rounded-lg border',
                        t.status === 'failure'
                          ? 'bg-error-50 dark:bg-error-900/10 border-error-200 dark:border-error-800'
                          : 'bg-warning-50 dark:bg-warning-900/10 border-warning-200 dark:border-warning-800'
                      )}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex items-center gap-2">
                          <AlertCircle
                            className={cn(
                              'w-4 h-4 shrink-0',
                              t.status === 'failure' ? 'text-error-500' : 'text-warning-500'
                            )}
                          />
                          <span className="font-medium text-sm uppercase">{t.status}</span>
                        </div>
                        <span className="text-xs text-surface-400 font-mono truncate max-w-[40%]">
                          {t.request_id}
                        </span>
                      </div>
                      <p className="text-sm text-surface-700 dark:text-surface-300 mt-2">
                        {t.input_summary || '无摘要'}
                      </p>
                    </div>
                  ))
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>审计摘录</CardTitle>
              <CardDescription>同「审计」页数据源</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-3 max-h-[400px] overflow-y-auto">
                {auditLogs.slice(0, 12).map((log) => (
                  <div key={log.id} className="text-sm border-b border-surface-100 dark:border-surface-800 pb-2">
                    <span className="font-medium text-surface-900 dark:text-surface-100">{log.action}</span>
                    <span className="text-surface-400 text-xs ml-2">{log.created_at}</span>
                  </div>
                ))}
                {auditLogs.length === 0 && (
                  <p className="text-sm text-surface-500">{auditError || '无数据'}</p>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
