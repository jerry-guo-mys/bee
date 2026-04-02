-- M3：与 SQLite `saas_tasks` / API `Task` 对齐的工作台扩展列（云库 Postgres）
-- 注：现有 `tasks.assignee_agent_id` 保留；多承接人用 `assignee_ids_json`。

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS workflow_run_id VARCHAR(256);
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS workflow_template_id VARCHAR(256);
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS workflow_template_version INTEGER;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS assignee_ids_json TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS group_id VARCHAR(256);
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS coordinator_id TEXT;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS internal_group BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX IF NOT EXISTS idx_tasks_workflow_run_id ON tasks (workflow_run_id)
    WHERE workflow_run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_team_status ON tasks (team_id, status);
