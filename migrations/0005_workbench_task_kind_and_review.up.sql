-- M5：任务类型、artifacts、execution、review_report（Postgres）

ALTER TABLE tasks ADD COLUMN IF NOT EXISTS task_kind VARCHAR(64);
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS artifacts_json JSONB;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS execution_json JSONB;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS review_report_json JSONB;

CREATE INDEX IF NOT EXISTS idx_tasks_task_kind ON tasks(task_kind);
