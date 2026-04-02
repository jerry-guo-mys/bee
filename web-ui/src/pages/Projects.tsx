import { useCallback, useEffect, useState } from 'react'
import { Card, CardContent } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { createProject, getProjects } from '@/lib/api'
import { ADMIN_SCOPE_CHANGED_EVENT } from '@/lib/scope-storage'
import type { ProjectRecord } from '@/lib/api-types'

export default function ProjectsPage() {
  const [projects, setProjects] = useState<ProjectRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [workflowRunId, setWorkflowRunId] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setError(null)
      setProjects(await getProjects())
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    const onScope = () => void load()
    window.addEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope)
    return () => window.removeEventListener(ADMIN_SCOPE_CHANGED_EVENT, onScope)
  }, [load])

  async function onCreate(e: React.FormEvent) {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed || submitting) return
    setSubmitting(true)
    try {
      await createProject({
        name: trimmed,
        description: description.trim() || undefined,
        workflow_run_id: workflowRunId.trim() || undefined,
      })
      setName('')
      setDescription('')
      setWorkflowRunId('')
      await load()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardContent className="p-5">
          <h2 className="text-lg font-semibold mb-4">创建项目</h2>
          <form className="grid gap-3 md:grid-cols-3" onSubmit={onCreate}>
            <input
              className="border rounded-lg px-3 py-2 bg-white dark:bg-surface-800"
              placeholder="项目名称"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
            <input
              className="border rounded-lg px-3 py-2 bg-white dark:bg-surface-800"
              placeholder="描述（可选）"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
            <div className="flex gap-2">
              <input
                className="border rounded-lg px-3 py-2 bg-white dark:bg-surface-800 flex-1"
                placeholder="关联 workflow_run_id（可选）"
                value={workflowRunId}
                onChange={(e) => setWorkflowRunId(e.target.value)}
              />
              <Button type="submit" disabled={submitting}>
                新建
              </Button>
            </div>
          </form>
          {error ? <p className="text-sm text-error-600 mt-3">{error}</p> : null}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-5">
          <h2 className="text-lg font-semibold mb-4">项目列表</h2>
          {loading ? <p className="text-sm text-surface-500">加载中...</p> : null}
          {!loading && projects.length === 0 ? (
            <p className="text-sm text-surface-500">暂无项目</p>
          ) : null}
          <div className="space-y-2">
            {projects.map((p) => (
              <div key={p.id} className="border rounded-lg p-3">
                <div className="font-medium">{p.name}</div>
                {p.description ? <div className="text-sm text-surface-500">{p.description}</div> : null}
                <div className="text-xs text-surface-400 mt-1">
                  run: {p.workflow_run_id || '-'} | team: {p.team_id || '-'}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
